use super::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RepositoryOriginState {
    Loading,
    Available,
    Unavailable,
}

impl RepositoryOriginState {
    fn blocked_message(self) -> Option<&'static str> {
        match self {
            Self::Loading => Some(
                "Please wait for installed package-origin information before changing this repository.",
            ),
            Self::Available => None,
            Self::Unavailable => Some(
                "Recorded DNF install-origin information is unavailable. Refresh repositories before changing this repository.",
            ),
        }
    }
}

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
            let name = row.widget_name().to_ascii_lowercase();
            if name == "repository-section" {
                return query.is_empty();
            }
            query.is_empty() || name.contains(&query)
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
        Ok(mut repositories) => {
            repositories.sort_by(|left, right| {
                right.enabled.cmp(&left.enabled).then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
            });
            let enabled = repositories.iter().filter(|repo| repo.enabled).count();
            let disabled = repositories.len() - enabled;
            feedback.set_text(&format!(
                "{} COPR repositories found · {} enabled",
                repositories.len(),
                enabled
            ));

            let mut current_section = None;
            for repository in repositories {
                if current_section != Some(repository.enabled) {
                    current_section = Some(repository.enabled);
                    let section_title = if repository.enabled {
                        format!("Enabled · {enabled}")
                    } else {
                        format!("Disabled · {disabled}")
                    };
                    let section_label = gtk::Label::new(Some(&section_title));
                    section_label.set_xalign(0.0);
                    section_label.set_margin_top(16);
                    section_label.set_margin_bottom(4);
                    section_label.set_margin_start(12);
                    section_label.add_css_class("heading");
                    section_label.add_css_class("dim-label");
                    let section_row = gtk::ListBoxRow::new();
                    section_row.set_child(Some(&section_label));
                    section_row.set_selectable(false);
                    section_row.set_activatable(false);
                    section_row.set_widget_name("repository-section");
                    list.append(&section_row);
                }

                let status = if repository.enabled {
                    "Enabled"
                } else {
                    "Disabled"
                };
                let row = adw::ActionRow::builder()
                    .title(repository.name.as_str())
                    .subtitle(repository.id.as_str())
                    .build();
                row.set_widget_name(&format!(
                    "{} {} {}",
                    repository.name,
                    repository.id,
                    repository.file.display()
                ));
                row.set_tooltip_text(Some(&repository.file.display().to_string()));
                row.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));

                let status_label = gtk::Label::new(Some(status));
                status_label.add_css_class(if repository.enabled {
                    "success"
                } else {
                    "warning"
                });
                status_label.set_tooltip_text(Some("Repository enabled state"));
                row.add_suffix(&status_label);

                // Keep secondary information and less-used actions in a popover,
                // rather than squeezing every control into the repository row.
                let actions = gtk::Box::new(gtk::Orientation::Vertical, 8);
                actions.set_margin_top(12);
                actions.set_margin_bottom(12);
                actions.set_margin_start(12);
                actions.set_margin_end(12);
                actions.set_size_request(240, -1);

                let file_name = repository
                    .file
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("repository file"))
                    .to_string_lossy();
                let file_label = gtk::Label::new(Some(&format!("Repository file: {file_name}")));
                file_label.set_width_chars(32);
                file_label.set_max_width_chars(32);
                file_label.set_wrap(true);
                file_label.set_xalign(0.0);
                file_label.set_tooltip_text(Some(&repository.file.display().to_string()));
                file_label.add_css_class("dim-label");
                actions.append(&file_label);

                // Show installed package origins before a user disables or removes
                // a repository. A failed query is explicitly shown as unknown.
                let origin_state = Rc::new(Cell::new(RepositoryOriginState::Loading));
                let origin_summary = gtk::Label::new(Some("Checking DNF recorded install origins…"));
                origin_summary.set_wrap(true);
                origin_summary.set_xalign(0.0);
                origin_summary.add_css_class("dim-label");
                actions.append(&origin_summary);

                let repair_state = gtk::Button::with_label("Fix DNF permissions");
                repair_state.set_halign(gtk::Align::Fill);
                repair_state.add_css_class("flat");
                repair_state.set_visible(false);
                actions.append(&repair_state);
                {
                    let repository_id = repository.id.clone();
                    let query_summary = origin_summary.clone();
                    let repair_state = repair_state.clone();
                    let origin_state = origin_state.clone();
                    gtk::glib::MainContext::default().spawn_local(async move {
                        let result = gtk::gio::spawn_blocking(move || {
                            system::installed_packages_from_repository(&repository_id)
                        })
                        .await;
                        match result {
                            Ok(Ok(packages)) if packages.is_empty() => {
                                origin_state.set(RepositoryOriginState::Available);
                                query_summary
                                    .set_text("No RPMs report this as their recorded install origin");
                                query_summary.set_tooltip_text(Some(
                                    "This is DNF recorded install-origin data, not proof that the repository currently provides the package or would supply an upgrade.",
                                ));
                            }
                            Ok(Ok(packages)) => {
                                origin_state.set(RepositoryOriginState::Available);
                                set_installed_origin_summary(&query_summary, &packages);
                            }
                            Ok(Err(error)) => {
                                origin_state.set(RepositoryOriginState::Unavailable);
                                query_summary.set_text(&format!(
                                    "Could not read recorded DNF install origins: {error}"
                                ));
                                query_summary.set_tooltip_text(Some(&error));
                                repair_state.set_visible(
                                    system::dnf5_system_state_permission_repair_available(&error),
                                );
                            }
                            Err(_) => {
                                origin_state.set(RepositoryOriginState::Unavailable);
                                query_summary.set_text("Recorded DNF install origins unavailable");
                                query_summary
                                    .set_tooltip_text(Some("The background DNF query failed."));
                            }
                        }
                    });
                }

                repair_state.connect_clicked({
                    let repository_id = repository.id.clone();
                    let query_summary = origin_summary.clone();
                    let repair_state = repair_state.clone();
                    let origin_state = origin_state.clone();
                    move |button| {
                        origin_state.set(RepositoryOriginState::Loading);
                        button.set_sensitive(false);
                        button.set_label("Repairing…");
                        query_summary.set_text(
                            "Requesting administrator authorization to repair DNF5 permissions…",
                        );
                        let repository_id = repository_id.clone();
                        let query_summary = query_summary.clone();
                        let repair_state = repair_state.clone();
                        let origin_state = origin_state.clone();
                        gtk::glib::MainContext::default().spawn_local(async move {
                            match gtk::gio::spawn_blocking(
                                system::repair_dnf5_system_state_permissions,
                            )
                            .await
                            {
                                Ok(Ok(())) => {
                                    system::invalidate_installed_package_origins();
                                    let query_id = repository_id.clone();
                                    match gtk::gio::spawn_blocking(move || {
                                        system::installed_packages_from_repository(&query_id)
                                    })
                                    .await
                                    {
                                        Ok(Ok(packages)) => {
                                            origin_state.set(RepositoryOriginState::Available);
                                            set_installed_origin_summary(&query_summary, &packages);
                                            repair_state.set_visible(false);
                                            repair_state.set_label("Fix DNF permissions");
                                            repair_state.set_sensitive(true);
                                        }
                                        Ok(Err(error)) => {
                                            origin_state.set(RepositoryOriginState::Unavailable);
                                            query_summary.set_text(&format!(
                                                "Could not read installed package origins: {error}"
                                            ));
                                            repair_state.set_visible(
                                                system::dnf5_system_state_permission_repair_available(&error),
                                            );
                                            repair_state.set_label("Try again");
                                            repair_state.set_sensitive(true);
                                        }
                                        Err(_) => {
                                            origin_state.set(RepositoryOriginState::Unavailable);
                                            query_summary.set_text(
                                                "The background DNF query failed unexpectedly.",
                                            );
                                            repair_state.set_label("Try again");
                                            repair_state.set_sensitive(true);
                                        }
                                    }
                                }
                                Ok(Err(error)) => {
                                    origin_state.set(RepositoryOriginState::Unavailable);
                                    query_summary.set_text(&error);
                                    repair_state.set_label("Try again");
                                    repair_state.set_sensitive(true);
                                }
                                Err(_) => {
                                    origin_state.set(RepositoryOriginState::Unavailable);
                                    query_summary.set_text(
                                        "The DNF5 permission repair failed unexpectedly.",
                                    );
                                    repair_state.set_label("Try again");
                                    repair_state.set_sensitive(true);
                                }
                            }
                        });
                    }
                });

                let action = if repository.enabled {
                    "Disable"
                } else {
                    "Enable"
                };
                let toggle = gtk::Button::with_label(action);
                toggle.set_halign(gtk::Align::Fill);
                toggle.add_css_class("flat");
                toggle.connect_clicked({
                    let parent = parent.clone();
                    let repository = repository.clone();
                    let list = list.clone();
                    let feedback = feedback.clone();
                    let origin_state = origin_state.clone();
                    let action_button = toggle.clone();
                    move |_| {
                        if let Some(message) = origin_state.get().blocked_message() {
                            feedback.set_text(message);
                            return;
                        }
                        confirm_repository_toggle(
                            &parent,
                            &repository,
                            &list,
                            &feedback,
                            &action_button,
                        );
                    }
                });
                actions.append(&toggle);

                if let Some(command) =
                    system::copr_repository_command(&repository.id, !repository.enabled)
                {
                    let label = if repository.enabled {
                        "Copy disable command"
                    } else {
                        "Copy enable command"
                    };
                    let copy_command = gtk::Button::with_label(label);
                    copy_command.set_halign(gtk::Align::Fill);
                    copy_command.add_css_class("flat");
                    copy_command.set_tooltip_text(Some(&command));
                    copy_command.connect_clicked({
                        let command = command.clone();
                        let feedback = feedback.clone();
                        move |_| {
                            if let Some(display) = gtk::gdk::Display::default() {
                                display.clipboard().set_text(&command);
                                feedback.set_text("Command copied to clipboard.");
                            } else {
                                feedback.set_text("The system clipboard is unavailable.");
                            }
                        }
                    });
                    actions.append(&copy_command);
                }

                if system::copr_project_spec_from_repository_id(&repository.id).is_some() {
                    let remove = gtk::Button::with_label("Remove repository");
                    remove.set_halign(gtk::Align::Fill);
                    remove.add_css_class("destructive-action");
                    remove.connect_clicked({
                        let parent = parent.clone();
                        let repository = repository.clone();
                        let list = list.clone();
                        let feedback = feedback.clone();
                        let origin_state = origin_state.clone();
                        let action_button = remove.clone();
                        move |_| {
                            if let Some(message) = origin_state.get().blocked_message() {
                                feedback.set_text(message);
                                return;
                            }
                            confirm_repository_removal(
                                &parent,
                                &repository,
                                &list,
                                &feedback,
                                &action_button,
                            );
                        }
                    });
                    actions.append(&remove);
                }

                // Present repository details and actions in an AdwDialog,
                // which is designed to appear within the parent application window.
                let action_dialog = adw::Dialog::builder()
                    .title(format!("{} actions", repository.name))
                    .content_width(320)
                    .content_height(400)
                    .child(&actions)
                    .build();

                let menu = gtk::Button::new();
                menu.set_icon_name("view-more-symbolic");
                menu.set_tooltip_text(Some("Repository actions and details"));
                menu.add_css_class("flat");
                menu.connect_clicked({
                    let action_dialog = action_dialog.clone();
                    let parent = parent.clone();
                    move |_| action_dialog.present(Some(&parent))
                });
                row.add_suffix(&menu);
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}

