use super::project_details::confirm_package_operation;
use super::*;

pub(super) fn build_installed_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
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

    let filter_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let all_filter = gtk::ToggleButton::with_label("All");
    all_filter.add_css_class("pill");
    all_filter.set_active(true);
    let updates_filter = gtk::ToggleButton::with_label("Updates available");
    updates_filter.add_css_class("pill");
    updates_filter.set_group(Some(&all_filter));
    filter_row.append(&all_filter);
    filter_row.append(&updates_filter);
    content.append(&filter_row);

    let filter_mode = Rc::new(std::cell::Cell::new(false));
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
        let filter_mode = filter_mode.clone();
        move |row| {
            let metadata = row.widget_name().to_ascii_lowercase();
            let updates_only = filter_mode.get();
            if updates_only && !metadata.contains("[update-available]") {
                return false;
            }
            let query = filter_query.borrow().trim().to_ascii_lowercase();
            query.is_empty() || metadata.contains(&query)
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

    all_filter.connect_toggled({
        let filter_mode = filter_mode.clone();
        let list = list.clone();
        move |button| {
            if button.is_active() {
                filter_mode.set(false);
                list.invalidate_filter();
            }
        }
    });
    updates_filter.connect_toggled({
        let filter_mode = filter_mode.clone();
        let list = list.clone();
        move |button| {
            if button.is_active() {
                filter_mode.set(true);
                list.invalidate_filter();
            }
        }
    });

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
            "{} {} {} {}{}",
            package.name,
            package.version,
            package.repository,
            update_text.as_deref().unwrap_or(""),
            if updates.contains_key(&package.name) {
                " [update-available]"
            } else {
                ""
            }
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
