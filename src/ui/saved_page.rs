use super::*;
use super::project_details::show_project_details;

pub(super) fn build_saved_page(
    parent: &adw::ApplicationWindow,
    stack: &adw::ViewStack,
    app: &adw::Application,
) -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::builder()
        .label("Saved projects")
        .xalign(0.0)
        .hexpand(true)
        .build();
    title.add_css_class("title-2");
    let export = gtk::Button::with_label("Export");
    export.set_tooltip_text(Some("Save a JSON backup of your saved projects"));
    export.set_icon_name("document-save-symbolic");
    let import = gtk::Button::with_label("Import");
    import.set_tooltip_text(Some("Merge projects from a Copr Hub JSON backup"));
    import.set_icon_name("document-open-symbolic");
    let compare = gtk::Button::with_label("Compare");
    compare.set_tooltip_text(Some("Compare live build health for 2–3 saved projects"));
    compare.set_icon_name("view-grid-symbolic");
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    heading.append(&title);
    heading.append(&compare);
    heading.append(&export);
    heading.append(&import);
    heading.append(&refresh);
    content.append(&heading);

    let filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter saved projects by name, owner, or description")
        .tooltip_text("Search your locally saved projects")
        .build();
    content.append(&filter);

    let feedback = gtk::Label::builder()
        .label("Saved projects are stored locally on this user account.")
        .xalign(0.0)
        .wrap(true)
        .build();
    feedback.add_css_class("dim-label");
    content.append(&feedback);

    let monitoring_toggle =
        gtk::CheckButton::with_label("Monitor saved projects while Copr Hub is open");
    monitoring_toggle.set_tooltip_text(Some(
        "Check for newly failed builds, upstream releases, and projects with no build activity for 90 days. Checks run at most every six hours and stop when the app is closed.",
    ));
    monitoring_toggle.set_active(monitoring::load_config().enabled);
    content.append(&monitoring_toggle);

    let monitoring_note = gtk::Label::builder()
        .label("Optional · checks run every six hours while the app is open; notifications are sent only for changes after the first baseline.")
        .xalign(0.0)
        .wrap(true)
        .build();
    monitoring_note.add_css_class("dim-label");
    content.append(&monitoring_note);

    let freshness_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let freshness_label = gtk::Label::with_mnemonic("_Mark builds stale after (days)");
    freshness_label.set_xalign(0.0);
    freshness_label.set_hexpand(true);
    let freshness_days = gtk::SpinButton::with_range(7.0, 3650.0, 7.0);
    freshness_days.set_value(f64::from(
        monitoring::load_config()
            .freshness_threshold_days
            .clamp(7, 3650),
    ));
    freshness_days.set_numeric(true);
    freshness_label.set_mnemonic_widget(Some(&freshness_days));
    freshness_days.set_tooltip_text(Some(
        "Choose how long without a reported build activity marks a project as stale.",
    ));
    freshness_row.append(&freshness_label);
    freshness_row.append(&freshness_days);
    content.append(&freshness_row);
    freshness_days.connect_value_changed({
        let feedback = feedback.clone();
        let freshness_days = freshness_days.clone();
        move |_| {
            let mut config = monitoring::load_config();
            config.freshness_threshold_days = freshness_days.value_as_int().clamp(7, 3650) as u32;
            match monitoring::save_config(&config) {
                Ok(()) => feedback.set_text(&format!("Project freshness threshold set to {} days. New health observations will use this threshold.", config.freshness_threshold_days)),
                Err(error) => feedback.set_text(&format!("Could not save freshness threshold: {error}")),
            }
        }
    });

    let backup_toggle =
        gtk::CheckButton::with_label("Create scheduled local backups of saved projects");
    backup_toggle.set_tooltip_text(Some("When enabled, create a JSON backup about once per day while Copr Hub is open. Keep the latest seven backups. No background system service is installed."));
    backup_toggle.set_active(backups::load_config().enabled);
    content.append(&backup_toggle);
    let backup_note = gtk::Label::new(Some("Optional · daily while the app is open · saved under the Copr Hub data directory · retains 7 backups."));
    backup_note.set_xalign(0.0);
    backup_note.set_wrap(true);
    backup_note.add_css_class("dim-label");
    content.append(&backup_note);
    backup_toggle.connect_toggled({
        let feedback = feedback.clone();
        let app = app.clone();
        move |toggle| {
            match backups::set_enabled(toggle.is_active()) {
                Ok(()) if toggle.is_active() => {
                    feedback.set_text("Scheduled backups enabled. The first backup will be created shortly while Copr Hub is open.");
                    launch_scheduled_backup(&app);
                }
                Ok(()) => feedback.set_text("Scheduled backups disabled. Existing backup files were kept."),
                Err(error) => {
                    feedback.set_text(&format!("Could not update backup schedule: {error}"));
                    toggle.set_active(!toggle.is_active());
                }
            }
        }
    });

    monitoring_toggle.connect_toggled({
        let app = app.clone();
        let monitoring_note = monitoring_note.clone();
        move |toggle| {
            let mut config = monitoring::load_config();
            config.enabled = toggle.is_active();
            match monitoring::save_config(&config) {
                Ok(()) if config.enabled => {
                    monitoring_note.set_text("Monitoring enabled. A check will run shortly, then every six hours while Copr Hub remains open.");
                    launch_monitoring_check(&app);
                }
                Ok(()) => monitoring_note.set_text("Monitoring disabled. No background checks will run."),
                Err(error) => {
                    toggle.set_active(!config.enabled);
                    monitoring_note.set_text(&format!("Could not save monitoring settings: {error}"));
                }
            }
        }
    });

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
                Some("Export saved projects"),
                Some(&parent),
                gtk::FileChooserAction::Save,
                Some("Export"),
                Some("Cancel"),
            );
            dialog.set_current_name("copr-hub-saved-projects.json");
            dialog.connect_response({
                let feedback = feedback.clone();
                move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        match dialog.file().and_then(|file| file.path()) {
                            Some(path) => match favorites::export_to(&path) {
                                Ok(count) => feedback.set_text(&format!(
                                    "Exported {count} saved project(s) to {}.",
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
                Some("Import saved projects"),
                Some(&parent),
                gtk::FileChooserAction::Open,
                Some("Import"),
                Some("Cancel"),
            );
            dialog.connect_response({
                let parent = parent.clone();
                let list = list.clone();
                let feedback = feedback.clone();
                move |dialog, response| {
                    if response == gtk::ResponseType::Accept {
                        match dialog.file().and_then(|file| file.path()) {
                            Some(path) => match favorites::preview_import(&path) {
                                Ok(preview) => {
                                    let confirmation = gtk::MessageDialog::builder()
                                        .transient_for(&parent)
                                        .modal(true)
                                        .message_type(gtk::MessageType::Question)
                                        .buttons(gtk::ButtonsType::None)
                                        .text("Preview import changes")
                                        .secondary_text(format!(
                                            "{} project entries in backup\n{} new project(s) will be added\n{} already saved locally\n{} duplicate entry/entries within the backup will be skipped\n\nYour current saved projects will be preserved.",
                                            preview.incoming,
                                            preview.new_projects,
                                            preview.already_saved,
                                            preview.duplicate_in_backup
                                        ))
                                        .build();
                                    confirmation.add_button("Cancel", gtk::ResponseType::Cancel);
                                    confirmation.add_button("Import changes", gtk::ResponseType::Accept);
                                    confirmation.connect_response({
                                        let parent = parent.clone();
                                        let list = list.clone();
                                        let feedback = feedback.clone();
                                        let path = path.clone();
                                        move |confirmation, answer| {
                                            confirmation.close();
                                            if answer != gtk::ResponseType::Accept { return; }
                                            feedback.set_text("Importing saved projects…");
                                            let parent = parent.clone();
                                            let list = list.clone();
                                            let feedback = feedback.clone();
                                            let path = path.clone();
                                            gtk::glib::MainContext::default().spawn_local(async move {
                                                let result = gtk::gio::spawn_blocking(move || favorites::import_from(&path)).await;
                                                match result {
                                                    Ok(Ok(added)) => {
                                                        refresh_saved_projects(&parent, &list, &feedback);
                                                        feedback.set_text(&format!("Imported {added} new project(s); existing saved projects were preserved."));
                                                    }
                                                    Ok(Err(error)) => feedback.set_text(&format!("Import failed: {error}")),
                                                    Err(_) => feedback.set_text("The background import task failed unexpectedly."),
                                                }
                                            });
                                        }
                                    });
                                    confirmation.present();
                                }
                                Err(error) => feedback.set_text(&format!("Import preview failed: {error}")),
                            },
                            None => feedback.set_text("Could not access the selected backup file."),
                        }
                    }
                    dialog.destroy();
                }
            });
            dialog.show();
        }
    });
    compare.connect_clicked({
        let parent = parent.clone();
        move |_| show_compare_picker(&parent)
    });
    refresh.connect_clicked({
        let parent = parent.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        move |_| refresh_saved_projects(&parent, &list, &feedback)
    });
    refresh_saved_projects(parent, &list, &feedback);
    stack.connect_visible_child_name_notify({
        let parent = parent.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        let stack = stack.clone();
        move |_| {
            if stack.visible_child_name().as_deref() == Some("saved") {
                refresh_saved_projects(&parent, &list, &feedback);
            }
        }
    });

    scroll_page(&content)
}

fn show_compare_picker(parent: &adw::ApplicationWindow) {
    let projects = match favorites::load() {
        Ok(projects) => projects,
        Err(error) => {
            let dialog = gtk::MessageDialog::builder()
                .transient_for(parent)
                .modal(true)
                .message_type(gtk::MessageType::Error)
                .buttons(gtk::ButtonsType::Close)
                .text("Could not load saved projects")
                .secondary_text(&error)
                .build();
            dialog.connect_response(|dialog, _| dialog.close());
            dialog.present();
            return;
        }
    };

    if projects.len() < 2 {
        let dialog = gtk::MessageDialog::builder()
            .transient_for(parent)
            .modal(true)
            .message_type(gtk::MessageType::Info)
            .buttons(gtk::ButtonsType::Close)
            .text("Save at least two projects")
            .secondary_text("Project comparison needs at least two saved COPR projects. Save projects from their details windows, then try again.")
            .build();
        dialog.connect_response(|dialog, _| dialog.close());
        dialog.present();
        return;
    }

    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("Choose projects to compare")
        .default_width(620)
        .default_height(560)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = adw::StatusPage::builder()
        .icon_name("view-grid-symbolic")
        .title("Compare COPR projects")
        .description("Select two or three saved projects. Copr Hub will fetch their current build health and compare package counts, target outcomes, and build freshness.")
        .build();
    heading.set_vexpand(false);
    content.append(&heading);

    let choices_box = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let mut choices = Vec::with_capacity(projects.len());
    for project in projects {
        let description = project
            .description
            .as_deref()
            .filter(|description| !description.trim().is_empty())
            .unwrap_or("No description");
        let check = gtk::CheckButton::with_label(&format!(
            "{} (@{}) — {}",
            project.name, project.owner, description
        ));
        check.set_halign(gtk::Align::Start);
        choices_box.append(&check);
        choices.push((check, project));
    }

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&choices_box)
        .build();
    content.append(&scroll);

    let feedback = gtk::Label::new(Some("Choose 2–3 projects to compare."));
    feedback.set_xalign(0.0);
    feedback.set_wrap(true);
    feedback.add_css_class("dim-label");
    content.append(&feedback);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel = gtk::Button::with_label("Cancel");
    let compare = gtk::Button::with_label("Compare selected");
    compare.add_css_class("suggested-action");
    actions.append(&cancel);
    actions.append(&compare);
    content.append(&actions);
    window.set_content(Some(&content));

    cancel.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });

    let choices = std::rc::Rc::new(choices);
    compare.connect_clicked({
        let parent = parent.clone();
        let window = window.clone();
        let feedback = feedback.clone();
        let choices = choices.clone();
        move |_| {
            let selected: Vec<copr::CoprProject> = choices
                .iter()
                .filter(|(check, _)| check.is_active())
                .map(|(_, project)| project.clone())
                .collect();
            if !(2..=3).contains(&selected.len()) {
                feedback.set_text(
                    "Select exactly two or three projects to keep the comparison readable.",
                );
                return;
            }
            window.close();
            show_project_comparison(&parent, selected);
        }
    });

    window.present();
}