fn set_installed_origin_summary(summary: &gtk::Label, packages: &[String]) {
    if packages.is_empty() {
        summary.set_text("No RPMs report this as their recorded install origin");
        summary.set_tooltip_text(Some(
            "This is DNF recorded install-origin data, not proof that the repository currently provides the package or would supply an upgrade.",
        ));
        return;
    }

    let names = packages.join(", ");
    let preview = packages.iter().take(5).cloned().collect::<Vec<_>>();
    let remaining = packages.len().saturating_sub(preview.len());
    let text = if remaining == 0 {
        format!("Recorded install origin: {}", preview.join(", "))
    } else {
        format!("Recorded install origin: {}, +{remaining} more", preview.join(", "))
    };
    summary.set_text(&text);
    summary.set_tooltip_text(Some(&format!(
        "DNF recorded these RPM install origins. This does not prove the repository currently provides them or would supply an upgrade.\n\nPackages: {names}"
    )));
}

fn confirm_repository_toggle(
    parent: &adw::ApplicationWindow,
    repository: &system::CoprRepository,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    action_button: &gtk::Button,
) {
    action_button.set_sensitive(false);
    let fedora_release = system::current_fedora_release();
    if repository.enabled {
        show_repository_toggle_dialog(parent, repository, list, feedback, action_button, None);
        return;
    }

    let Some(project_spec) = system::copr_project_spec_from_repository_id(&repository.id) else {
        show_repository_toggle_dialog(
            parent,
            repository,
            list,
            feedback,
            action_button,
            Some((fedora_release, copr::FedoraCompatibility::Unknown)),
        );
        return;
    };
    let Some((owner, project)) = project_spec.split_once('/') else {
        show_repository_toggle_dialog(
            parent,
            repository,
            list,
            feedback,
            action_button,
            Some((fedora_release, copr::FedoraCompatibility::Unknown)),
        );
        return;
    };

    if fedora_release.is_none() {
        show_repository_toggle_dialog(
            parent,
            repository,
            list,
            feedback,
            action_button,
            Some((None, copr::FedoraCompatibility::Unknown)),
        );
        return;
    }

    feedback.set_text("Checking COPR build targets for Fedora compatibility…");
    let owner = owner.to_string();
    let project = project.to_string();
    let parent = parent.clone();
    let repository = repository.clone();
    let list = list.clone();
    let feedback = feedback.clone();
    let action_button = action_button.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let result =
            gtk::gio::spawn_blocking(move || copr::fetch_project_monitor(&owner, &project)).await;
        let compatibility = match result {
            Ok(Ok(monitor)) => copr::fedora_compatibility(&monitor, fedora_release),
            Ok(Err(_)) | Err(_) => copr::FedoraCompatibility::Unknown,
        };
        show_repository_toggle_dialog(
            &parent,
            &repository,
            &list,
            &feedback,
            &action_button,
            Some((fedora_release, compatibility)),
        );
    });
}

