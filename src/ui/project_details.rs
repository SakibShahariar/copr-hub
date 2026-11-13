use super::*;

use std::time::{SystemTime, UNIX_EPOCH};

fn update_last_checked(label: &gtk::Label, project: &copr::CoprProject) {
    let latest = snapshots::load(project).and_then(|snapshot| {
        [
            snapshot.monitor_fetched_at_unix,
            snapshot.history_fetched_at_unix,
            snapshot.upstream_fetched_at_unix,
        ]
        .into_iter()
        .flatten()
        .max()
    });
    label.set_text(&format!(
        "Last checked: {}",
        snapshots::cache_age_label(latest)
    ));
}

pub(super) fn is_web_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
}

fn release_notes_url(release: &upstream::UpstreamRelease) -> Option<&str> {
    if release.prerelease {
        return None;
    }
    release.url.as_deref().filter(|url| is_web_url(url))
}

fn update_release_notes_link(link: &gtk::LinkButton, release: &upstream::UpstreamRelease) {
    if let Some(url) = release_notes_url(release) {
        link.set_uri(url);
        link.set_label(if release.stale_cache {
            "Release notes (cached)"
        } else {
            "Release notes"
        });
        link.set_visible(true);
    } else {
        link.set_visible(false);
    }
}

pub(super) fn show_project_details(parent: &adw::ApplicationWindow, project: &copr::CoprProject) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(project.name.as_str())
        .default_width(760)
        .default_height(760)
        .build();
    let scroll_position = Rc::new(RefCell::new(0.0));
    populate_project_details(&window, parent, project, scroll_position, 0.0);
}

