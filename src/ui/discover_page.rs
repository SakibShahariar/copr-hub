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

    let hero = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    hero.set_hexpand(true);
    let hero_icon = gtk::Image::from_icon_name("system-software-install-symbolic");
    hero_icon.set_pixel_size(40);
    hero_icon.set_valign(gtk::Align::Start);
    hero.append(&hero_icon);

    let hero_text = gtk::Box::new(gtk::Orientation::Vertical, 6);
    hero_text.set_hexpand(true);
    let hero_title = gtk::Label::new(Some("Find your next COPR project"));
    hero_title.set_xalign(0.0);
    hero_title.add_css_class("title-1");
    hero_text.append(&hero_title);
    let hero_description = gtk::Label::new(Some(
        "Explore community-built software for Fedora. Search projects, inspect their details, and decide what to add to your system.",
    ));
    hero_description.set_xalign(0.0);
    hero_description.set_wrap(true);
    hero_description.add_css_class("dim-label");
    hero_text.append(&hero_description);
    hero.append(&hero_text);
    content.append(&hero);

    let search_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    search_row.set_hexpand(true);

    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search COPR projects, packages, or owners")
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
    let sort =
        gtk::DropDown::from_strings(&["Most relevant", "COPR order", "Project name", "Owner"]);
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
                render_projects(
                    &parent,
                    &results,
                    &feedback,
                    response,
                    &activity_query,
                    sort.selected(),
                );
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
        let cached_query = cached.query;
        render_projects(
            parent,
            &results,
            &feedback,
            cached_response,
            &cached_query,
            sort.selected(),
        );
        feedback.set_text(&format!(
            "Showing {count} saved result(s) for “{cached_query}”. Cached results may be out of date; search to refresh."
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
        let search = search.clone();
        let sort = sort.clone();
        move |_| {
            if let Some(response) = latest_response.borrow().clone() {
                render_projects(
                    &parent,
                    &results,
                    &feedback,
                    response,
                    search.text().trim(),
                    sort.selected(),
                );
            }
        }
    });

    let clamp = adw::Clamp::builder()
        .maximum_size(640)
        .child(&content)
        .build();
    scroll_page(&clamp)
}

fn normalize_search_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();

    for (left_index, left_character) in left.chars().enumerate() {
        let mut current = vec![left_index + 1; right.len() + 1];
        for (right_index, right_character) in right.iter().enumerate() {
            let substitution_cost = if left_character == *right_character { 0 } else { 1 };
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous = current;
    }

    previous[right.len()]
}

fn fuzzy_name_distance(name: &str, query: &str) -> Option<usize> {
    let query_length = query.chars().count();
    if query_length < 4 {
        return None;
    }

    let max_distance = if query_length <= 6 { 1 } else { 2 };
    let name: Vec<char> = name.chars().collect();
    let shortest = query_length.saturating_sub(max_distance).max(1);
    let longest = (query_length + max_distance).min(name.len());
    if shortest > longest {
        return None;
    }

    (shortest..=longest)
        .flat_map(|window_length| name.windows(window_length))
        .map(|window| {
            let candidate: String = window.iter().collect();
            edit_distance(&candidate, query)
        })
        .filter(|distance| *distance <= max_distance)
        .min()
}

fn project_relevance(project: &copr::CoprProject, query: &str) -> (u8, usize) {
    let query = normalize_search_text(query);
    if query.is_empty() {
        return (0, 0);
    }

    let name = normalize_search_text(&project.name);
    if name == query {
        return (0, 0);
    }
    if name.starts_with(&query) {
        return (1, name.chars().count() - query.chars().count());
    }
    if name.contains(&query) {
        return (2, name.find(&query).unwrap_or_default());
    }

    let owner = normalize_search_text(&project.owner);
    if owner == query {
        return (3, 0);
    }
    if owner.starts_with(&query) {
        return (4, owner.chars().count() - query.chars().count());
    }
    if owner.contains(&query) {
        return (5, owner.find(&query).unwrap_or_default());
    }

    // Unknown fields may include an exact package-name hit from COPR.
    // Keep those results in server order rather than guessing from metadata.
    if let Some(distance) = fuzzy_name_distance(&name, &query) {
        return (7, distance);
    }
    if let Some(distance) = fuzzy_name_distance(&owner, &query) {
        return (8, distance);
    }

    let description = project
        .description
        .as_deref()
        .map(normalize_search_text)
        .unwrap_or_default();
    if description.contains(&query) {
        return (9, description.find(&query).unwrap_or_default());
    }

    (6, 0)
}