fn show_repository_toggle_dialog(
    parent: &adw::ApplicationWindow,
    repository: &system::CoprRepository,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    action_button: &gtk::Button,
    compatibility: Option<(Option<u32>, copr::FedoraCompatibility)>,
) {
    let enabling = !repository.enabled;
    let action = if enabling { "Enable" } else { "Disable" };
    let action_past = if enabling { "enabled" } else { "disabled" };
    let (secondary_text, action_label) = if !enabling {
        (
            "This changes system-wide DNF repository configuration. You will be asked to authenticate."
                .to_string(),
            action.to_string(),
        )
    } else {
        match compatibility.unwrap_or((None, copr::FedoraCompatibility::Unknown)) {
            (Some(release), copr::FedoraCompatibility::Compatible) => (
                format!(
                    "COPR reports build targets for Fedora {release}. Compatibility is informational; package availability can still vary."
                ),
                "Enable".to_string(),
            ),
            (Some(release), copr::FedoraCompatibility::Incompatible) => (
                format!(
                    "No Fedora {release} build targets were reported. Packages may fail to install, target another release, or stop receiving updates. Choose Enable anyway to continue."
                ),
                "Enable anyway".to_string(),
            ),
            _ => (
                "Compatibility is unknown because the current Fedora release or COPR build-target data could not be determined. Review the project before enabling."
                    .to_string(),
                "Enable".to_string(),
            ),
        }
    };
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Question)
        .buttons(gtk::ButtonsType::None)
        .text(format!("{action} {}?", repository.name))
        .secondary_text(secondary_text)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button(&action_label, gtk::ResponseType::Accept);

    dialog.connect_response({
        let parent = parent.clone();
        let repository = repository.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        let action_button = action_button.clone();
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                action_button.set_sensitive(true);
                return;
            }

            feedback.set_text("Applying repository change…");
            let repository_id = repository.id.clone();
            let activity_repository_id = repository.id.clone();
            let list = list.clone();
            let feedback = feedback.clone();
            let parent = parent.clone();
            let action_button = action_button.clone();
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
                    Ok(Err(error)) => {
                        action_button.set_sensitive(true);
                        feedback.set_text(&error);
                    }
                    Err(_) => {
                        action_button.set_sensitive(true);
                        feedback.set_text("The repository operation failed unexpectedly.");
                    }
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
    action_button: &gtk::Button,
) {
    action_button.set_sensitive(false);
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
        let action_button = action_button.clone();
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                action_button.set_sensitive(true);
                return;
            }

            feedback.set_text("Removing repository…");
            let repository_id = repository.id.clone();
            let activity_repository_id = repository.id.clone();
            let list = list.clone();
            let feedback = feedback.clone();
            let parent = parent.clone();
            let action_button = action_button.clone();
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
                    Ok(Err(error)) => {
                        action_button.set_sensitive(true);
                        feedback.set_text(&error);
                    }
                    Err(_) => {
                        action_button.set_sensitive(true);
                        feedback.set_text("The repository operation failed unexpectedly.");
                    }
                }
            });
        }
    });
    dialog.present();
}

#[cfg(test)]
mod tests {
    use super::RepositoryOriginState;

    #[test]
    fn repository_changes_require_successful_package_origin_data() {
        assert!(RepositoryOriginState::Loading.blocked_message().is_some());
        assert!(RepositoryOriginState::Unavailable
            .blocked_message()
            .is_some());
        assert!(RepositoryOriginState::Available.blocked_message().is_none());
    }
}
