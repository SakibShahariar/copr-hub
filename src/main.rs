mod activity;
mod backups;
mod copr;
mod favorites;
mod monitoring;
mod snapshots;
mod storage;
mod system;
mod upstream;
mod ui;

use adw::prelude::*;

fn main() {
    let app = adw::Application::builder()
        .application_id("io.github.SakibShahariar.CoprHub")
        .build();

    app.connect_activate(ui::build_ui);
    app.run();
}