fn rank_projects(projects: &mut [copr::CoprProject], query: &str) {
    // Stable sorting keeps COPR's server order when relevance scores tie.
    projects.sort_by_key(|project| project_relevance(project, query));
}

fn render_projects(
    parent: &adw::ApplicationWindow,
    list: &gtk::ListBox,
    feedback: &gtk::Label,
    response: Result<Vec<copr::CoprProject>, String>,
    query: &str,
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
                0 => rank_projects(&mut projects, query),
                2 => projects.sort_by(|a, b| {
                    a.name
                        .to_ascii_lowercase()
                        .cmp(&b.name.to_ascii_lowercase())
                        .then_with(|| {
                            a.owner
                                .to_ascii_lowercase()
                                .cmp(&b.owner.to_ascii_lowercase())
                        })
                }),
                3 => projects.sort_by(|a, b| {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn project(name: &str, owner: &str, description: &str) -> copr::CoprProject {
        copr::CoprProject {
            name: name.to_string(),
            owner: owner.to_string(),
            description: Some(description.to_string()),
            homepage: None,
        }
    }

    #[test]
    fn relevance_ranks_exact_and_prefix_project_names_first() {
        let mut projects = vec![
            project("applications", "alice", "Includes Zen Browser"),
            project("zen-browser-nightly", "bob", "Nightly builds"),
            project("zen-browser", "carol", "The browser"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].name, "zen-browser");
        assert_eq!(projects[1].name, "zen-browser-nightly");
        assert_eq!(projects[2].name, "applications");
    }

    #[test]
    fn fuzzy_project_name_match_beats_description_only_match() {
        let mut projects = vec![
            project("applications", "alice", "Includes Zen Browser"),
            project("zen-brower", "bob", "A browser build"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].name, "zen-brower");
        assert_eq!(projects[1].name, "applications");
    }

    #[test]
    fn exact_owner_match_ranks_above_description_match() {
        let mut projects = vec![
            project("applications", "alice", "Zen Browser packages"),
            project("unrelated", "zen-browser", "Other software"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].owner, "zen-browser");
    }

    #[test]
    fn equal_relevance_preserves_copr_server_order() {
        let mut projects = vec![
            project("zen-browser-nightly", "alice", "Nightly"),
            project("zen-browser-stable", "bob", "Stable"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].name, "zen-browser-nightly");
        assert_eq!(projects[1].name, "zen-browser-stable");
    }

    #[test]
    fn fuzzy_matching_checks_inside_compound_project_names() {
        let mut projects = vec![
            project("applications", "alice", "Includes Zen Browser"),
            project("zen-brower-nightly", "bob", "Nightly builds"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].name, "zen-brower-nightly");
    }

    #[test]
    fn unknown_matches_keep_server_order_ahead_of_fuzzy_guesses() {
        let mut projects = vec![
            project("unrelated-project", "alice", "Package name is not exposed"),
            project("zen-brower", "bob", "A typo in the project name"),
        ];

        rank_projects(&mut projects, "zen-browser");

        assert_eq!(projects[0].name, "unrelated-project");
        assert_eq!(projects[1].name, "zen-brower");
    }

    #[test]
    fn short_queries_do_not_trigger_fuzzy_name_matches() {
        let exact = project("zena", "alice", "Example");
        let typo = project("zenb", "bob", "Example");

        assert_eq!(project_relevance(&typo, "zen"), (9, 0));
        assert_eq!(project_relevance(&exact, "zena"), (0, 0));
    }
}
