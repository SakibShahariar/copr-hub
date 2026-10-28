mod activity;
mod copr;
mod favorites;
mod storage;
mod system;
mod upstream;

use adw::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

type ProjectSearchResponse = Result<Vec<copr::CoprProject>, String>;

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
    let saved = build_saved_page(&window, &stack);
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

    let results = gtk::ListBox::new();
    let latest_response: Rc<RefCell<Option<ProjectSearchResponse>>> = Rc::new(RefCell::new(None));
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
            search.set_sensitive(false);
            search_button.set_sensitive(false);
            clear_list_box(&results);

            let activity_query = query.clone();
            let results = results.clone();
            let feedback = feedback.clone();
            let search_button = search_button.clone();
            let search = search.clone();
            let parent = parent.clone();
            let latest_response = latest_response.clone();
            let sort = sort.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let response =
                    gtk::gio::spawn_blocking(move || copr::search_projects(&query)).await;
                search.set_sensitive(true);
                search_button.set_sensitive(true);
                let response = match response {
                    Ok(result) => result,
                    Err(_) => Err("The background search task failed unexpectedly.".to_string()),
                };
                let summary = match &response {
                    Ok(projects) => format!("{} result(s)", projects.len()),
                    Err(error) => format!("Failed: {error}"),
                };
                let activity_result = activity::record("Project search", &activity_query, &summary);
                *latest_response.borrow_mut() = Some(response.clone());
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
    heading.append(&title);
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
        clear_list_box(&list);

        match response {
            Ok(Ok(packages)) if packages.is_empty() => {
                feedback
                    .set_text("No installed packages from COPR repositories were reported by DNF.");
            }
            Ok(Ok(packages)) => {
                feedback.set_text(&format!("{} installed COPR package(s)", packages.len()));
                for package in packages {
                    let row = adw::ActionRow::builder()
                        .title(package.name.as_str())
                        .subtitle(format!("{} · {}", package.version, package.repository))
                        .build();
                    row.set_widget_name(&format!(
                        "{} {} {}",
                        package.name, package.version, package.repository
                    ));
                    row.add_prefix(&gtk::Image::from_icon_name("package-x-generic-symbolic"));
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
            Ok(Err(error)) => feedback.set_text(&error),
            Err(_) => feedback.set_text("The background package query failed unexpectedly."),
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
                row.add_suffix(&gtk::Label::new(Some(status)));

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
                    move |_| {
                        confirm_repository_toggle(&parent, &repository, &list, &feedback);
                    }
                });
                row.add_suffix(&toggle);
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
    clear_list_box(list);

    match response {
        Ok(projects) if projects.is_empty() => {
            feedback.set_text("No projects matched that search. Try a different name or owner.");
        }
        Ok(mut projects) => {
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

    let upstream_status = gtk::Label::builder()
        .label("Checking for a latest upstream release when the project homepage points to GitHub or GitLab…")
        .xalign(0.0)
        .wrap(true)
        .selectable(true)
        .build();
    upstream_status.add_css_class("dim-label");
    content.append(&upstream_status);

    let package_filter = gtk::SearchEntry::builder()
        .placeholder_text("Filter packages in this project")
        .tooltip_text("Filter the loaded package list locally")
        .build();
    content.append(&package_filter);

    let packages = gtk::ListBox::new();
    packages.set_selection_mode(gtk::SelectionMode::None);
    packages.add_css_class("boxed-list");
    packages.set_filter_func({
        let package_filter = package_filter.clone();
        move |row| {
            let query = package_filter.text().trim().to_ascii_lowercase();
            query.is_empty() || row.widget_name().to_ascii_lowercase().contains(&query)
        }
    });
    package_filter.connect_search_changed({
        let packages = packages.clone();
        move |_| packages.invalidate_filter()
    });
    content.append(&packages);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&content)
        .build();
    window.set_content(Some(&scroll));
    window.present();

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

    let owner = project.owner.clone();
    let name = project.name.clone();
    let parent = window.clone();
    {
        let monitor_state = monitor_state.clone();
        let release_state = release_state.clone();
        let parent = parent.clone();
        let packages = packages.clone();
        let status = status.clone();
        let homepage = homepage_for_comparison.clone();
        gtk::glib::MainContext::default().spawn_local(async move {
            let response =
                gtk::gio::spawn_blocking(move || copr::fetch_project_monitor(&owner, &name)).await;
            let response = match response {
                Ok(result) => result,
                Err(_) => Err("The background project lookup failed unexpectedly.".to_string()),
            };
            *monitor_state.borrow_mut() = Some(response);
            let response = monitor_state.borrow().clone();
            let release = release_state.borrow().clone();
            if let Some(response) = response {
                render_project_monitor(
                    &parent,
                    &packages,
                    &status,
                    response,
                    release.as_ref(),
                    homepage.as_deref(),
                );
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
        let homepage_for_comparison = Some(homepage.clone());
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
                    upstream_status.set_text(&format!(
                        "Latest upstream stable release: {} ({}) · {}{}{}",
                        release.version,
                        release.name,
                        date,
                        if url.is_empty() {
                            String::new()
                        } else {
                            format!(" · {url}")
                        },
                        stale_notice
                    ));
                    Some(release)
                }
                Ok(Ok(None)) => {
                    upstream_status.set_text(
                        "No stable upstream release metadata is available for this project homepage.",
                    );
                    None
                }
                Ok(Err(error)) => {
                    upstream_status.set_text(&format!("Could not check upstream releases: {error}"));
                    None
                }
                Err(_) => {
                    upstream_status
                        .set_text("The background upstream release lookup failed unexpectedly.");
                    None
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
                    response,
                    release.as_ref(),
                    homepage_for_comparison.as_deref(),
                );
            }
        });
    }
}

