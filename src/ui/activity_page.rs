use super::*;

pub(super) fn build_activity_page(parent: &adw::ApplicationWindow, stack: &adw::ViewStack) -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::builder()
        .label("Recent activity")
        .xalign(0.0)
        .hexpand(true)
        .build();
    title.add_css_class("title-2");
    let export = gtk::Button::with_label("Export");
    export.set_icon_name("document-save-symbolic");
    export.set_tooltip_text(Some("Export activity history as JSON"));
    let import = gtk::Button::with_label("Import");
    import.set_icon_name("document-open-symbolic");
    import.set_tooltip_text(Some("Merge an activity-history JSON backup"));
    let clear = gtk::Button::with_label("Clear history");
    clear.add_css_class("destructive-action");
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    heading.append(&title);
    heading.append(&export);
    heading.append(&import);
    heading.append(&clear);
    heading.append(&refresh);
    content.append(&heading);

    let filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter activity by action, target, or summary")
        .tooltip_text("Search the locally recorded activity without changing history")
        .build();
    content.append(&filter);

    let feedback = gtk::Label::builder()
        .label("Activity history is stored locally on this user account.")
        .xalign(0.0)
        .wrap(true)
        .build();
    feedback.add_css_class("dim-label");
    content.append(&feedback);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    list.set_filter_func({
        let filter = filter.clone();
        move |row| {
            let query = filter.text().trim().to_ascii_lowercase();
            query.is_empty() || row.widget_name().to_ascii_lowercase().contains(&query)
        }
    });
    filter.connect_search_changed({
        let list = list.clone();
        move |_| list.invalidate_filter()
    });
    content.append(&list);

    export.connect_clicked({
        let parent = parent.clone();
        let feedback = feedback.clone();
        move |_| {
            let dialog = gtk::FileChooserNative::new(
                Some("Export activity history"),
                Some(&parent),
                gtk::FileChooserAction::Save,
                Some("Export"),
                Some("Cancel"),
            );
            dialog.set_current_name("copr-hub-activity.json");
            dialog.connect_response({
                let feedback = feedback.clone();
                move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        match dialog.file().and_then(|file| file.path()) {
                            Some(path) => match activity::export_to(&path) {
                                Ok(count) => feedback.set_text(&format!(
                                    "Exported {count} activity event(s) to {}.",
                                    path.display()
                                )),
                                Err(error) => feedback.set_text(&error),
                            },
                            None => feedback.set_text("Could not access the selected export path."),
                        }
                    }
                    dialog.destroy();
                }
            });
            dialog.show();
        }
    });

    import.connect_clicked({
        let parent = parent.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        move |_| {
            let dialog = gtk::FileChooserNative::new(
                Some("Import activity history"),
                Some(&parent),
                gtk::FileChooserAction::Open,
                Some("Import"),
                Some("Cancel"),
            );
            dialog.connect_response({
                let list = list.clone();
                let feedback = feedback.clone();
                move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        match dialog.file().and_then(|file| file.path()) {
                            Some(path) => match activity::import_from(&path) {
                                Ok(added) => {
                                    refresh_activity(&list, &feedback);
                                    feedback.set_text(&format!(
                                        "Imported {added} new activity event(s); existing history was preserved."
                                    ));
                                }
                                Err(error) => feedback.set_text(&error),
                            },
                            None => feedback.set_text("Could not access the selected activity backup."),
                        }
                    }
                    dialog.destroy();
                }
            });
            dialog.show();
        }
    });

    refresh.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        move |_| refresh_activity(&list, &feedback)
    });
    clear.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        let parent = parent.clone();
        move |_| {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(&parent)
                .modal(true)
                .message_type(gtk::MessageType::Warning)
                .buttons(gtk::ButtonsType::None)
                .text("Clear activity history?")
                .secondary_text("This permanently removes the local activity log. Saved projects will not be affected.")
                .build();
            dialog.add_button("Cancel", gtk::ResponseType::Cancel);
            dialog.add_button("Clear history", gtk::ResponseType::Accept);
            dialog.connect_response({
                let list = list.clone();
                let feedback = feedback.clone();
                move |dialog, response| {
                    dialog.close();
                    if response != gtk::ResponseType::Accept {
                        return;
                    }
                    match activity::clear() {
                        Ok(()) => {
                            feedback.set_text("Activity history cleared.");
                            refresh_activity(&list, &feedback);
                        }
                        Err(error) => feedback.set_text(&error),
                    }
                }
            });
            dialog.present();
        }
    });
    refresh_activity(&list, &feedback);
    stack.connect_visible_child_name_notify({
        let list = list.clone();
        let feedback = feedback.clone();
        let stack = stack.clone();
        move |_| {
            if stack.visible_child_name().as_deref() == Some("activity") {
                refresh_activity(&list, &feedback);
            }
        }
    });

    scroll_page(&content)
}

fn format_activity_timestamp(timestamp: u64) -> String {
    let Ok(timestamp) = i64::try_from(timestamp) else {
        return "Unknown time".to_string();
    };
    gtk::glib::DateTime::from_unix_local(timestamp)
        .and_then(|date_time| date_time.format("%Y-%m-%d %H:%M"))
        .map(|formatted| formatted.to_string())
        .unwrap_or_else(|_| "Unknown time".to_string())
}

fn refresh_activity(list: &gtk::ListBox, feedback: &gtk::Label) {
    clear_list_box(list);
    match activity::load() {
        Ok(entries) if entries.is_empty() => {
            feedback.set_text("No activity recorded yet. Project searches and repository changes will appear here.");
        }
        Ok(entries) => {
            feedback.set_text(&format!(
                "{} recent event(s) · stored locally",
                entries.len()
            ));
            for entry in entries {
                let row = adw::ActionRow::builder()
                    .title(format!("{} — {}", entry.action, entry.target))
                    .subtitle(format!(
                        "{} · {}",
                        format_activity_timestamp(entry.timestamp),
                        entry.summary
                    ))
                    .build();
                row.set_widget_name(&format!(
                    "{} {} {}",
                    entry.action, entry.target, entry.summary
                ));
                row.add_prefix(&gtk::Image::from_icon_name("document-properties-symbolic"));
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}