fn comparison_cell(text: &str, heading: bool) -> gtk::Label {
    let label = gtk::Label::builder()
        .label(text)
        .xalign(0.0)
        .yalign(0.5)
        .wrap(true)
        .selectable(true)
        .build();
    if heading {
        label.add_css_class("heading");
    } else {
        label.add_css_class("dim-label");
    }
    label
}

fn show_project_comparison(parent: &adw::ApplicationWindow, projects: Vec<copr::CoprProject>) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title("COPR project comparison")
        .default_width(900)
        .default_height(520)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = gtk::Label::builder()
        .label("Project comparison")
        .xalign(0.0)
        .build();
    title.add_css_class("title-1");
    content.append(&title);

    let note = gtk::Label::builder()
        .label("Build data is fetched live from COPR. Failed or unavailable lookups are shown explicitly; results are informational, not a security assessment.")
        .xalign(0.0)
        .wrap(true)
        .build();
    note.add_css_class("dim-label");
    content.append(&note);

    let grid = gtk::Grid::builder()
        .column_spacing(20)
        .row_spacing(14)
        .hexpand(true)
        .build();

    grid.attach(&comparison_cell("Metric", true), 0, 0, 1, 1);
    let metric_names = [
        "Packages",
        "Succeeded targets",
        "Failed targets",
        "In-progress targets",
        "Unknown targets",
        "Latest build",
    ];
    for (row, metric) in metric_names.iter().enumerate() {
        grid.attach(&comparison_cell(metric, true), 0, row as i32 + 1, 1, 1);
    }

    for (index, project) in projects.into_iter().enumerate() {
        let column = index as i32 + 1;
        grid.attach(
            &comparison_cell(&format!("{}/{}", project.owner, project.name), true),
            column,
            0,
            1,
            1,
        );

        let cells: Vec<gtk::Label> = (0..metric_names.len())
            .map(|_| comparison_cell("Loading…", false))
            .collect();
        for (row, cell) in cells.iter().enumerate() {
            grid.attach(cell, column, row as i32 + 1, 1, 1);
        }

        let owner = project.owner.clone();
        let name = project.name.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let result =
                gtk::gio::spawn_blocking(move || copr::fetch_project_monitor(&owner, &name)).await;

            match result {
                Ok(Ok(monitor)) => {
                    let mut succeeded = 0usize;
                    let mut failed = 0usize;
                    let mut active = 0usize;
                    let mut unknown = 0usize;
                    for build in monitor
                        .packages
                        .iter()
                        .flat_map(|package| package.chroots.values())
                    {
                        match build
                            .state
                            .as_deref()
                            .unwrap_or("unknown")
                            .to_ascii_lowercase()
                            .as_str()
                        {
                            "succeeded" | "success" | "complete" | "completed" => succeeded += 1,
                            "failed" | "error" => failed += 1,
                            "pending" | "running" | "starting" | "building" | "waiting" => {
                                active += 1
                            }
                            _ => unknown += 1,
                        }
                    }
                    let now = gtk::glib::DateTime::now_local()
                        .map(|date| date.to_unix())
                        .unwrap_or(0);
                    let freshness =
                        copr::build_freshness_message(monitor.latest_build.as_ref(), now);
                    let values = [
                        monitor.packages.len().to_string(),
                        succeeded.to_string(),
                        failed.to_string(),
                        active.to_string(),
                        unknown.to_string(),
                        freshness,
                    ];
                    for (cell, value) in cells.iter().zip(values) {
                        cell.set_text(&value);
                        cell.remove_css_class("dim-label");
                    }
                }
                Ok(Err(error)) => {
                    for cell in &cells {
                        cell.set_text(&format!("Unavailable: {error}"));
                        cell.remove_css_class("dim-label");
                    }
                }
                Err(_) => {
                    for cell in &cells {
                        cell.set_text("Unavailable: background lookup failed");
                        cell.remove_css_class("dim-label");
                    }
                }
            }
        });
    }

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&grid)
        .build();
    content.append(&scroll);

    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    close.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });
    content.append(&close);

    window.set_content(Some(&content));
    window.present();
}