fn populate_project_details(
    window: &adw::Window,
    parent: &adw::ApplicationWindow,
    project: &copr::CoprProject,
    scroll_position: Rc<RefCell<f64>>,
    initial_scroll_position: f64,
) {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    heading.set_hexpand(true);
    let project_icon = gtk::Image::from_icon_name("package-x-generic-symbolic");
    project_icon.set_pixel_size(48);
    project_icon.set_valign(gtk::Align::Start);
    heading.append(&project_icon);

    let heading_text = gtk::Box::new(gtk::Orientation::Vertical, 6);
    heading_text.set_hexpand(true);
    let project_title = gtk::Label::builder()
        .label(project.name.as_str())
        .xalign(0.0)
        .wrap(true)
        .build();
    project_title.add_css_class("title-1");
    heading_text.append(&project_title);
    let project_description = gtk::Label::builder()
        .label(
            project
                .description
                .as_deref()
                .unwrap_or("This project does not provide a description."),
        )
        .xalign(0.0)
        .wrap(true)
        .build();
    project_description.add_css_class("dim-label");
    heading_text.append(&project_description);
    heading.append(&heading_text);
    content.append(&heading);

    let owner = adw::ActionRow::builder()
        .title("Project owner")
        .subtitle(format!("@{}", project.owner))
        .build();
    owner.add_prefix(&gtk::Image::from_icon_name("avatar-default-symbolic"));
    let details = gtk::ListBox::new();
    details.add_css_class("boxed-list");
    details.append(&owner);
    content.append(&details);

    let project_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    project_actions.set_hexpand(true);

    let saved_state = favorites::is_saved(project);
    let (saved, initial_error) = match saved_state {
        Ok(saved) => (saved, None),
        Err(error) => (false, Some(error)),
    };
    let save_button = gtk::Button::with_label(if saved {
        "Remove from saved"
    } else {
        "Save project"
    });
    save_button.set_halign(gtk::Align::Center);
    save_button.add_css_class("suggested-action");
    save_button.set_sensitive(initial_error.is_none());
    let save_feedback = gtk::Label::new(initial_error.as_deref());
    save_feedback.add_css_class("dim-label");
    let saved_value = Rc::new(RefCell::new(saved));
    save_button.connect_clicked({
        let project = project.clone();
        let saved_value = saved_value.clone();
        let save_button = save_button.clone();
        let save_feedback = save_feedback.clone();
        move |_| {
            let target_saved = !*saved_value.borrow();
            save_button.set_sensitive(false);
            save_feedback.set_text(if target_saved {
                "Saving project…"
            } else {
                "Removing project…"
            });

            let project = project.clone();
            let project_target = format!("{}/{}", project.owner, project.name);
            let saved_value = saved_value.clone();
            let save_button = save_button.clone();
            let save_feedback = save_feedback.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let result = gtk::gio::spawn_blocking(move || {
                    let saved = favorites::toggle(&project)?;
                    let activity_result = activity::record(
                        "Saved projects",
                        &project_target,
                        if saved {
                            "Project saved"
                        } else {
                            "Project removed"
                        },
                    );
                    Ok::<_, String>((saved, activity_result))
                })
                .await;

                save_button.set_sensitive(true);
                match result {
                    Ok(Ok((saved, activity_result))) => {
                        *saved_value.borrow_mut() = saved;
                        save_button.set_label(if saved {
                            "Remove from saved"
                        } else {
                            "Save project"
                        });
                        save_feedback.set_text(if saved {
                            "Project saved locally."
                        } else {
                            "Project removed from saved projects."
                        });
                        if let Err(error) = activity_result {
                            save_feedback.set_text(&format!(
                                "Project state changed, but activity history could not be updated: {error}"
                            ));
                        }
                    }
                    Ok(Err(error)) => save_feedback.set_text(&error),
                    Err(_) => save_feedback.set_text(
                        "The background save operation failed unexpectedly.",
                    ),
                }
            });
        }
    });
    project_actions.append(&save_button);

    let command_feedback = gtk::Label::new(None);
    command_feedback.add_css_class("dim-label");
    if let Some(copr_url) = copr::copr_project_url(&project.owner, &project.name) {
        let copr_link = gtk::LinkButton::with_label(&copr_url, "Open on COPR");
        copr_link.add_css_class("flat");
        copr_link.set_valign(gtk::Align::Center);
        project_actions.append(&copr_link);

        let repository_id = format!(
            "copr:copr.fedorainfracloud.org:{}:{}",
            project.owner, project.name
        );
        if let Some(command) = system::copr_repository_command(&repository_id, true) {
            let copy_command = gtk::Button::with_label("Copy enable command");
            copy_command.add_css_class("flat");
            copy_command.set_valign(gtk::Align::Center);
            copy_command.set_tooltip_text(Some("Copy the equivalent DNF command to the clipboard"));
            copy_command.connect_clicked({
                let command = command.clone();
                let command_feedback = command_feedback.clone();
                move |_| {
                    if let Some(display) = gtk::gdk::Display::default() {
                        display.clipboard().set_text(&command);
                        command_feedback.set_text("Command copied to clipboard.");
                    } else {
                        command_feedback.set_text("The system clipboard is unavailable.");
                    }
                }
            });
            project_actions.append(&copy_command);
        }
    }

    if let Some(homepage) = project.homepage.as_deref().filter(|url| is_web_url(url)) {
        let link = gtk::LinkButton::with_label(homepage, "Open project homepage");
        link.add_css_class("flat");
        link.set_valign(gtk::Align::Center);
        project_actions.append(&link);
    }

    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    refresh.set_tooltip_text(Some(
        "Refresh build targets, history, and upstream release data",
    ));
    refresh.add_css_class("flat");
    refresh.set_valign(gtk::Align::Center);
    refresh.connect_clicked({
        let parent = parent.clone();
        let project = project.clone();
        let window = window.downgrade();
        let scroll_position = scroll_position.clone();
        move |_| {
            if let Some(window) = window.upgrade() {
                let position = *scroll_position.borrow();
                window.set_content(None::<&gtk::Widget>);
                populate_project_details(
                    &window,
                    &parent,
                    &project,
                    scroll_position.clone(),
                    position,
                );
            }
        }
    });
    project_actions.append(&refresh);

    let last_checked = gtk::Label::new(Some("Last checked: waiting for the first refresh…"));
    last_checked.set_xalign(0.0);
    last_checked.add_css_class("dim-label");

    content.append(&project_actions);
    content.append(&save_feedback);
    content.append(&command_feedback);
    content.append(&last_checked);

    let section_title = gtk::Label::builder()
        .label("Packages and build targets")
        .xalign(0.0)
        .build();
    section_title.add_css_class("title-3");
    content.append(&section_title);

    let status = gtk::Label::builder()
        .label("Loading COPR build information…")
        .xalign(0.0)
        .wrap(true)
        .build();
    status.add_css_class("dim-label");
    content.append(&status);

    let compatibility_status = gtk::Label::builder()
        .label("Fedora compatibility: waiting for COPR build targets…")
        .xalign(0.0)
        .wrap(true)
        .build();
    compatibility_status.add_css_class("dim-label");
    content.append(&compatibility_status);

    let intelligence = gtk::Label::builder()
        .label("Update intelligence will appear when COPR build data and upstream release metadata are available.")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    intelligence.add_css_class("card");
    intelligence.add_css_class("caption");
    content.append(&intelligence);

    let upstream_status = gtk::Label::builder()
        .label("Checking for a latest upstream release when the project homepage points to GitHub or GitLab…")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    upstream_status.add_css_class("dim-label");
    content.append(&upstream_status);

    let release_notes = gtk::LinkButton::with_label("https://example.invalid/", "Release notes");
    release_notes.set_halign(gtk::Align::Center);
    release_notes.set_visible(false);
    content.append(&release_notes);

    let package_filter_mode = gtk::DropDown::from_strings(&[
        "All packages",
        "Upstream updates",
        "Failed builds",
        "In progress",
    ]);
    package_filter_mode.set_tooltip_text(Some("Show packages by update or build status"));
    content.append(&package_filter_mode);

    let package_filter = gtk::SearchEntry::builder()
        .placeholder_text("Search packages, versions, targets, or build states")
        .tooltip_text("Filter the loaded package list locally")
        .build();
    content.append(&package_filter);

    let packages = gtk::ListBox::new();
    packages.set_selection_mode(gtk::SelectionMode::None);
    packages.add_css_class("boxed-list");
    packages.set_filter_func({
        let package_filter = package_filter.clone();
        let package_filter_mode = package_filter_mode.clone();
        move |row| {
            package_matches_filter(
                &row.widget_name(),
                &package_filter.text(),
                package_filter_mode.selected(),
            )
        }
    });
    package_filter.connect_search_changed({
        let packages = packages.clone();
        move |_| packages.invalidate_filter()
    });
    package_filter_mode.connect_selected_notify({
        let packages = packages.clone();
        move |_| packages.invalidate_filter()
    });
    content.append(&packages);

    let matrix_title = gtk::Label::builder()
        .label("Build-target matrix")
        .xalign(0.0)
        .build();
    matrix_title.add_css_class("title-3");
    content.append(&matrix_title);

    let matrix_note = gtk::Label::builder()
        .label("Each cell shows the status COPR reports for a package and target. A dash means COPR did not report that package/target combination.")
        .xalign(0.0)
        .wrap(true)
        .build();
    matrix_note.add_css_class("dim-label");
    content.append(&matrix_note);

    let matrix = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(8)
        .hexpand(true)
        .build();
    let matrix_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .height_request(240)
        .child(&matrix)
        .build();
    matrix_scroll.add_css_class("card");
    content.append(&matrix_scroll);

    let timeline_title = gtk::Label::builder()
        .label("Recent build timeline")
        .xalign(0.0)
        .build();
    timeline_title.add_css_class("title-3");
    content.append(&timeline_title);

    let timeline_status = gtk::Label::builder()
        .label("Loading recent COPR build history…")
        .xalign(0.0)
        .wrap(true)
        .build();
    timeline_status.add_css_class("dim-label");
    content.append(&timeline_status);

    let timeline = gtk::ListBox::new();
    timeline.set_selection_mode(gtk::SelectionMode::None);
    timeline.add_css_class("boxed-list");
    content.append(&timeline);

    let clamp = adw::Clamp::builder()
        .maximum_size(760)
        .child(&content)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&clamp)
        .build();
    let adjustment = scroll.vadjustment();
    adjustment.connect_value_changed({
        let scroll_position = scroll_position.clone();
        move |adjustment| *scroll_position.borrow_mut() = adjustment.value()
    });
    scroll.connect_map({
        let adjustment = adjustment.clone();
        move |_| adjustment.set_value(initial_scroll_position)
    });
    window.set_content(Some(&scroll));
    window.present();

    let history_project_data = project.clone();
    if let Some(snapshot) = snapshots::load(project) {
        if let Some(builds) = snapshot.build_history.clone() {
            render_build_history(
                &timeline,
                &timeline_status,
                &history_project_data,
                &builds,
                snapshot.history_fetched_at_unix,
                true,
            );
        }
    }
    let history_owner = project.owner.clone();
    let history_project = project.name.clone();
    {
        let timeline = timeline.clone();
        let timeline_status = timeline_status.clone();
        let last_checked = last_checked.clone();
        let project_data = history_project_data.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let fetch_owner = history_owner.clone();
            let fetch_project = history_project.clone();
            let response = gtk::gio::spawn_blocking(move || {
                copr::fetch_project_build_history(&fetch_owner, &fetch_project)
            })
            .await;
            match response {
                Ok(Ok(builds)) => {
                    let _ = snapshots::save_history(&project_data, builds.clone());
                    update_last_checked(&last_checked, &project_data);
                    render_build_history(
                        &timeline,
                        &timeline_status,
                        &project_data,
                        &builds,
                        Some(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|v| v.as_secs())
                                .unwrap_or(0),
                        ),
                        false,
                    );
                }
                Ok(Err(error)) => {
                    if let Some(snapshot) = snapshots::load(&project_data) {
                        if let Some(builds) = snapshot.build_history {
                            render_build_history(
                                &timeline,
                                &timeline_status,
                                &project_data,
                                &builds,
                                snapshot.history_fetched_at_unix,
                                true,
                            );
                            timeline_status.set_text(&format!(
                                "{} · Live refresh failed: {error}",
                                timeline_status.text()
                            ));
                        } else {
                            timeline_status
                                .set_text(&format!("Could not load build history: {error}"));
                        }
                    } else {
                        timeline_status.set_text(&format!("Could not load build history: {error}"));
                    }
                }
                Err(_) => {
                    if let Some(snapshot) = snapshots::load(&project_data) {
                        if let Some(builds) = snapshot.build_history {
                            render_build_history(
                                &timeline,
                                &timeline_status,
                                &project_data,
                                &builds,
                                snapshot.history_fetched_at_unix,
                                true,
                            );
                            timeline_status.set_text(&format!(
                                "{} · The live background lookup failed.",
                                timeline_status.text()
                            ));
                        } else {
                            timeline_status.set_text(
                                "The background build-history lookup failed unexpectedly.",
                            );
                        }
                    } else {
                        timeline_status
                            .set_text("The background build-history lookup failed unexpectedly.");
                    }
                }
            }
        });
    }

    let homepage = project.homepage.clone();
    if homepage.is_none() {
        upstream_status.set_text(
            "Upstream release tracking unavailable: COPR does not provide a project homepage.",
        );
    }

    let homepage_for_comparison = homepage.clone();
    let monitor_state: Rc<RefCell<Option<Result<copr::ProjectMonitor, String>>>> =
        Rc::new(RefCell::new(None));
    let release_state: Rc<RefCell<Option<upstream::UpstreamRelease>>> = Rc::new(RefCell::new(None));
    let project_for_snapshot = project.clone();

    if let Some(snapshot) = snapshots::load(&project_for_snapshot) {
        update_last_checked(&last_checked, &project_for_snapshot);
        if let Some(release) = snapshot.upstream_release.clone() {
            *release_state.borrow_mut() = Some(release);
        }
        if let Some(monitor) = snapshot.monitor.clone() {
            *monitor_state.borrow_mut() = Some(Ok(monitor.clone()));
            render_project_monitor(
                &window,
                &packages,
                (&status, &compatibility_status),
                &intelligence,
                &matrix,
                Ok(monitor),
                release_state
                    .borrow()
                    .as_ref()
                    .zip(homepage_for_comparison.as_deref()),
            );
            let cache_note = snapshots::cache_age_label(snapshot.monitor_fetched_at_unix);
            let stale_note = if snapshots::cache_is_stale(snapshot.monitor_fetched_at_unix) {
                " · STALE SNAPSHOT"
            } else {
                " · cached snapshot"
            };
            status.set_text(&format!(
                "{} · Offline-capable data: {cache_note}{stale_note}",
                status.text()
            ));
        }
        if let Some(release) = snapshot.upstream_release.as_ref() {
            update_release_notes_link(&release_notes, release);
            upstream_status.set_text(&format!(
                "Cached upstream release: {} ({}) · {} · {}{}",
                release.version,
                release.name,
                release
                    .published_at
                    .as_deref()
                    .unwrap_or("release date unavailable"),
                snapshots::cache_age_label(snapshot.upstream_fetched_at_unix),
                if snapshots::cache_is_stale(snapshot.upstream_fetched_at_unix) {
                    " · STALE SNAPSHOT"
                } else {
                    " · cached"
                }
            ));
        }
    }

    let owner = project.owner.clone();
    let name = project.name.clone();
    let parent = window.clone();
    {
        let monitor_state = monitor_state.clone();
        let release_state = release_state.clone();
        let parent = parent.clone();
        let packages = packages.clone();
        let status = status.clone();
        let compatibility_status = compatibility_status.clone();
        let intelligence = intelligence.clone();
        let homepage = homepage_for_comparison.clone();
        let matrix = matrix.clone();
        let last_checked = last_checked.clone();
        let project_data = project_for_snapshot.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let fetch_owner = owner.clone();
            let fetch_name = name.clone();
            let result = gtk::gio::spawn_blocking(move || {
                copr::fetch_project_monitor(&fetch_owner, &fetch_name)
            })
            .await;
            let (response, cache_note) = match result {
                Ok(Ok(monitor)) => {
                    let _ = snapshots::save_monitor(&project_data, monitor.clone());
                    update_last_checked(&last_checked, &project_data);
                    (Ok(monitor), None)
                }
                Ok(Err(error)) => {
                    let cached = snapshots::load(&project_data).and_then(|snapshot| {
                        snapshot.monitor.map(|monitor| {
                            let note = format!(
                                "Live refresh failed: {error} · showing offline snapshot: {}{}",
                                snapshots::cache_age_label(snapshot.monitor_fetched_at_unix),
                                if snapshots::cache_is_stale(snapshot.monitor_fetched_at_unix) {
                                    " · STALE"
                                } else {
                                    ""
                                }
                            );
                            (monitor, note)
                        })
                    });
                    match cached {
                        Some((monitor, note)) => (Ok(monitor), Some(note)),
                        None => (Err(error), None),
                    }
                }
                Err(_) => {
                    let error = "The background project lookup failed unexpectedly.".to_string();
                    let cached = snapshots::load(&project_data).and_then(|snapshot| {
                        snapshot.monitor.map(|monitor| {
                            let note = format!(
                                "Live refresh failed · showing offline snapshot: {}{}",
                                snapshots::cache_age_label(snapshot.monitor_fetched_at_unix),
                                if snapshots::cache_is_stale(snapshot.monitor_fetched_at_unix) {
                                    " · STALE"
                                } else {
                                    ""
                                }
                            );
                            (monitor, note)
                        })
                    });
                    match cached {
                        Some((monitor, note)) => (Ok(monitor), Some(note)),
                        None => (Err(error), None),
                    }
                }
            };
            *monitor_state.borrow_mut() = Some(response);
            let response = monitor_state.borrow().clone();
            let release = release_state.borrow().clone();
            if let Some(response) = response {
                render_project_monitor(
                    &parent,
                    &packages,
                    (&status, &compatibility_status),
                    &intelligence,
                    &matrix,
                    response,
                    release.as_ref().zip(homepage.as_deref()),
                );
                if let Some(note) = cache_note {
                    status.set_text(&format!("{} · {note}", status.text()));
                }
            }
        });
    }

    if let Some(homepage) = homepage {
        let upstream_status = upstream_status.clone();
        let release_notes = release_notes.clone();
        let monitor_state = monitor_state.clone();
        let release_state = release_state.clone();
        let parent = parent.clone();
        let packages = packages.clone();
        let status = status.clone();
        let compatibility_status = compatibility_status.clone();
        let intelligence = intelligence.clone();
        let homepage_for_comparison = Some(homepage.clone());
        let matrix = matrix.clone();
        let last_checked = last_checked.clone();
        let project_data = project_for_snapshot.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let response =
                gtk::gio::spawn_blocking(move || upstream::latest_release(&homepage)).await;
            let release = match response {
                Ok(Ok(Some(release))) => {
                    let date = release
                        .published_at
                        .as_deref()
                        .unwrap_or("release date unavailable");
                    let url = release.url.as_deref().unwrap_or("");
                    let stale_notice = if upstream::release_is_stale(release.published_at.as_deref()) {
                        " · Release is over one year old"
                    } else {
                        ""
                    };
                    let cache_notice = if release.stale_cache {
                        " · Showing cached metadata because the upstream service is unavailable"
                    } else {
                        ""
                    };
                    if !release.stale_cache {
                        let _ = snapshots::save_release(&project_data, Some(release.clone()));
                    }
                    update_release_notes_link(&release_notes, &release);
                    upstream_status.set_text(&format!(
                        "Latest upstream stable release: {} ({}) · {}{}{}{}",
                        release.version,
                        release.name,
                        date,
                        if url.is_empty() {
                            String::new()
                        } else {
                            format!(" · {url}")
                        },
                        stale_notice,
                        cache_notice
                    ));
                    Some(release)
                }
                Ok(Ok(None)) => {
                    release_notes.set_visible(false);
                    let _ = snapshots::save_release(&project_data, None);
                    upstream_status.set_text(
                        "No stable upstream release metadata is available for this project homepage.",
                    );
                    None
                }
                Ok(Err(error)) => {
                    if let Some(snapshot) = snapshots::load(&project_data) {
                        if let Some(release) = snapshot.upstream_release {
                            update_release_notes_link(&release_notes, &release);
                            upstream_status.set_text(&format!(
                                "Could not refresh upstream releases: {error} · showing cached {} · {}{}",
                                release.version,
                                snapshots::cache_age_label(snapshot.upstream_fetched_at_unix),
                                if snapshots::cache_is_stale(snapshot.upstream_fetched_at_unix) {
                                    " · STALE SNAPSHOT"
                                } else {
                                    ""
                                }
                            ));
                            Some(release)
                        } else {
                            upstream_status.set_text(&format!("Could not check upstream releases: {error}"));
                            None
                        }
                    } else {
                        upstream_status.set_text(&format!("Could not check upstream releases: {error}"));
                        None
                    }
                }
                Err(_) => {
                    if let Some(snapshot) = snapshots::load(&project_data) {
                        if let Some(release) = snapshot.upstream_release {
                            update_release_notes_link(&release_notes, &release);
                            upstream_status.set_text(&format!(
                                "Live upstream lookup failed · showing cached {} · {}{}",
                                release.version,
                                snapshots::cache_age_label(snapshot.upstream_fetched_at_unix),
                                if snapshots::cache_is_stale(snapshot.upstream_fetched_at_unix) {
                                    " · STALE SNAPSHOT"
                                } else {
                                    ""
                                }
                            ));
                            Some(release)
                        } else {
                            upstream_status.set_text("The background upstream release lookup failed unexpectedly.");
                            None
                        }
                    } else {
                        upstream_status.set_text("The background upstream release lookup failed unexpectedly.");
                        None
                    }
                }
            };
            update_last_checked(&last_checked, &project_data);
            *release_state.borrow_mut() = release;
            let response = monitor_state.borrow().clone();
            let release = release_state.borrow().clone();
            if let Some(response) = response {
                render_project_monitor(
                    &parent,
                    &packages,
                    (&status, &compatibility_status),
                    &intelligence,
                    &matrix,
                    response,
                    release.as_ref().zip(homepage_for_comparison.as_deref()),
                );
            }
        });
    }
}

