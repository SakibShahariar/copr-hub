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

    let discover = build_discover_page();
    let repositories = build_placeholder_page(
        "folder-symbolic",
        "Your repositories",
        "COPR repositories enabled on this system will appear here.",
    );
    let installed = build_placeholder_page(
        "package-x-generic-symbolic",
        "Installed packages",
        "Packages installed from COPR repositories will appear here.",
    );
    let activity = build_placeholder_page(
        "document-properties-symbolic",
        "Recent activity",
        "Repository changes and package operations will be recorded here.",
    );

    add_page(&stack, &discover, "discover", "Discover", "system-search-symbolic");
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

fn build_discover_page() -> gtk::Widget {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 24);
    content.set_margin_top(36);
    content.set_margin_bottom(36);
    content.set_margin_start(32);
    content.set_margin_end(32);
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Start);
    content.set_hexpand(true);
    content.set_width_request(560);

    let hero = adw::StatusPage::builder()
        .icon_name("system-software-install-symbolic")
        .title("Find your next COPR project")
        .description(
            "Explore community-built software for Fedora. Search projects, inspect their packages, and decide what to add to your system.",
        )
        .build();
    hero.set_hexpand(true);
    content.append(&hero);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search projects, packages, or owners")
        .tooltip_text("Search the COPR project catalogue")
        .hexpand(true)
        .build();
    content.append(&search);

    let note = gtk::Label::builder()
        .label("Project search is the next feature to be connected.")
        .wrap(true)
        .css_classes(["dim-label"])
        .build();
    content.append(&note);

    content.upcast()
}

fn build_placeholder_page(icon_name: &str, title: &str, description: &str) -> gtk::Widget {
    let page = adw::StatusPage::builder()
        .icon_name(icon_name)
        .title(title)
        .description(description)
        .build();
    page.upcast()
}
