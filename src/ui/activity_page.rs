use super::*;

pub(super) fn build_activity_page(
    parent: &adw::ApplicationWindow,
    stack: &adw::ViewStack,
) -> gtk::Widget {
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
    let category =
        gtk::DropDown::from_strings(&["All", "Projects", "Packages", "Repositories", "Other"]);
    category.set_tooltip_text(Some("Filter activity by event category"));
    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filters.append(&filter);
    filters.append(&category);
    let time_range =
        gtk::DropDown::from_strings(&["Any time", "Past 24 hours", "Past 7 days", "Past 30 days"]);
    time_range.set_tooltip_text(Some("Filter activity by age"));
    let clear_filters = gtk::Button::with_label("Clear filters");
    clear_filters.set_tooltip_text(Some("Reset text, category, and time filters"));
    filters.append(&clear_filters);
    content.append(&filters);
    content.append(&time_range);
    clear_filters.connect_clicked({
        let filter = filter.clone();
        let category = category.clone();
        let time_range = time_range.clone();
        move |_| {
            filter.set_text("");
            category.set_selected(0);
            time_range.set_selected(0);
        }
    });

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
        let category = category.clone();
        let time_range = time_range.clone();
        move |row| {
            let query = filter.text().trim().to_lowercase();
            let category_name = match category.selected() {
                1 => "Projects",
                2 => "Packages",
                3 => "Repositories",
                4 => "Other",
                _ => "All",
            }
            .to_string();
            let range = match time_range.selected() {
                1 => "Past 24 hours",
                2 => "Past 7 days",
                3 => "Past 30 days",
                _ => "Any time",
            }.to_string();
            let range = match time_range.selected() {
                1 => "Past 24 hours",
                2 => "Past 7 days",
                3 => "Past 30 days",
                _ => "Any time",
            };
            let metadata = row.widget_name().to_lowercase();
            let mut parts = metadata.splitn(3, '|');
            let row_category = parts.next().unwrap_or_default();
            let timestamp = parts
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(0);
            let searchable = parts.next().unwrap_or_default();
            let now = gtk::glib::DateTime::now_utc()
                .and_then(|date_time| date_time.to_unix())
                .unwrap_or_default()
                .max(0) as u64;
            (query.is_empty() || searchable.contains(&query))
                && (category_name == "All" || row_category == category_name.to_lowercase())
                && activity::matches_time_range(timestamp, range, now)
        }
    });
    filter.connect_search_changed({
        let list = list.clone();
        move |_| list.invalidate_filter()
    });
    category.connect_selected_notify({
        let list = list.clone();
        move |_| list.invalidate_filter()
    });
    time_range.connect_selected_notify({
        let list = list.clone();
        move |_| list.invalidate_filter()
    });
    content.append(&list);

    export.connect_clicked({
        let parent = parent.clone();
        let feedback = feedback.clone();
        let filter = filter.clone();
        let category = category.clone();
        let time_range = time_range.clone();
        move |_| {
            let query = filter.text().to_string();
            let category_name = match category.selected() {
                1 => "Projects",
                2 => "Packages",
                3 => "Repositories",
                4 => "Other",
                _ => "All",
            }.to_string();
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
                let category_name = category_name.clone();
                let range = range.clone();
                move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        match dialog.file().and_then(|file| file.path()) {
                            Some(path) => match activity::export_filtered_to(
                &path,
                &query,
                &category_name,
                &range,
            ) {
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
                    "{}|{}|{} {} {}",
                    activity::category_for_action(&entry.action),
                    entry.timestamp,
                    entry.action,
                    entry.target,
                    entry.summary
                ));
                row.add_prefix(&gtk::Image::from_icon_name("document-properties-symbolic"));
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}
