mod activity;
mod backups;
mod copr;
mod favorites;
mod monitoring;
mod snapshots;
mod storage;
mod system;
mod upstream;

use adw::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

type ProjectSearchResponse = Result<Vec<copr::CoprProject>, String>;

const SEARCH_CACHE_MAX_BYTES: u64 = 2 * 1024 * 1024;
const SEARCH_CACHE_MAX_AGE_SECS: u64 = 30 * 24 * 60 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedSearch {
    query: String,
    saved_at_unix: u64,
    projects: Vec<copr::CoprProject>,
}

fn search_cache_path() -> PathBuf {
    gtk::glib::user_cache_dir()
        .join("copr-hub")
        .join("last-search.json")
}

fn save_search_cache(query: &str, projects: &[copr::CoprProject]) {
    if query.is_empty() || query.len() > 256 || projects.len() > 25 {
        return;
    }
    let saved_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let entry = PersistedSearch {
        query: query.to_string(),
        saved_at_unix,
        projects: projects.to_vec(),
    };
    let Ok(serialized) = serde_json::to_vec(&entry) else {
        return;
    };
    if serialized.len() as u64 > SEARCH_CACHE_MAX_BYTES {
        return;
    }
    let path = search_cache_path();
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_ok() {
        let _ = crate::storage::atomic_write(&path, &serialized);
    }
}

fn load_search_cache() -> Option<PersistedSearch> {
    let path = search_cache_path();
    let metadata = fs::metadata(&path).ok()?;
    if metadata.len() > SEARCH_CACHE_MAX_BYTES {
        return None;
    }
    let serialized = fs::read(&path).ok()?;
    let entry: PersistedSearch = serde_json::from_slice(&serialized).ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let age = now.checked_sub(entry.saved_at_unix)?;
    if age > SEARCH_CACHE_MAX_AGE_SECS
        || entry.query.trim().is_empty()
        || entry.query.len() > 256
        || entry.projects.len() > 25
    {
        return None;
    }
    Some(entry)
}

fn main() {
    let app = adw::Application::builder()
        .application_id("io.github.SakibShahariar.CoprHub")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Copr Hub")
        .default_width(1080)
        .default_height(720)
        .build();

    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    let discover = build_discover_page(&window);
    let repositories = build_repositories_page(&window);
    let installed = build_installed_page(&window);
    let saved = build_saved_page(&window, &stack, app);
    let activity = build_activity_page(&window, &stack);

    add_page(
        &stack,
        &discover,
        "discover",
        "Discover",
        "system-search-symbolic",
    );
    add_page(
        &stack,
        &repositories,
        "repositories",
        "Repositories",
        "folder-symbolic",
    );
    add_page(
        &stack,
        &installed,
        "installed",
        "Installed",
        "package-x-generic-symbolic",
    );
    add_page(&stack, &saved, "saved", "Saved", "starred-symbolic");
    add_page(
        &stack,
        &activity,
        "activity",
        "Activity",
        "document-properties-symbolic",
    );

    // Keep section navigation in the header, like GNOME Software and Bazaar.
    // Wide policy keeps the page labels visible instead of collapsing into a sidebar.
    let switcher = adw::ViewSwitcher::builder()
        .stack(&stack)
        .policy(adw::ViewSwitcherPolicy::Wide)
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&header);
    root.append(&stack);

    window.set_content(Some(&root));
    window.present();
    start_monitoring(app);
    start_scheduled_backups(app);
}

const MONITOR_INTERVAL_SECONDS: u32 = 6 * 60 * 60;

fn start_monitoring(app: &adw::Application) {
    let periodic_app = app.clone();
    gtk::glib::timeout_add_seconds_local(MONITOR_INTERVAL_SECONDS, move || {
        if monitoring::load_config().enabled {
            launch_monitoring_check(&periodic_app);
        }
        gtk::glib::ControlFlow::Continue
    });

    if monitoring::load_config().enabled {
        let startup_app = app.clone();
        gtk::glib::timeout_add_seconds_local(45, move || {
            launch_monitoring_check(&startup_app);
            gtk::glib::ControlFlow::Break
        });
    }
}