fn refresh_saved_projects(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    clear_list_box(list);
    match favorites::load() {
        Ok(projects) if projects.is_empty() => {
            feedback.set_text("No saved projects yet. Save a project from its details window.");
        }
        Ok(projects) => {
            feedback.set_text(&format!(
                "{} saved project(s). Select Check health on a project to inspect its latest COPR build status.",
                projects.len()
            ));
            for project in projects {
                let row = adw::ActionRow::builder()
                    .title(project.name.as_str())
                    .subtitle(format!(
                        "@{} · {}",
                        project.owner,
                        project.description.as_deref().unwrap_or("No description")
                    ))
                    .activatable(true)
                    .build();
                row.set_widget_name(&format!(
                    "{} {} {}",
                    project.name,
                    project.owner,
                    project.description.as_deref().unwrap_or("")
                ));
                row.add_prefix(&gtk::Image::from_icon_name("starred-symbolic"));
                row.connect_activated({
                    let parent = parent.clone();
                    let project = project.clone();
                    move |_| show_project_details(&parent, &project)
                });

                let health = gtk::Button::with_label("Check health");
                health.set_tooltip_text(Some(
                    "Fetch current build status and target health from COPR",
                ));
                health.set_valign(gtk::Align::Center);
                health.add_css_class("flat");
                health.connect_clicked({
                    let row = row.clone();
                    let feedback = feedback.clone();
                    let owner = project.owner.clone();
                    let name = project.name.clone();
                    move |button| {
                        button.set_sensitive(false);
                        button.set_label("Checking…");
                        row.set_subtitle("Fetching current build health from COPR…");
                        let row = row.clone();
                        let feedback = feedback.clone();
                        let button = button.clone();
                        let owner = owner.clone();
                        let name = name.clone();
                        let activity_owner = owner.clone();
                        let activity_name = name.clone();
                        let fetch_owner = owner.clone();
                        let fetch_name = name.clone();
                        gtk::glib::MainContext::default().spawn_local(async move {
                            let result = gtk::gio::spawn_blocking(move || {
                                copr::fetch_project_monitor(&fetch_owner, &fetch_name)
                            })
                            .await;
                            button.set_sensitive(true);
                            button.set_label("Check health");
                            match result {
                                Ok(Ok(monitor)) => {
                                    let now = gtk::glib::DateTime::now_local()
                                        .map(|date| date.to_unix())
                                        .unwrap_or(0);
                                    let freshness = copr::build_freshness_message(
                                        monitor.latest_build.as_ref(),
                                        now,
                                    );
                                    let targets = copr::build_target_health_summary(
                                        &monitor.packages,
                                    );
                                    let failed = monitor
                                        .packages
                                        .iter()
                                        .flat_map(|package| package.chroots.values())
                                        .filter(|build| {
                                            matches!(
                                                build.state.as_deref().unwrap_or("unknown")
                                                    .to_ascii_lowercase().as_str(),
                                                "failed" | "error"
                                            )
                                        })
                                        .count();
                                    let summary = format!(
                                        "{} package(s) · {} · {}",
                                        monitor.packages.len(),
                                        freshness,
                                        targets
                                    );
                                    row.set_subtitle(&summary);
                                    feedback.set_text(&format!(
                                        "Health checked for {}/{}: {} failed target(s) reported. Build status is informational, not a security assessment.",
                                        name, owner, failed
                                    ));
                                    let _ = activity::record(
                                        "Project health check",
                                        &format!("{activity_owner}/{activity_name}"),
                                        &format!("{} package(s); {failed} failed target(s)", monitor.packages.len()),
                                    );
                                }
                                Ok(Err(error)) => {
                                    row.set_subtitle(&format!("Health check failed: {error}"));
                                    feedback.set_text(&format!(
                                        "Could not check {}/{}: {error}",
                                        name, owner
                                    ));
                                }
                                Err(_) => {
                                    row.set_subtitle("Health check failed: background task error.");
                                    feedback.set_text("The background health-check task failed.");
                                }
                            }
                        });
                    }
                });
                row.add_suffix(&health);
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}
