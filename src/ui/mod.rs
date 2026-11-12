mod activity_page;
mod discover_page;
mod installed_page;
mod project_details;
mod repositories_page;
mod saved_page;
use crate::{activity, backups, copr, favorites, monitoring, snapshots, system, upstream};
use adw::prelude::*;
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;

type ProjectSearchResponse = Result<Vec<copr::CoprProject>, String>;

fn clear_legacy_search_cache() {
    let path = gtk::glib::user_cache_dir()
        .join("copr-hub")
        .join("last-search.json");
    // Older versions restored the previous query and results on startup.
    // The cache is no longer used; remove it so stale results do not linger.
    let _ = fs::remove_file(path);
}

pub(crate) fn build_ui(app: &adw::Application) {
    clear_legacy_search_cache();
    if let Ok(projects) = favorites::load() {
        let _ = monitoring::prune_to_projects(&projects);
    }
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Copr Hub")
        .default_width(1080)
        .default_height(720)
        .build();

    // Keep the app's interface readable even when the desktop's chosen UI font
    // is monospace. The generic family follows the user's installed sans-serif
    // font; colors remain entirely theme-driven for Adwaita and Matugen palettes.
    let typography = gtk::CssProvider::new();
    typography.load_from_data("window.copr-hub { font-family: sans-serif; }");
    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(&window),
        &typography,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    window.add_css_class("copr-hub");

    let stack = adw::ViewStack::new();
    stack.set_vexpand(true);

    let discover = discover_page::build_discover_page(&window);
    let repositories = repositories_page::build_repositories_page(&window);
    let installed = installed_page::build_installed_page(&window);
    let saved = saved_page::build_saved_page(&window, &stack, app);
    let activity = activity_page::build_activity_page(&window, &stack);

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

    // ViewSwitcherTitle was deprecated in libadwaita 1.4. Use the
    // adaptive ViewSwitcher directly for the application's primary navigation.
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

pub(super) fn launch_manual_monitor(button: &gtk::Button, feedback: &gtk::Label) {
    button.set_sensitive(false);
    feedback.set_text("Checking saved projects…");
    let button = button.clone();
    let feedback = feedback.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        match gtk::gio::spawn_blocking(check_saved_projects).await {
            Ok(notifications) => {
                let changed = notifications.len();
                feedback.set_text(&format!(
                    "Health check finished. {} project(s) reported new changes.",
                    changed
                ));
            }
            Err(error) => feedback.set_text(&format!("Health check failed: {error:?}")),
        }
        button.set_sensitive(true);
    });
}

fn check_saved_projects() -> Vec<(String, String, String)> {
    let Ok(projects) = favorites::load() else {
        return Vec::new();
    };
    let observations = upstream::monitoring_cache_batch(|| {
        let mut observations = Vec::new();
        for project in projects {
            let (monitor, history) =
                match copr::fetch_project_monitor_with_history(&project.owner, &project.name) {
                    Ok(result) => result,
                    Err(error) if copr::should_stop_monitoring_scan(&error) => {
                        // Network failures, rate limits, and service outages affect the
                        // whole scan. Avoid repeating the same failing request per project.
                        eprintln!("Stopping COPR monitoring scan: {error}");
                        break;
                    }
                    Err(error) => {
                        // A project-specific failure (for example, a missing project or
                        // permission error) should not prevent checks for later projects.
                        eprintln!(
                            "Skipping COPR monitoring for @{} / {}: {error}",
                            project.owner, project.name
                        );
                        continue;
                    }
                };
            let release = project.homepage.as_deref().and_then(|homepage| {
                upstream::latest_release_for_monitoring_batch(homepage)
                    .ok()
                    .flatten()
            });
            let cached_release = release
                .as_ref()
                .filter(|release| !release.stale_cache)
                .cloned();
            let _ =
                snapshots::save_monitoring_data(&project, monitor.clone(), history, cached_release);
            observations.push(monitoring::MonitoringObservation {
                project,
                monitor,
                release,
            });
        }

        observations
    });
    let recorded = match monitoring::record_observations(&observations) {
        Ok(recorded) => recorded,
        Err(error) => {
            eprintln!(
                "Could not save monitoring state; this cycle's alerts were suppressed: {error}"
            );
            return Vec::new();
        }
    };
    recorded
        .into_iter()
        .filter_map(|(owner, name, alerts)| {
            if alerts.is_empty() {
                return None;
            }
            Some((
                format!("copr-hub-{owner}-{name}"),
                format!("{name} · @{owner}"),
                alerts.join(" "),
            ))
        })
        .collect()
}

fn add_loading_bar(parent: &gtk::Box) -> gtk::ProgressBar {
    let progress = gtk::ProgressBar::new();
    progress.set_show_text(false);
    progress.set_pulse_step(0.08);
    progress.set_visible(false);
    progress.add_css_class("osd");
    parent.append(&progress);

    progress.connect_notify_local(Some("visible"), |progress, _| {
        if !progress.is_visible() {
            return;
        }
        let progress = progress.clone();
        gtk::glib::timeout_add_local(std::time::Duration::from_millis(90), move || {
            if !progress.is_visible() {
                return gtk::glib::ControlFlow::Break;
            }
            progress.pulse();
            gtk::glib::ControlFlow::Continue
        });
    });
    progress
}

fn scroll_page(child: &impl IsA<gtk::Widget>) -> gtk::Widget {
    // Keep page content comfortably readable on wide displays while allowing
    // the clamp to tighten naturally on smaller windows and larger text scales.
    let clamp = adw::Clamp::builder()
        .maximum_size(960)
        .tightening_threshold(720)
        .child(child)
        .build();
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&clamp)
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

fn clear_list_box(list: &gtk::ListBox) {
    while let Some(child) = list.first_child() {
        list.remove(&child);
    }
}

#[cfg(test)]
mod url_tests {
    use super::project_details::{is_web_url, package_matches_filter, update_intelligence_summary};
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