fn launch_monitoring_check(app: &adw::Application) {
    if !monitoring::load_config().enabled {
        return;
    }
    let app = app.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let result = gtk::gio::spawn_blocking(check_saved_projects).await;
        if !monitoring::load_config().enabled {
            return;
        }
        match result {
            Ok(notifications) => {
                for (id, title, body) in notifications {
                    let notification = gtk::gio::Notification::new(&title);
                    notification.set_body(Some(&body));
                    app.send_notification(Some(&id), &notification);
                }
            }
            Err(_) => {
                // Monitoring is best-effort; a failed background worker must not
                // interrupt the main UI or generate repeated failure notifications.
            }
        }
    });
}

fn start_scheduled_backups(app: &adw::Application) {
    let periodic_app = app.clone();
    gtk::glib::timeout_add_seconds_local(6 * 60 * 60, move || {
        launch_scheduled_backup(&periodic_app);
        gtk::glib::ControlFlow::Continue
    });
    let startup_app = app.clone();
    gtk::glib::timeout_add_seconds_local(60, move || {
        launch_scheduled_backup(&startup_app);
        gtk::glib::ControlFlow::Break
    });
}

fn launch_scheduled_backup(app: &adw::Application) {
    if !backups::load_config().enabled {
        return;
    }
    let app = app.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let result = gtk::gio::spawn_blocking(backups::run_if_due).await;
        match result {
            Ok(Ok(Some(path))) => {
                let notification = gtk::gio::Notification::new("Copr Hub backup created");
                notification.set_body(Some(&format!("Saved-project backup: {}", path.display())));
                app.send_notification(Some("copr-hub-scheduled-backup"), &notification);
            }
            Ok(Ok(None)) => {}
            Ok(Err(_)) | Err(_) => {
                let notification = gtk::gio::Notification::new("Copr Hub backup failed");
                notification.set_body(Some("The scheduled saved-project backup could not be created. Open Copr Hub for details."));
                app.send_notification(Some("copr-hub-scheduled-backup-error"), &notification);
            }
        }
    });
}

fn check_saved_projects() -> Vec<(String, String, String)> {
    let Ok(projects) = favorites::load() else {
        return Vec::new();
    };
    let mut notifications = Vec::new();
    for project in projects {
        let monitor = match copr::fetch_project_monitor(&project.owner, &project.name) {
            Ok(monitor) => monitor,
            Err(_) => {
                // A connection failure usually means we're offline. Stop the scan
                // instead of issuing one timeout per remaining saved project.
                break;
            }
        };
        let release = project
            .homepage
            .as_deref()
            .and_then(|homepage| upstream::latest_release(homepage).ok().flatten());
        let _ = snapshots::save_monitor(&project, monitor.clone());
        if let Some(release) = release.as_ref().filter(|release| !release.stale_cache) {
            let _ = snapshots::save_release(&project, Some(release.clone()));
        }
        if let Ok(history) = copr::fetch_project_build_history(&project.owner, &project.name) {
            let _ = snapshots::save_history(&project, history);
        }
        let alerts = monitoring::record_observation(&project, &monitor, release.as_ref())
            .unwrap_or_default();
        if !alerts.is_empty() {
            let body = alerts.join(" ");
            let id = format!("copr-hub-{}-{}", project.owner, project.name);
            let title = format!("{} · @{}", project.name, project.owner);
            notifications.push((id, title, body));
        }
    }
    notifications
}

fn add_loading_bar(parent: &gtk::Box) -> gtk::ProgressBar {
    let progress = gtk::ProgressBar::new();
    progress.set_show_text(false);
    progress.set_pulse_step(0.08);
    progress.set_visible(false);
    progress.add_css_class("osd");
    parent.append(&progress);

    gtk::glib::timeout_add_local(std::time::Duration::from_millis(90), {
        let progress = progress.clone();
        move || {
            if progress.is_visible() {
                progress.pulse();
            }
            gtk::glib::ControlFlow::Continue
        }
    });
    progress
}

fn scroll_page(child: &impl IsA<gtk::Widget>) -> gtk::Widget {
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(child)
        .build();
    scroll.upcast()
}

fn add_page(
    stack: &adw::ViewStack,
    child: &impl IsA<gtk::Widget>,
    name: &str,
    title: &str,
    icon_name: &str,
) {
    let page = stack.add_titled(child, Some(name), title);
    page.set_icon_name(Some(icon_name));
}

