use super::*;

pub(super) fn build_repositories_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::builder()
        .label("COPR repositories")
        .xalign(0.0)
        .hexpand(true)
        .build();
    title.add_css_class("title-2");
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    heading.append(&title);
    heading.append(&refresh);
    content.append(&heading);

    let filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter repositories by name, ID, or file")
        .tooltip_text("Filter the loaded repository list locally")
        .build();
    content.append(&filter);

    let feedback = gtk::Label::builder()
        .label("Reading repository configuration…")
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

    refresh.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        let parent = parent.clone();
        move |_| refresh_repositories(&parent, &list, &feedback)
    });
    refresh_repositories(parent, &list, &feedback);

    scroll_page(&content)
}

fn refresh_repositories(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    refresh_repositories_with_notice(parent, list, feedback, None);
}

fn refresh_repositories_with_notice(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    notice: Option<String>,
) {
    system::invalidate_installed_package_origins();
    feedback.set_text("Reading COPR repository configuration…");
    clear_list_box(list);

    let parent = parent.clone();
    let list = list.clone();
    let feedback = feedback.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let response = gtk::gio::spawn_blocking(system::scan_copr_repositories).await;
        match response {
            Ok(result) => render_repositories(&parent, &list, &feedback, result),
            Err(_) => {
                clear_list_box(&list);
                feedback.set_text("The background repository scan failed unexpectedly.");
            }
        }

        if let Some(notice) = notice {
            let current_feedback = feedback.text();
            feedback.set_text(&format!("{current_feedback}\n{notice}"));
        }
    });
}

fn render_repositories(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    response: Result<Vec<system::CoprRepository>, String>,
) {
    clear_list_box(list);
    match response {
        Ok(repositories) if repositories.is_empty() => {
            feedback.set_text(
                "No COPR repositories were found in /etc/yum.repos.d. Add a COPR repository with DNF to see it here.",
            );
        }
        Ok(repositories) => {
            let enabled = repositories.iter().filter(|repo| repo.enabled).count();
            feedback.set_text(&format!(
                "{} COPR repositories found · {} enabled",
                repositories.len(),
                enabled
            ));

            for repository in repositories {
                let status = if repository.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                };
                let row = adw::ActionRow::builder()
                    .title(repository.name.as_str())
                    .subtitle(format!("{} · {}", repository.id, repository.file.display()))
                    .build();
                row.set_widget_name(&format!(
                    "{} {} {}",
                    repository.name,
                    repository.id,
                    repository.file.display()
                ));
                row.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));
                let status_label = gtk::Label::new(Some(status));
                status_label.set_tooltip_text(Some("Repository enabled state"));
                row.add_suffix(&status_label);

                // Show installed package origins before a user disables or removes
                // a repository. A failed query is explicitly shown as unknown.
                let origin_summary = gtk::Label::new(Some("Checking installed packages…"));
                origin_summary.set_wrap(true);
                origin_summary.set_xalign(0.0);
                origin_summary.add_css_class("dim-label");
                row.add_suffix(&origin_summary);
                {
                    let repository_id = repository.id.clone();
                    let query_summary = origin_summary.clone();
                    gtk::glib::MainContext::default().spawn_local(async move {
                        let result = gtk::gio::spawn_blocking(move || {
                            system::installed_packages_from_repository(&repository_id)
                        })
                        .await;
                        match result {
                            Ok(Ok(packages)) if packages.is_empty() => {
                                query_summary
                                    .set_text("No installed packages attributed to this repo");
                                query_summary.set_tooltip_text(Some(
                                    "DNF reports no installed RPMs with this source repository ID.",
                                ));
                            }
                            Ok(Ok(packages)) => {
                                let names = packages.join(", ");
                                query_summary.set_text(&format!(
                                    "{} installed package(s) from this repo",
                                    packages.len()
                                ));
                                query_summary.set_tooltip_text(Some(&names));
                            }
                            Ok(Err(error)) => {
                                query_summary.set_text("Installed package origins unavailable");
                                query_summary.set_tooltip_text(Some(&error));
                            }
                            Err(_) => {
                                query_summary.set_text("Installed package origins unavailable");
                                query_summary
                                    .set_tooltip_text(Some("The background DNF query failed."));
                            }
                        }
                    });
                }

                let action = if repository.enabled {
                    "Disable"
                } else {
                    "Enable"
                };
                let toggle = gtk::Button::with_label(action);
                toggle.set_valign(gtk::Align::Center);
                toggle.add_css_class("flat");
                toggle.connect_clicked({
                    let parent = parent.clone();
                    let repository = repository.clone();
                    let list = list.clone();
                    let feedback = feedback.clone();
                    let origin_summary = origin_summary.clone();
                    move |_| {
                        if origin_summary.text().contains("Checking installed packages") {
                            feedback.set_text("Please wait for installed package-origin information before changing this repository.");
                            return;
                        }
                        confirm_repository_toggle(&parent, &repository, &list, &feedback);
                    }
                });
                row.add_suffix(&toggle);

                if system::copr_project_spec_from_repository_id(&repository.id).is_some() {
                    let remove = gtk::Button::with_label("Remove");
                    remove.set_valign(gtk::Align::Center);
                    remove.add_css_class("destructive-action");
                    remove.connect_clicked({
                        let parent = parent.clone();
                        let repository = repository.clone();
                        let list = list.clone();
                        let feedback = feedback.clone();
                        let origin_summary = origin_summary.clone();
                        move |_| {
                            if origin_summary.text().contains("Checking installed packages") {
                                feedback.set_text("Please wait for installed package-origin information before removing this repository.");
                                return;
                            }
                            confirm_repository_removal(&parent, &repository, &list, &feedback);
                        }
                    });
                    row.add_suffix(&remove);
                }
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}