fn render_build_history(
    timeline: &gtk::ListBox,
    status: &gtk::Label,
    project: &copr::CoprProject,
    builds: &[copr::ProjectBuildEvent],
    fetched_at: Option<u64>,
    cached: bool,
) {
    clear_list_box(timeline);
    let source = if cached {
        format!(
            "Offline snapshot · {} · data may be stale",
            snapshots::cache_age_label(fetched_at)
        )
    } else {
        "Fetched live from COPR".to_string()
    };
    if builds.is_empty() {
        status.set_text(&format!("No recent build-history records. {source}"));
        return;
    }
    status.set_text(&format!(
        "Showing {} recent build record(s) · {source}. Older builds may be pruned by COPR, so this is not necessarily a complete lifetime history.",
        builds.len()
    ));
    for build in builds {
        let package_name = build
            .source_package
            .as_ref()
            .and_then(|source| source.name.as_deref())
            .unwrap_or("Unknown package");
        let version = build
            .source_package
            .as_ref()
            .and_then(|source| source.version.as_deref())
            .unwrap_or("version unavailable");
        let state = build.state.as_deref().unwrap_or("unknown");
        let timestamp = build
            .submitted_on
            .or(build.started_on)
            .or(build.ended_on)
            .and_then(format_build_timestamp);
        let date = timestamp.unwrap_or_else(|| "Timestamp unavailable".to_string());
        let target_summary = if build.chroots.is_empty() {
            "targets unavailable".to_string()
        } else {
            build.chroots.join(", ")
        };
        let row = adw::ActionRow::builder()
            .title(format!("{package_name} {version}"))
            .subtitle(format!("{state} · {date} · {target_summary}"))
            .build();
        row.add_prefix(&gtk::Image::from_icon_name(
            if matches!(
                state.to_ascii_lowercase().as_str(),
                "succeeded" | "success" | "complete" | "completed"
            ) {
                "emblem-ok-symbolic"
            } else if matches!(state.to_ascii_lowercase().as_str(), "failed" | "error") {
                "dialog-error-symbolic"
            } else {
                "content-loading-symbolic"
            },
        ));
        if let Some(build_id) = build.id {
            let url = format!(
                "https://copr.fedorainfracloud.org/coprs/{}/{}/build/{}/",
                project.owner, project.name, build_id
            );
            let link = gtk::LinkButton::with_label(&url, "Open build");
            link.set_valign(gtk::Align::Center);
            row.add_suffix(&link);
        }
        timeline.append(&row);
    }
}