fn build_discover_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
    content.set_margin_top(32);
    content.set_margin_bottom(32);
    content.set_margin_start(32);
    content.set_margin_end(32);
    content.set_valign(gtk::Align::Start);
    content.set_hexpand(true);

    let hero = adw::StatusPage::builder()
        .icon_name("system-software-install-symbolic")
        .title("Find your next COPR project")
        .description(
            "Explore community-built software for Fedora. Search projects, inspect their details, and decide what to add to your system.",
        )
        .build();
    hero.set_hexpand(true);
    content.append(&hero);

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    search_row.set_hexpand(true);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search COPR project names")
        .tooltip_text("Enter a query and press Enter to search COPR")
        .hexpand(true)
        .build();
    let search_button = gtk::Button::with_label("Search");
    search_button.add_css_class("suggested-action");
    search_row.append(&search);
    search_row.append(&search_button);
    content.append(&search_row);

    let sort_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let sort_label = gtk::Label::builder()
        .label("Sort results")
        .xalign(0.0)
        .hexpand(true)
        .build();
    sort_label.add_css_class("dim-label");
    let sort = gtk::DropDown::from_strings(&["COPR order", "Project name", "Owner"]);
    sort.set_tooltip_text(Some("Sort the current search results"));
    sort_row.append(&sort_label);
    sort_row.append(&sort);
    content.append(&sort_row);

    let result_filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter results by project, owner, or description")
        .tooltip_text("Filter the current results locally without making another request")
        .build();
    content.append(&result_filter);

    let feedback = gtk::Label::builder()
        .label("Search the COPR catalogue to find community projects.")
        .wrap(true)
        .xalign(0.0)
        .build();
    feedback.add_css_class("dim-label");
    content.append(&feedback);
    let progress = add_loading_bar(&content);

    let results = gtk::ListBox::new();
    let latest_response: Rc<RefCell<Option<ProjectSearchResponse>>> = Rc::new(RefCell::new(None));
    let cached_search = load_search_cache();
    results.set_selection_mode(gtk::SelectionMode::None);
    results.add_css_class("boxed-list");
    results.set_filter_func({
        let result_filter = result_filter.clone();
        move |row| {
            let query = result_filter.text().trim().to_ascii_lowercase();
            query.is_empty() || row.widget_name().to_ascii_lowercase().contains(&query)
        }
    });
    result_filter.connect_search_changed({
        let results = results.clone();
        move |_| results.invalidate_filter()
    });
    content.append(&results);

    let perform_search = {
        let search = search.clone();
        let results = results.clone();
        let feedback = feedback.clone();
        let search_button = search_button.clone();
        let progress = progress.clone();
        let parent = parent.clone();
        let latest_response = latest_response.clone();
        let sort = sort.clone();
        move || {
            let query = search.text().trim().to_string();
            if query.is_empty() {
                clear_list_box(&results);
                feedback.set_text("Enter a COPR project name to search.");
                return;
            }

            feedback.set_text("Searching COPR…");
            progress.set_visible(true);
            search.set_sensitive(false);
            search_button.set_sensitive(false);

            let activity_query = query.clone();
            let results = results.clone();
            let feedback = feedback.clone();
            let search_button = search_button.clone();
            let search = search.clone();
            let progress = progress.clone();
            let parent = parent.clone();
            let latest_response = latest_response.clone();
            let sort = sort.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let response =
                    gtk::gio::spawn_blocking(move || copr::search_projects(&query)).await;
                search.set_sensitive(true);
                search_button.set_sensitive(true);
                progress.set_visible(false);
                let response = match response {
                    Ok(result) => result,
                    Err(_) => Err("The background search task failed unexpectedly.".to_string()),
                };
                let summary = match &response {
                    Ok(projects) => {
                        save_search_cache(&activity_query, projects);
                        format!("{} result(s)", projects.len())
                    }
                    Err(error) => format!("Failed: {error}"),
                };
                let activity_result = activity::record("Project search", &activity_query, &summary);
                if response.is_ok() {
                    *latest_response.borrow_mut() = Some(response.clone());
                }
                render_projects(&parent, &results, &feedback, response, sort.selected());
                if let Err(error) = activity_result {
                    feedback.set_text(&format!(
                        "{} Activity history could not be saved: {error}",
                        feedback.text()
                    ));
                }
            });
        }
    };

    if let Some(cached) = cached_search {
        search.set_text(&cached.query);
        let count = cached.projects.len();
        let cached_response = Ok(cached.projects);
        *latest_response.borrow_mut() = Some(cached_response.clone());
        render_projects(
            parent,
            &results,
            &feedback,
            cached_response,
            sort.selected(),
        );
        feedback.set_text(&format!(
            "Showing {count} saved result(s) for “{}”. Cached results may be out of date; search to refresh.",
            cached.query
        ));
    }

    search.connect_activate({
        let perform_search = perform_search.clone();
        move |_| perform_search()
    });
    search_button.connect_clicked(move |_| perform_search());

    sort.connect_selected_notify({
        let parent = parent.clone();
        let results = results.clone();
        let feedback = feedback.clone();
        let latest_response = latest_response.clone();
        let sort = sort.clone();
        move |_| {
            if let Some(response) = latest_response.borrow().clone() {
                render_projects(&parent, &results, &feedback, response, sort.selected());
            }
        }
    });

    let clamp = adw::Clamp::builder()
        .maximum_size(640)
        .child(&content)
        .build();
    scroll_page(&clamp)
}

