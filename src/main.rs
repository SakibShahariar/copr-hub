mod copr;
mod system;

use adw::prelude::*;
use gtk::prelude::*;

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
    let repositories = build_repositories_page();
    let installed = build_placeholder_page(
        "package-x-generic-symbolic",
        "Installed packages",
        "Installed packages and their source repositories will appear here.",
    );
    let activity = build_placeholder_page(
        "document-properties-symbolic",
        "Recent activity",
        "Repository changes and package operations will be recorded here.",
    );

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
    add_page(
        &stack,
        &activity,
        "activity",
        "Activity",
        "document-properties-symbolic",
    );

    let switcher_title = adw::ViewSwitcherTitle::builder()
        .stack(&stack)
        .title("Copr Hub")
        .build();

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&switcher_title));

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&header);
    root.append(&stack);

    window.set_content(Some(&root));
    window.present();
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
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Start);
    content.set_hexpand(true);
    content.set_width_request(640);

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
        .placeholder_text("Search projects, packages, or owners")
        .tooltip_text("Enter a query and press Enter to search COPR")
        .hexpand(true)
        .build();
    let search_button = gtk::Button::with_label("Search");
    search_button.add_css_class("suggested-action");
    search_row.append(&search);
    search_row.append(&search_button);
    content.append(&search_row);

    let feedback = gtk::Label::builder()
        .label("Search the COPR catalogue to find community projects.")
        .wrap(true)
        .xalign(0.0)
        .build();
    feedback.add_css_class("dim-label");
    content.append(&feedback);

    let results = gtk::ListBox::new();
    results.set_selection_mode(gtk::SelectionMode::None);
    results.add_css_class("boxed-list");
    content.append(&results);

    let perform_search = {
        let search = search.clone();
        let results = results.clone();
        let feedback = feedback.clone();
        let search_button = search_button.clone();
        let parent = parent.clone();
        move || {
            let query = search.text().trim().to_string();
            if query.is_empty() {
                feedback.set_text("Enter a project, package, or owner to search.");
                return;
            }

            feedback.set_text("Searching COPR…");
            search_button.set_sensitive(false);
            clear_list_box(&results);

            let results = results.clone();
            let feedback = feedback.clone();
            let search_button = search_button.clone();
            let parent = parent.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                let response =
                    gtk::gio::spawn_blocking(move || copr::search_projects(&query)).await;
                search_button.set_sensitive(true);
                render_projects(&parent, &results, &feedback, response);
            });
        }
    };

    search.connect_search_activated({
        let perform_search = perform_search.clone();
        move |_| perform_search()
    });
    search_button.connect_clicked(move |_| perform_search());

    content.upcast()
}

fn build_repositories_page() -> gtk::Widget {
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
    content.append(&list);

    refresh.connect_clicked({
        let list = list.clone();
        let feedback = feedback.clone();
        move |_| refresh_repositories(&list, &feedback)
    });
    refresh_repositories(&list, &feedback);

    content.upcast()
}

fn refresh_repositories(list: &gtk::ListBox, feedback: &gtk::Label) {
    feedback.set_text("Reading COPR repository configuration…");
    clear_list_box(list);

    let list = list.clone();
    let feedback = feedback.clone();
    gtk::glib::MainContext::default().spawn_local(async move {
        let response = gtk::gio::spawn_blocking(system::scan_copr_repositories).await;
        clear_list_box(&list);

        match response {
            Ok(Ok(repositories)) if repositories.is_empty() => {
                feedback.set_text(
                    "No COPR repositories were found in /etc/yum.repos.d. Add a COPR repository with DNF to see it here.",
                );
            }
            Ok(Ok(repositories)) => {
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
                    row.add_prefix(&gtk::Image::from_icon_name("folder-symbolic"));
                    row.add_suffix(&gtk::Label::new(Some(status)));
                    list.append(&row);
                }
            }
            Ok(Err(error)) => feedback.set_text(&error),
            Err(error) => feedback.set_text(&format!("Could not inspect repositories: {error}")),
        }
    });
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
) {
    clear_list_box(list);

    match response {
        Ok(projects) if projects.is_empty() => {
            feedback.set_text("No projects matched that search. Try a different name or owner.");
        }
        Ok(projects) => {
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

fn show_project_details(parent: &adw::ApplicationWindow, project: &copr::CoprProject) {
    let window = adw::Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(project.name.as_str())
        .default_width(560)
        .default_height(420)
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

    if let Some(homepage) = project.homepage.as_deref().filter(|url| !url.is_empty()) {
        let link = gtk::LinkButton::with_label(homepage, "Open project homepage");
        link.set_halign(gtk::Align::Center);
        content.append(&link);
    }

    let note = gtk::Label::builder()
        .label(
            "Build targets, package availability, and repository actions will be shown here as project inspection is expanded.",
        )
        .wrap(true)
        .build();
    note.add_css_class("dim-label");
    content.append(&note);

    window.set_content(Some(&content));
    window.present();
}

fn build_placeholder_page(icon_name: &str, title: &str, description: &str) -> gtk::Widget {
    let page = adw::StatusPage::builder()
        .icon_name(icon_name)
        .title(title)
        .description(description)
        .build();
    page.upcast()
}
