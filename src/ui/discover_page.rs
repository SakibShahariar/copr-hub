use super::project_details::show_project_details;
use super::*;

pub(super) fn build_discover_page(parent: &adw::ApplicationWindow) -> gtk::Widget {
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