fn build_installed_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::builder()
        .label("Installed COPR packages")
        .xalign(0.0)
        .hexpand(true)
        .build();
    title.add_css_class("title-2");
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    let check_updates = gtk::Button::with_label("Check updates");
    check_updates.set_tooltip_text(Some(
        "Check enabled repositories for available upgrades to installed COPR packages",
    ));
    heading.append(&title);
    heading.append(&check_updates);
    heading.append(&refresh);
    content.append(&heading);

    let filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter by package, version, or repository")
        .tooltip_text("Filter the installed COPR package list without making another DNF query")
        .build();
    content.append(&filter);

    let feedback = gtk::Label::builder()
        .label("Checking installed packages…")
        .xalign(0.0)
        .wrap(true)
        .build();
    feedback.add_css_class("dim-label");
    content.append(&feedback);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("boxed-list");
    let filter_query = Rc::new(RefCell::new(String::new()));
    list.set_filter_func({
        let filter_query = filter_query.clone();
        move |row| {
            let query = filter_query.borrow().trim().to_ascii_lowercase();
            if query.is_empty() {
                return true;
            }
            row.child()
                .is_some_and(|child| child.widget_name().to_ascii_lowercase().contains(&query))
        }
    });
    filter.connect_search_changed({
        let filter_query = filter_query.clone();
        let list = list.clone();
        let feedback = feedback.clone();
        move |entry| {
            let query = entry.text().trim().to_string();
            *filter_query.borrow_mut() = query.clone();
            list.invalidate_filter();
            if query.is_empty() {
                feedback.set_text("Showing all installed COPR packages.");
            } else {
                feedback.set_text(&format!("Filtering installed packages by “{query}”."));
            }
        }
    });
    content.append(&list);

    refresh.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        let parent = parent.clone();
        move |_| refresh_installed_packages(&parent, &list, &feedback)
    });
    check_updates.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        let parent = parent.clone();
        move |_| refresh_installed_package_updates(&parent, &list, &feedback)
    });
    refresh_installed_packages(parent, &list, &feedback);

    scroll_page(&content)
}

fn refresh_installed_packages(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    feedback.set_text("Querying DNF for installed COPR packages…");
    clear_list_box(list);

    let parent = parent.clone();
    let list = list.clone();
    let feedback = feedback.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let response = gtk::gio::spawn_blocking(system::list_installed_copr_packages).await;
        match response {
            Ok(Ok(packages)) => {
                let count = packages.len();
                render_installed_package_rows(
                    &parent,
                    &list,
                    &feedback,
                    packages,
                    std::collections::HashMap::new(),
                );
                if count == 0 {
                    feedback.set_text(
                        "No installed packages from COPR repositories were reported by DNF.",
                    );
                } else {
                    feedback.set_text(&format!(
                        "{count} installed COPR package(s). Use Check updates to compare them with enabled repositories."
                    ));
                }
            }
            Ok(Err(error)) => {
                clear_list_box(&list);
                feedback.set_text(&error);
            }
            Err(_) => {
                clear_list_box(&list);
                feedback.set_text("The background package query failed unexpectedly.");
            }
        }
    });
}