pub(super) fn package_matches_filter(metadata: &str, query: &str, mode: u32) -> bool {
    let metadata = metadata.to_ascii_lowercase();
    let query = query.trim().to_ascii_lowercase();
    let matches_mode = match mode {
        1 => metadata.contains("[upstream-update]"),
        2 => metadata.contains("[failed-build]"),
        3 => metadata.contains("[active-build]"),
        _ => true,
    };
    matches_mode && (query.is_empty() || metadata.contains(&query))
}

pub(super) fn update_intelligence_summary(
    packages: &[copr::MonitoredPackage],
    homepage: Option<&str>,
    release: Option<&upstream::UpstreamRelease>,
    latest_build: Option<&copr::ProjectBuildSummary>,
    now: i64,
) -> String {
    let mut newer_packages = 0;
    let mut compared_packages = 0;
    let mut unsupported_packages = 0;
    let mut failed_targets = 0;
    let mut active_targets = 0;

    for package in packages {
        let mut has_newer_release = false;
        if let Some((release, homepage)) = release.zip(homepage) {
            let versions: Vec<_> = package
                .chroots
                .values()
                .filter_map(|build| build.pkg_version.as_deref())
                .collect();
            if !versions.is_empty() {
                let comparisons: Vec<_> = versions
                    .iter()
                    .map(|version| {
                        upstream::compare_package_version(
                            homepage,
                            &release.version,
                            &package.name,
                            version,
                        )
                    })
                    .collect();
                if comparisons.iter().any(Option::is_some) {
                    compared_packages += 1;
                    has_newer_release = comparisons.contains(&Some(std::cmp::Ordering::Greater));
                } else {
                    unsupported_packages += 1;
                }
            }
        }
        if has_newer_release {
            newer_packages += 1;
        }

        for build in package.chroots.values() {
            match build
                .state
                .as_deref()
                .unwrap_or("unknown")
                .to_ascii_lowercase()
                .as_str()
            {
                "failed" | "error" => failed_targets += 1,
                "pending" | "waiting" | "starting" | "running" | "building" => {
                    active_targets += 1;
                }
                _ => {}
            }
        }
    }

    let update_summary = match (homepage.is_some(), release) {
        (true, Some(release)) if release.prerelease => format!(
            "upstream tag {} is a prerelease; stable update comparison skipped",
            release.version
        ),
        (true, Some(release)) if release.stale_cache => format!(
            "{newer_packages} package(s) with a newer matched upstream release; {compared_packages} compared and {unsupported_packages} unsupported/unmatched (cached metadata; may be out of date)"
        ),
        (true, Some(_)) => format!(
            "{newer_packages} package(s) with a newer matched upstream release; {compared_packages} compared and {unsupported_packages} unsupported/unmatched"
        ),
        _ => "upstream update comparison unavailable".to_string(),
    };
    let build_age = copr::build_freshness_message(latest_build, now);
    let stale_release = release
        .filter(|release| upstream::release_is_stale(release.published_at.as_deref()))
        .map(|_| " · upstream release is over one year old")
        .unwrap_or("");
    let stale_build = if build_age.starts_with("No recent project build") {
        " · latest project build is at least 90 days old"
    } else {
        ""
    };

    format!(
        "Update intelligence: {update_summary} · {failed_targets} failed build target(s) · {active_targets} in-progress target(s){}{}. Signals are informational, not a security assessment.",
        stale_build, stale_release
    )
}

