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
        .default_width(1000)
        .default_height(700)
        .build();

    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let title = gtk::Label::builder()
        .label("Copr Hub")
        .css_classes(["title"])
        .build();
    header.set_title_widget(Some(&title));
    root.append(&header);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search COPR projects and packages")
        .hexpand(true)
        .margin_top(18)
        .margin_bottom(12)
        .margin_start(18)
        .margin_end(18)
        .build();
    root.append(&search);

    let navigation = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    navigation.set_halign(gtk::Align::Center);
    navigation.set_margin_bottom(12);

    let status = adw::StatusPage::builder()
        .icon_name("system-search-symbolic")
        .title("Discover COPR projects")
        .description(
            "Search community-built Fedora packages and explore projects. \
             Project discovery and repository management are coming next.",
        )
        .build();

    for (label, icon, page_title, description) in [
        (
            "Discover",
            "system-search-symbolic",
            "Discover COPR projects",
            "Search community-built Fedora packages and explore projects.",
        ),
        (
            "Repositories",
            "folder-symbolic",
            "Your repositories",
            "Enabled COPR repositories will appear here when repository management is implemented.",
        ),
        (
            "Packages",
            "package-x-generic-symbolic",
            "Packages",
            "Inspect package details and installation options from COPR projects.",
        ),
    ] {
        let button = gtk::Button::builder()
            .label(label)
            .icon_name(icon)
            .build();
        button.add_css_class("flat");
        let status_page = status.clone();
        button.connect_clicked(move |_| {
            status_page.set_icon_name(Some(icon));
            status_page.set_title(page_title);
            status_page.set_description(description);
        });
        navigation.append(&button);
    }

    root.append(&navigation);

    search.connect_search_changed({
        let status = status.clone();
        move |entry| {
            let query = entry.text();
            if query.trim().is_empty() {
                status.set_title("Discover COPR projects");
                status.set_description(
                    "Search community-built Fedora packages and explore projects. \
                     Project discovery and repository management are coming next.",
                );
            } else {
                status.set_title("COPR search");
                status.set_description(&format!(
                    "Search for “{}” will be available when COPR API integration is added.",
                    query.trim()
                ));
            }
        }
    });

    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    root.append(&status);
    window.set_content(Some(&root));
    window.present();
}