fn render_project_monitor(
    parent: &adw::Window,
    list: &gtk::ListBox,
    status: &gtk::Label,
    response: Result<copr::ProjectMonitor, String>,
    upstream_release: Option<&upstream::UpstreamRelease>,
    homepage: Option<&str>,
) {
    clear_list_box(list);

    match response {
        Ok(monitor) if monitor.packages.is_empty() => {
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
            status.set_text(&format!(
                "{} packages · {} package build-target entries. {} Build status is informational, not a security assessment.",
                monitor.packages.len(),
                target_count,
                freshness
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
                        summary
                    })
                    .build();
                row.set_widget_name(&package.name);
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
        Err(error) => status.set_text(&format!(
            "Could not load project build information: {error}"
        )),
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

fn build_saved_page(parent: &adw::ApplicationWindow, stack: &adw::ViewStack) -> gtk::Widget {
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
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    heading.append(&title);
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
            feedback.set_text(&format!("{} saved project(s)", projects.len()));
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
                row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
                row.connect_activated({
                    let parent = parent.clone();
                    move |_| show_project_details(&parent, &project)
                });
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
    let clear = gtk::Button::with_label("Clear history");
    clear.add_css_class("destructive-action");
    let refresh = gtk::Button::with_label("Refresh");
    refresh.set_icon_name("view-refresh-symbolic");
    heading.append(&title);
    heading.append(&clear);
    heading.append(&refresh);
    content.append(&heading);

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
    content.append(&list);

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
                row.add_prefix(&gtk::Image::from_icon_name("document-properties-symbolic"));
                list.append(&row);
            }
        }
        Err(error) => feedback.set_text(&error),
    }
}

#[cfg(test)]
mod url_tests {
    use super::is_web_url;

    #[test]
    fn only_http_and_https_homepages_are_linked() {
        assert!(is_web_url("https://example.com/project"));
        assert!(is_web_url("http://example.com"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("file:///etc/passwd"));
        assert!(!is_web_url("not a url"));
    }
}