fn refresh_installed_package_updates(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
) {
    feedback.set_text("Checking enabled repositories for upgrades to installed COPR packages…");
    clear_list_box(list);

    let parent = parent.clone();
    let list = list.clone();
    let feedback = feedback.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let response = gtk::gio::spawn_blocking(|| {
            (
                system::list_installed_copr_packages(),
                system::list_package_updates(),
            )
        })
        .await;

        match response {
            Ok((Ok(packages), Ok(candidates))) => {
                let installed_names = packages
                    .iter()
                    .map(|package| package.name.as_str())
                    .collect::<std::collections::HashSet<_>>();
                let mut updates: std::collections::HashMap<
                    String,
                    Vec<system::AvailablePackageUpdate>,
                > = std::collections::HashMap::new();
                for candidate in candidates {
                    if installed_names.contains(candidate.name.as_str()) {
                        updates
                            .entry(candidate.name.clone())
                            .or_default()
                            .push(candidate);
                    }
                }
                let updated_packages = updates.len();
                let package_count = packages.len();
                render_installed_package_rows(
                    &parent,
                    &list,
                    &feedback,
                    packages,
                    updates,
                );
                feedback.set_text(&format!(
                    "Checked enabled repositories: {updated_packages} of {package_count} installed COPR package(s) have an available upgrade candidate. This is not a security audit."
                ));
            }
            Ok((Ok(packages), Err(error))) => {
                render_installed_package_rows(
                    &parent,
                    &list,
                    &feedback,
                    packages,
                    std::collections::HashMap::new(),
                );
                feedback.set_text(&format!(
                    "Could not check available updates: {error}. Installed packages are still shown."
                ));
            }
            Ok((Err(error), _)) => {
                clear_list_box(&list);
                feedback.set_text(&format!("Could not read installed COPR packages: {error}"));
            }
            Err(_) => {
                clear_list_box(&list);
                feedback.set_text("The background update query failed unexpectedly.");
            }
        }
    });
}

fn render_installed_package_rows(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    packages: Vec<system::InstalledCoprPackage>,
    updates: std::collections::HashMap<String, Vec<system::AvailablePackageUpdate>>,
) {
    clear_list_box(list);
    for package in packages {
        let update_text = updates.get(&package.name).map(|candidates| {
            let versions = candidates
                .iter()
                .take(3)
                .map(|candidate| format!("{} ({})", candidate.version, candidate.repository))
                .collect::<Vec<_>>()
                .join(", ");
            let extra = if candidates.len() > 3 {
                format!(" and {} more candidate(s)", candidates.len() - 3)
            } else {
                String::new()
            };
            format!(" · Update available: {versions}{extra}")
        });
        let subtitle = format!(
            "{} · {}{}",
            package.version,
            package.repository,
            update_text.as_deref().unwrap_or("")
        );
        let row = adw::ActionRow::builder()
            .title(package.name.as_str())
            .subtitle(subtitle)
            .build();
        row.set_widget_name(&format!(
            "{} {} {} {}",
            package.name,
            package.version,
            package.repository,
            update_text.as_deref().unwrap_or("")
        ));
        row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));

        let dependencies = gtk::Button::with_label("Dependencies");
        dependencies.set_tooltip_text(Some(
            "Inspect direct requirements and installed packages that require this package",
        ));
        dependencies.set_valign(gtk::Align::Center);
        dependencies.add_css_class("flat");
        dependencies.connect_clicked({
            let parent = parent.clone();
            let package_name = package.name.clone();
            move |_| show_package_dependency_impact(&parent, &package_name)
        });
        row.add_suffix(&dependencies);

        let remove = gtk::Button::with_label("Remove");
        remove.set_valign(gtk::Align::Center);
        remove.add_css_class("destructive-action");
        remove.connect_clicked({
            let parent = parent.clone();
            let package_name = package.name.clone();
            let list = list.clone();
            let feedback = feedback.clone();
            move |_| {
                let refresh_list = list.clone();
                let refresh_feedback = feedback.clone();
                let refresh_parent = parent.clone();
                confirm_package_operation(
                    &parent,
                    package_name.clone(),
                    false,
                    feedback.clone(),
                    move || {
                        refresh_installed_packages(
                            &refresh_parent,
                            &refresh_list,
                            &refresh_feedback,
                        )
                    },
                );
            }
        });
        row.add_suffix(&remove);
        list.append(&row);
    }
}

