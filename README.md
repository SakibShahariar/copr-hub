# Copr Hub

[![CI](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Linux](https://img.shields.io/badge/platform-Linux-informational)

**Discover, inspect, and manage Fedora COPR projects from a native Linux desktop app.**

Copr Hub is built with Rust, GTK4, and libadwaita, with a GNOME-friendly interface that follows your active system theme.

## What you can do

- **Find projects** — search COPR, sort results, and filter locally.
- **Understand build health** — inspect package versions, Fedora build targets, failed/in-progress builds, and project-build freshness.
- **Spot upstream updates** — compare supported package versions with GitHub/GitLab stable releases, with clearly labeled cached data and freshness hints.
- **Track and compare projects** — save favorites, compare build health, optionally receive change-only notifications, and export/import saved-project backups.
- **Manage COPR repositories** — inspect repository configuration, enable/disable repositories, and remove supported COPR definitions after confirmation.
- **Manage packages** — check enabled repositories for available upgrades to installed COPR packages, inspect direct dependency impact, install packages, or remove them with explicit confirmation.
- **Keep a local history** — search, export, import, or clear recent activity.
- **Work through interruptions** — cached project details, recent builds, upstream metadata, saved projects, activity, and local repository information remain useful offline with visible freshness labels.

Build freshness and upstream-age hints are informational signals, not security assessments. Offline support is partial; live COPR searches and fresh remote data require connectivity. See the [detailed feature and limitations guide](docs/FEATURES.md).

## Install on Fedora

Install build dependencies:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Install [Rust with rustup](https://rustup.rs/), clone this repository, then run:

```bash
git clone https://github.com/SakibShahariar/copr-hub.git
cd copr-hub
sh scripts/install-local.sh
```

The per-user installer builds the release binary and installs the launcher under `~/.local`; it does not require root privileges. To uninstall the launcher and application metadata:

```bash
sh scripts/uninstall-local.sh
```

Uninstalling preserves your saved projects and activity history.

## Build from source

```bash
cargo run
```

Run the same core checks used by CI:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo check
cargo clippy --all-targets --all-features -- -D warnings
```

## Local data

Copr Hub keeps saved projects, activity history, and upstream release cache in your user account's XDG data/cache directories. These files stay local and are not uploaded. See [Local data and privacy](docs/FEATURES.md#local-data-and-privacy) for paths and storage details.

## Project

- **Application ID:** `io.github.SakibShahariar.CoprHub`
- **License:** [MIT](LICENSE)
- **Detailed documentation:** [Features, offline behavior, and limitations](docs/FEATURES.md)