fn update_fedora_compatibility_status(label: &gtk::Label, monitor: &copr::ProjectMonitor) {
    match system::current_fedora_release() {
        Some(release) => match copr::fedora_compatibility(monitor, Some(release)) {
            copr::FedoraCompatibility::Compatible => label.set_text(&format!(
                "Fedora compatibility: compatible · COPR reports Fedora {release} build targets."
            )),
            copr::FedoraCompatibility::Incompatible => label.set_text(&format!(
                "Fedora compatibility: not compatible · no Fedora {release} build targets were reported."
            )),
            copr::FedoraCompatibility::Unknown => label.set_text(
                "Fedora compatibility: unknown · this project has no reported build-target data.",
            ),
        },
        None => label.set_text(
            "Fedora compatibility: unknown · the current Fedora release could not be detected.",
        ),
    }
}

fn render_project_monitor(
    parent: &adw::Window,
    list: &gtk::ListBox,
    status: (&gtk::Label, &gtk::Label),
    intelligence: &gtk::Label,
    matrix: &gtk::Grid,
    response: Result<copr::ProjectMonitor, String>,
    upstream_context: Option<(&upstream::UpstreamRelease, &str)>,
) {
    let (status, compatibility_status) = status;
    let upstream_release = upstream_context.map(|(release, _)| release);
    let homepage = upstream_context.map(|(_, homepage)| homepage);
    match &response {
        Ok(monitor) => update_fedora_compatibility_status(compatibility_status, monitor),
        Err(_) => compatibility_status
            .set_text("Fedora compatibility: unknown · COPR build-target data is unavailable."),
    }
    clear_list_box(list);
    while let Some(child) = matrix.first_child() {
        matrix.remove(&child);
    }

    match response {
        Ok(monitor) if monitor.packages.is_empty() => {
            let empty = gtk::Label::new(Some(
                "No package/target data is available to build a matrix.",
            ));
            empty.set_xalign(0.0);
            matrix.attach(&empty, 0, 0, 1, 1);
            status.set_text("COPR did not report any packages for this project.");
        }
        Ok(monitor) => {
            let target_count = monitor
                .packages
                .iter()
                .map(|package| package.chroots.len())
                .sum::<usize>();
            let freshness = copr::build_freshness_message(
                monitor.latest_build.as_ref(),
                gtk::glib::DateTime::now_local()
                    .map(|date| date.to_unix())
                    .unwrap_or(0),
            );
            render_build_target_matrix(matrix, &monitor.packages);
            let target_health = copr::build_target_health_summary(&monitor.packages);
            intelligence.set_text(&update_intelligence_summary(
                &monitor.packages,
                homepage,
                upstream_release,
                monitor.latest_build.as_ref(),
                gtk::glib::DateTime::now_local()
                    .map(|date| date.to_unix())
                    .unwrap_or(0),
            ));
            status.set_text(&format!(
                "{} packages · {} package build-target entries. {} Build health by target: {}. Build status is informational, not a security assessment.",
                monitor.packages.len(),
                target_count,
                freshness,
                target_health
            ));

            for package in monitor.packages {
                let mut targets: Vec<_> = package.chroots.iter().collect();
                targets.sort_by_key(|(left, _)| *left);
                let summary = targets
                    .iter()
                    .take(3)
                    .map(|(target, build)| {
                        let state = build.state.as_deref().unwrap_or("unknown");
                        match build.pkg_version.as_deref() {
                            Some(version) => {
                                let freshness = upstream_release.zip(homepage).and_then(
                                    |(release, homepage)| {
                                        upstream::compare_package_version(
                                            homepage,
                                            &release.version,
                                            &package.name,
                                            version,
                                        )
                                    },
                                );
                                let indicator = if freshness == Some(std::cmp::Ordering::Greater) {
                                    " · upstream release newer"
                                } else {
                                    ""
                                };
                                format!("{target}: {state} ({version}){indicator}")
                            }
                            None => format!("{target}: {state}"),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                let row = adw::ActionRow::builder()
                    .title(package.name.as_str())
                    .subtitle(if summary.is_empty() {
                        "No build-target information reported".to_string()
                    } else {
                        summary.clone()
                    })
                    .build();
                row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
                let has_newer_upstream = targets.iter().any(|(_, build)| {
                    build.pkg_version.as_deref().and_then(|version| {
                        upstream_release
                            .zip(homepage)
                            .and_then(|(release, homepage)| {
                                upstream::compare_package_version(
                                    homepage,
                                    &release.version,
                                    &package.name,
                                    version,
                                )
                            })
                    }) == Some(std::cmp::Ordering::Greater)
                });
                let has_failed_target = targets.iter().any(|(_, build)| {
                    matches!(
                        build
                            .state
                            .as_deref()
                            .unwrap_or("unknown")
                            .to_ascii_lowercase()
                            .as_str(),
                        "failed" | "error"
                    )
                });
                let has_active_target = targets.iter().any(|(_, build)| {
                    matches!(
                        build
                            .state
                            .as_deref()
                            .unwrap_or("unknown")
                            .to_ascii_lowercase()
                            .as_str(),
                        "pending" | "waiting" | "starting" | "running" | "building"
                    )
                });
                let mut searchable_metadata = format!("{} {}", package.name, summary);
                for (_, build) in &targets {
                    searchable_metadata.push(' ');
                    searchable_metadata.push_str(build.state.as_deref().unwrap_or("unknown"));
                    if let Some(version) = build.pkg_version.as_deref() {
                        searchable_metadata.push(' ');
                        searchable_metadata.push_str(version);
                    }
                }
                if has_newer_upstream {
                    searchable_metadata.push_str(" [upstream-update]");
                }
                if has_failed_target {
                    searchable_metadata.push_str(" [failed-build]");
                }
                if has_active_target {
                    searchable_metadata.push_str(" [active-build]");
                }
                row.set_widget_name(&searchable_metadata);

                if has_newer_upstream {
                    let label = gtk::Label::new(Some("Upstream newer"));
                    label.add_css_class("warning");
                    row.add_suffix(&label);
                }
                row.add_suffix(&gtk::Label::new(Some(&format!(
                    "{} targets",
                    targets.len()
                ))));

                let target_details: Vec<(String, String, Option<String>)> = targets
                    .iter()
                    .map(|(target, build)| {
                        (
                            (*target).to_string(),
                            build.state.as_deref().unwrap_or("unknown").to_string(),
                            build.pkg_version.clone(),
                        )
                    })
                    .collect();
                let details_button = gtk::Button::with_label("Build details");
                details_button.set_valign(gtk::Align::Center);
                details_button.add_css_class("flat");
                details_button.connect_clicked({
                    let parent = parent.clone();
                    let package_name = package.name.clone();
                    let target_details = target_details.clone();
                    move |_| show_build_targets(&parent, &package_name, &target_details)
                });
                row.add_suffix(&details_button);

                let install = gtk::Button::with_label("Install");
                install.set_valign(gtk::Align::Center);
                install.add_css_class("suggested-action");
                install.connect_clicked({
                    let parent = parent.clone();
                    let package_name = package.name.clone();
                    let status = status.clone();
                    let action_button = install.clone();
                    move |_| {
                        confirm_package_operation(
                            &parent,
                            package_name.clone(),
                            true,
                            status.clone(),
                            &action_button,
                            || {},
                        );
                    }
                });
                row.add_suffix(&install);
                list.append(&row);
            }
        }
        Err(error) => {
            status.set_text(&format!(
                "Could not load project build information: {error}"
            ));
            let unavailable = gtk::Label::new(Some(
                "Build-target matrix unavailable because project data could not be loaded.",
            ));
            unavailable.set_xalign(0.0);
            matrix.attach(&unavailable, 0, 0, 1, 1);
        }
    }
}

fn format_build_timestamp(timestamp: i64) -> Option<String> {
    let date = gtk::glib::DateTime::from_unix_local(timestamp).ok()?;
    date.format("%Y-%m-%d %H:%M")
        .ok()
        .map(|value| value.to_string())
}

fn render_build_target_matrix(matrix: &gtk::Grid, packages: &[copr::MonitoredPackage]) {
    while let Some(child) = matrix.first_child() {
        matrix.remove(&child);
    }

    let targets: std::collections::BTreeSet<String> = packages
        .iter()
        .flat_map(|package| package.chroots.keys().cloned())
        .collect();
    if targets.is_empty() {
        let empty = gtk::Label::new(Some("COPR did not report any build targets."));
        empty.set_xalign(0.0);
        matrix.attach(&empty, 0, 0, 1, 1);
        return;
    }

    let package_heading = gtk::Label::new(Some("Package"));
    package_heading.set_xalign(0.0);
    package_heading.add_css_class("heading");
    matrix.attach(&package_heading, 0, 0, 1, 1);

    let targets: Vec<String> = targets.into_iter().collect();
    for (column, target) in targets.iter().enumerate() {
        let heading = gtk::Label::builder()
            .label(target)
            .wrap(true)
            .max_width_chars(18)
            .build();
        heading.add_css_class("heading");
        matrix.attach(&heading, column as i32 + 1, 0, 1, 1);
    }

    for (row_index, package) in packages.iter().enumerate() {
        let row = row_index as i32 + 1;
        let package_label = gtk::Label::builder()
            .label(&package.name)
            .xalign(0.0)
            .wrap(true)
            .build();
        package_label.add_css_class("heading");
        matrix.attach(&package_label, 0, row, 1, 1);

        for (column, target) in targets.iter().enumerate() {
            let (label, class) = match package.chroots.get(target) {
                Some(build) => {
                    let state = build.state.as_deref().unwrap_or("unknown");
                    let normalized = state.to_ascii_lowercase();
                    let class = match normalized.as_str() {
                        "succeeded" | "success" | "complete" | "completed" => "success",
                        "failed" | "error" => "error",
                        "pending" | "waiting" | "starting" | "running" | "building" => "warning",
                        _ => "dim-label",
                    };
                    let version = build.pkg_version.as_deref().unwrap_or("");
                    let label = if version.is_empty() {
                        state.to_string()
                    } else {
                        format!("{state}\n{version}")
                    };
                    (label, class)
                }
                None => ("—".to_string(), "dim-label"),
            };
            let cell = gtk::Label::builder()
                .label(&label)
                .wrap(true)
                .selectable(true)
                .build();
            cell.add_css_class("card");
            cell.add_css_class(class);
            matrix.attach(&cell, column as i32 + 1, row, 1, 1);
        }
    }
}

fn show_build_targets(
    parent: &adw::Window,
    package_name: &str,
    targets: &[(String, String, Option<String>)],
) {
    let dialog = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("{package_name} build targets"))
        .default_width(520)
        .default_height(480)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);

    let heading = gtk::Label::builder()
        .label(format!("{} build target(s)", targets.len()))
        .xalign(0.0)
        .build();
    heading.add_css_class("title-2");
    content.append(&heading);

    let note = gtk::Label::builder()
        .label("Build states are reported by COPR and do not indicate whether a package is trustworthy.")
        .wrap(true)
        .xalign(0.0)
        .build();
    note.add_css_class("dim-label");
    content.append(&note);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    for (target, state, version) in targets {
        let subtitle = version
            .as_deref()
            .map(|version| format!("{state} · {version}"))
            .unwrap_or_else(|| state.clone());
        let row = adw::ActionRow::builder()
            .title(target.as_str())
            .subtitle(subtitle)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
        list.append(&row);
    }

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&list)
        .build();
    content.append(&scroll);

    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    close.connect_clicked({
        let dialog = dialog.clone();
        move |_| dialog.close()
    });
    content.append(&close);

    dialog.set_content(Some(&content));
    dialog.present();
}