fn show_package_dependency_impact(parent: &adw::ApplicationWindow, package_name: &str) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(format!("Dependency impact: {package_name}"))
        .default_width(560)
        .default_height(620)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    content.set_margin_start(20);
    content.set_margin_end(20);

    let heading = gtk::Label::builder()
        .label(format!("Dependency impact for {package_name}"))
        .xalign(0.0)
        .wrap(true)
        .build();
    heading.add_css_class("title-2");
    content.append(&heading);

    let status = gtk::Label::builder()
        .label("Reading the local RPM database…")
        .xalign(0.0)
        .wrap(true)
        .build();
    status.add_css_class("dim-label");
    content.append(&status);

    let requires_heading = gtk::Label::builder()
        .label("Direct requirements")
        .xalign(0.0)
        .build();
    requires_heading.add_css_class("title-3");
    content.append(&requires_heading);
    let requirements = gtk::Label::builder()
        .label("Loading…")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    content.append(&requirements);

    let required_by_heading = gtk::Label::builder()
        .label("Installed packages that require this package")
        .xalign(0.0)
        .wrap(true)
        .build();
    required_by_heading.add_css_class("title-3");
    content.append(&required_by_heading);
    let required_by = gtk::Label::builder()
        .label("Loading…")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    content.append(&required_by);

    let note = gtk::Label::builder()
        .label("Local RPM database only; no network refresh. This is a direct dependency view, not a complete DNF transaction preview. Optional dependencies and dependencies resolved only from remote metadata may not appear.")
        .xalign(0.0)
        .wrap(true)
        .build();
    note.add_css_class("dim-label");
    content.append(&note);

    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    close.connect_clicked({
        let window = window.clone();
        move |_| window.close()
    });
    content.append(&close);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&content)
        .build();
    window.set_content(Some(&scroll));
    window.present();

    let package_name = package_name.to_string();
    gtk::glib::MainContext::default().spawn_local(async move {
        let query_name = package_name.clone();
        let result = gtk::gio::spawn_blocking(move || {
            system::inspect_package_dependency_impact(&query_name)
        })
        .await;
        match result {
            Ok(Ok(impact)) => {
                status.set_text("Dependency data loaded from the local RPM database.");
                let requirements_text = if impact.requirements.is_empty() {
                    "No direct requirements were reported.".to_string()
                } else {
                    impact.requirements.join("\n")
                };
                requirements.set_text(&requirements_text);
                let required_by_text = if impact.required_by.is_empty() {
                    "No installed package was reported as requiring this package.".to_string()
                } else {
                    impact.required_by.join("\n")
                };
                required_by.set_text(&required_by_text);
                if impact.requirements.len() == 100 || impact.required_by.len() == 100 {
                    status.set_text("Dependency data loaded; each list is capped at 100 entries.");
                }
            }
            Ok(Err(error)) => {
                status.set_text(&format!("Could not inspect dependencies: {error}"));
                requirements.set_text("Unavailable.");
                required_by.set_text("Unavailable.");
            }
            Err(_) => {
                status.set_text("The background dependency query failed unexpectedly.");
                requirements.set_text("Unavailable.");
                required_by.set_text("Unavailable.");
            }
        }
    });
}

fn build_repositories_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
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

fn clear_list_box(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

fn render_projects(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    response: Result<Vec<copr::CoprProject>, String>,
    sort_order: u32,
) {
    match response {
        Ok(projects) if projects.is_empty() => {
            clear_list_box(list);
            feedback.set_text("No projects matched that search. Try a different name or owner.");
        }
        Ok(mut projects) => {
            clear_list_box(list);
            match sort_order {
                1 => projects.sort_by(|a, b| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                        .then_with(|| {
                            a.owner
                                .to_ascii_lowercase()
                                .cmp(&b.owner.to_ascii_lowercase())
                        })
                }),
                2 => projects.sort_by(|a, b| {
                    a.owner
                        .to_ascii_lowercase()
                        .cmp(&b.owner.to_ascii_lowercase())
                        .then_with(|| {
                            a.name
                                .to_ascii_lowercase()
                                .cmp(&b.name.to_ascii_lowercase())
                        })
                }),
                _ => {}
            }
            feedback.set_text(&format!(
                "Found {} project(s). Select one for details.",
                projects.len()
            ));
            for project in projects {
                let row = adw::ActionRow::builder()
                    .title(project.name.as_str())
                    .subtitle(
                        project
                            .description
                            .as_deref()
                            .unwrap_or("No project description provided"),
                    )
                    .activatable(true)
                    .build();
                row.set_widget_name(&format!(
                    "{} {} {}",
                    project.name,
                    project.owner,
                    project.description.as_deref().unwrap_or("")
                ));
                row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
                row.add_suffix(&gtk::Label::new(Some(&format!("@{}", project.owner))));
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));

                let parent = parent.clone();
                row.connect_activated(move |_| show_project_details(&parent, &project));
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&format!("Search failed: {error}")),
    }
}