fn confirm_repository_toggle(
    parent: &adw::ApplicationWindow,
    repository: &system::CoprRepository,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    let enabling = !repository.enabled;
    let action = if enabling { "Enable" } else { "Disable" };
    let action_past = if enabling { "enabled" } else { "disabled" };
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Question)
        .buttons(gtk::ButtonsType::None)
        .text(format!("{action} {}?", repository.name))
        .secondary_text(
            "This changes system-wide DNF repository configuration. You will be asked to authenticate.",
        )
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button(action, gtk::ResponseType::Accept);

    dialog.connect_response({
        let parent = parent.clone();
        let repository = repository.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                return;
            }

            feedback.set_text("Applying repository change…");
            let repository_id = repository.id.clone();
            let activity_repository_id = repository.id.clone();
            let list = list.clone();
            let feedback = feedback.clone();
            let parent = parent.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let result = gtk::gio::spawn_blocking(move || {
                    system::set_copr_repository_enabled(&repository_id, enabling)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        let notice = activity::record(
                            "Repository configuration",
                            &activity_repository_id,
                            &format!("Repository {action_past}"),
                        )
                        .err()
                        .map(|error| format!("Activity history could not be saved: {error}"));
                        refresh_repositories_with_notice(&parent, &list, &feedback, notice);
                    }
                    Ok(Err(error)) => feedback.set_text(&error),
                    Err(_) => feedback.set_text("The repository operation failed unexpectedly."),
                }
            });
        }
    });
    dialog.present();
}

fn confirm_repository_removal(
    parent: &adw::ApplicationWindow,
    repository: &system::CoprRepository,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Warning)
        .buttons(gtk::ButtonsType::None)
        .text(format!("Remove {}?", repository.name))
        .secondary_text(
            "This removes the COPR repository configuration from this system; it does not uninstall packages already installed from it. You will be asked to authenticate.",
        )
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Remove repository", gtk::ResponseType::Accept);

    dialog.connect_response({
        let parent = parent.clone();
        let repository = repository.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                return;
            }

            feedback.set_text("Removing repository…");
            let repository_id = repository.id.clone();
            let activity_repository_id = repository.id.clone();
            let list = list.clone();
            let feedback = feedback.clone();
            let parent = parent.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let result = gtk::gio::spawn_blocking(move || {
                    system::remove_copr_repository(&repository_id)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        let notice = activity::record(
                            "Repository configuration",
                            &activity_repository_id,
                            "Repository removed",
                        )
                        .err()
                        .map(|error| format!("Activity history could not be saved: {error}"));
                        refresh_repositories_with_notice(&parent, &list, &feedback, notice);
                    }
                    Ok(Err(error)) => feedback.set_text(&error),
                    Err(_) => feedback.set_text("The repository operation failed unexpectedly."),
                }
            });
        }
    });
    dialog.present();
}