pub(super) fn confirm_package_operation<F: Fn() + Clone + 'static>(
    parent: &impl IsA<gtk::Window>,
    package_name: String,
    install: bool,
    feedback: gtk::Label,
    action_button: &gtk::Button,
    on_success: F,
) {
    action_button.set_sensitive(false);
    let action = if install { "Install" } else { "Remove" };
    let dialog = gtk::MessageDialog::builder()
        .transient_for(parent)
        .modal(true)
        .message_type(gtk::MessageType::Question)
        .buttons(gtk::ButtonsType::None)
        .text(format!("{action} {package_name}?"))
        .secondary_text(
            "DNF will perform this system package operation and resolve dependencies. Administrator authentication is required. COPR is a community build service: install packages only from project owners you trust.",
        )
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button(action, gtk::ResponseType::Accept);

    dialog.connect_response({
        let package_name = package_name.clone();
        let feedback = feedback.clone();
        let action_button = action_button.clone();
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                action_button.set_sensitive(true);
                return;
            }

            feedback.set_text(&format!(
                "Waiting for DNF to {action_lower} {package_name}…",
                action_lower = action.to_lowercase()
            ));
            let package_name = package_name.clone();
            let feedback = feedback.clone();
            let action_button = action_button.clone();
            let on_success = on_success.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let operation_name = package_name.clone();
                let result = gtk::gio::spawn_blocking(move || {
                    system::set_package_installed(&package_name, install)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        action_button.set_sensitive(true);
                        let action_past = if install { "installed" } else { "removed" };
                        match activity::record(
                            "Package operation",
                            &operation_name,
                            &format!("Package {action_past} successfully"),
                        ) {
                            Ok(()) => feedback.set_text(&format!("Package {action_past} successfully.")),
                            Err(error) => feedback.set_text(&format!(
                                "Package {action_past} successfully, but activity history could not be saved: {error}"
                            )),
                        }
                        on_success();
                    }
                    Ok(Err(error)) => {
                        action_button.set_sensitive(true);
                        feedback.set_text(&error);
                    }
                    Err(_) => {
                        action_button.set_sensitive(true);
                        feedback.set_text("The background package operation failed unexpectedly.");
                    }
                }
            });
        }
    });
    dialog.present();
}

#[cfg(test)]
mod release_notes_tests {
    use super::*;

    fn release(url: Option<&str>, prerelease: bool) -> upstream::UpstreamRelease {
        upstream::UpstreamRelease {
            version: "1.2.3".into(),
            name: "Release 1.2.3".into(),
            published_at: None,
            url: url.map(str::to_string),
            stale_cache: false,
            prerelease,
        }
    }

    #[test]
    fn release_notes_require_a_stable_http_or_https_url() {
        assert_eq!(
            release_notes_url(&release(Some("https://example.com/releases/1.2.3"), false)),
            Some("https://example.com/releases/1.2.3")
        );
        assert_eq!(
            release_notes_url(&release(Some("http://example.com/releases/1.2.3"), false)),
            Some("http://example.com/releases/1.2.3")
        );
        assert_eq!(release_notes_url(&release(None, false)), None);
        assert_eq!(
            release_notes_url(&release(Some("javascript:alert(1)"), false)),
            None
        );
        assert_eq!(
            release_notes_url(&release(Some("https://example.com/releases/1.2.3"), true)),
            None
        );
    }
}