fn is_web_url(value: &str) -> bool {
    reqwest::Url::parse(value)
        .map(|url| matches!(url.scheme(), "http" | "https"))
        .unwrap_or(false)
}

fn show_project_details(parent: &adw::ApplicationWindow, project: &copr::CoprProject) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(project.name.as_str())
        .default_width(600)
        .default_height(560)
        .build();

    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let heading = adw::StatusPage::builder()
        .icon_name("package-x-generic-symbolic")
        .title(project.name.as_str())
        .description(
            project
                .description
                .as_deref()
                .unwrap_or("This project does not provide a description."),
        )
        .build();
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
    save_button.add_css_class("flat");
    save_button.set_sensitive(initial_error.is_none());
    let save_feedback = gtk::Label::new(initial_error.as_deref());
    save_feedback.add_css_class("dim-label");
    save_button.connect_clicked({
        let project = project.clone();
        let save_button = save_button.clone();
        let save_feedback = save_feedback.clone();
        move |_| match favorites::toggle(&project) {
            Ok(saved) => {
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
                if let Err(error) = activity::record(
                    "Saved projects",
                    &format!("{}/{}", project.owner, project.name),
                    if saved {
                        "Project saved"
                    } else {
                        "Project removed"
                    },
                ) {
                    save_feedback.set_text(&format!(
                        "Project state changed, but activity history could not be updated: {error}"
                    ));
                }
            }
            Err(error) => save_feedback.set_text(&error),
        }
    });
    content.append(&save_button);
    content.append(&save_feedback);

    let copr_url = format!(
        "https://copr.fedorainfracloud.org/coprs/{}/{}/",
        project.owner, project.name
    );
    let copr_link = gtk::LinkButton::with_label(&copr_url, "Open on COPR");
    copr_link.set_halign(gtk::Align::Center);
    content.append(&copr_link);

    if let Some(homepage) = project.homepage.as_deref().filter(|url| is_web_url(url)) {
        let link = gtk::LinkButton::with_label(homepage, "Open project homepage");
        link.set_halign(gtk::Align::Center);
        content.append(&link);
    }

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

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&content)
        .build();
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
        if let Some(release) = snapshot.upstream_release.clone() {
            *release_state.borrow_mut() = Some(release);
        }
        if let Some(monitor) = snapshot.monitor.clone() {
            *monitor_state.borrow_mut() = Some(Ok(monitor.clone()));
            render_project_monitor(
                &window,
                &packages,
                &status,
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
        let intelligence = intelligence.clone();
        let homepage = homepage_for_comparison.clone();
        let matrix = matrix.clone();
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
                    &status,
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
        let monitor_state = monitor_state.clone();
        let release_state = release_state.clone();
        let parent = parent.clone();
        let packages = packages.clone();
        let status = status.clone();
        let intelligence = intelligence.clone();
        let homepage_for_comparison = Some(homepage.clone());
        let matrix = matrix.clone();
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
                    let _ = snapshots::save_release(&project_data, None);
                    upstream_status.set_text(
                        "No stable upstream release metadata is available for this project homepage.",
                    );
                    None
                }
                Ok(Err(error)) => {
                    if let Some(snapshot) = snapshots::load(&project_data) {
                        if let Some(release) = snapshot.upstream_release {
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
            *release_state.borrow_mut() = release;
            let response = monitor_state.borrow().clone();
            let release = release_state.borrow().clone();
            if let Some(response) = response {
                render_project_monitor(
                    &parent,
                    &packages,
                    &status,
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

fn package_matches_filter(metadata: &str, query: &str, mode: u32) -> bool {
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

fn update_intelligence_summary(
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
                    has_newer_release = comparisons
                        .iter()
                        .any(|value| *value == Some(std::cmp::Ordering::Greater));
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

fn render_project_monitor(
    parent: &adw::Window,
    list: &gtk::ListBox,
    status: &gtk::Label,
    intelligence: &gtk::Label,
    matrix: &gtk::Grid,
    response: Result<copr::ProjectMonitor, String>,
    upstream_context: Option<(&upstream::UpstreamRelease, &str)>,
) {
    let upstream_release = upstream_context.map(|(release, _)| release);
    let homepage = upstream_context.map(|(_, homepage)| homepage);
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
                    move |_| {
                        confirm_package_operation(
                            &parent,
                            package_name.clone(),
                            true,
                            status.clone(),
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

fn confirm_package_operation<F: Fn() + Clone + 'static>(
    parent: &impl IsA<gtk::Window>,
    package_name: String,
    install: bool,
    feedback: gtk::Label,
    on_success: F,
) {
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
        move |dialog, response| {
            dialog.close();
            if response != gtk::ResponseType::Accept {
                return;
            }

            feedback.set_text(&format!(
                "Waiting for DNF to {action_lower} {package_name}…",
                action_lower = action.to_lowercase()
            ));
            let package_name = package_name.clone();
            let feedback = feedback.clone();
            let on_success = on_success.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let operation_name = package_name.clone();
                let result = gtk::gio::spawn_blocking(move || {
                    system::set_package_installed(&package_name, install)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
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
                    Ok(Err(error)) => feedback.set_text(&error),
                    Err(_) => {
                        feedback.set_text("The background package operation failed unexpectedly.")
                    }
                }
            });
        }
    });
    dialog.present();
}

fn build_saved_page(
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
                                            "{} project entries in backup\n{} new project(s) will be added\n{} duplicate/already-saved entry(s) will be skipped\n\nYour current saved projects will be preserved.",
                                            preview.incoming, preview.new_projects, preview.already_saved
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

fn build_activity_page(parent: &adw::ApplicationWindow, stack: &adw::ViewStack) -> gtk::Widget {
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

#[cfg(test)]
mod url_tests {
    use super::{is_web_url, package_matches_filter, update_intelligence_summary};
    use crate::{copr, upstream};

    #[test]
    fn update_intelligence_counts_newer_packages_and_build_failures() {
        let packages: Vec<copr::MonitoredPackage> = serde_json::from_str(
            r#"[{"name":"tool","chroots":{
                "fedora-44-x86_64":{"state":"failed","pkg_version":"1.2.0-1.fc44"},
                "fedora-43-x86_64":{"state":"running","pkg_version":"1.2.0-1.fc43"}
            }}]"#,
        )
        .unwrap();
        let release = upstream::UpstreamRelease {
            version: "1.3.0".into(),
            name: "Tool 1.3.0".into(),
            published_at: Some("2026-09-01T00:00:00Z".into()),
            url: None,
            stale_cache: false,
            prerelease: false,
        };
        let summary = update_intelligence_summary(
            &packages,
            Some("https://github.com/demo/tool"),
            Some(&release),
            None,
            1_800_000_000,
        );

        assert!(summary.contains("1 package(s) with a newer matched upstream release"));
        assert!(summary.contains("1 failed build target(s)"));
        assert!(summary.contains("1 in-progress target(s)"));
    }

    #[test]
    fn update_intelligence_warns_when_release_comparison_uses_cached_metadata() {
        let release = upstream::UpstreamRelease {
            version: "1.3.0".into(),
            name: "Tool 1.3.0".into(),
            published_at: Some("2026-09-01T00:00:00Z".into()),
            url: None,
            stale_cache: true,
            prerelease: false,
        };
        let summary = update_intelligence_summary(
            &[],
            Some("https://github.com/demo/tool"),
            Some(&release),
            None,
            1_800_000_000,
        );

        assert!(summary.contains("cached metadata; may be out of date"));
    }

    #[test]
    fn update_intelligence_does_not_guess_when_release_data_is_missing() {
        let summary = update_intelligence_summary(&[], None, None, None, 1_800_000_000);
        assert!(summary.contains("upstream update comparison unavailable"));
        assert!(summary.contains("0 failed build target(s)"));
    }

    #[test]
    fn package_filters_match_status_and_search_metadata() {
        let metadata = "tool fedora-44 failed 1.2.0 [upstream-update] [failed-build]";
        assert!(package_matches_filter(metadata, "", 0));
        assert!(package_matches_filter(metadata, "TOOL", 0));
        assert!(package_matches_filter(metadata, "fedora-44", 0));
        assert!(package_matches_filter(metadata, "", 1));
        assert!(package_matches_filter(metadata, "tool", 1));
        assert!(!package_matches_filter(metadata, "", 2 + 1));
        assert!(package_matches_filter(metadata, "", 2));
        assert!(!package_matches_filter(metadata, "other", 0));
        assert!(!package_matches_filter(metadata, "", 3));
    }

    #[test]
    fn only_http_and_https_homepages_are_linked() {
        assert!(is_web_url("https://example.com/project"));
        assert!(is_web_url("http://example.com"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("file:///etc/passwd"));
        assert!(!is_web_url("not a url"));
    }
}
