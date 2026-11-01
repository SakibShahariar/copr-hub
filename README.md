# Copr Hub

[![CI](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Linux](https://img.shields.io/badge/platform-Linux-informational)

**Discover, inspect, and manage Fedora COPR projects from a native Linux desktop app.**

Copr Hub is built with Rust, GTK4, and libadwaita, with a GNOME-friendly interface that follows your active system theme.

## What you can do

- **Find projects** — search COPR, sort results, and filter locally.
- **Understand build health** — inspect package versions and Fedora build targets, configure freshness thresholds, and review health changes in local activity history.
- **Spot upstream updates** — compare supported package versions with GitHub/GitLab releases, explicitly skip prereleases and unsupported version formats, and label cached data.
- **Track and compare projects** — save favorites, compare build health, optionally receive change-only notifications, preview backup imports, and enable scheduled local backups.
- **Manage COPR repositories** — check Fedora build compatibility before enabling a project, inspect repository configuration and installed package origins, and manage supported COPR definitions.
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

## Install a prebuilt release

Tagged releases include an x86_64 Linux archive with the application binary and desktop integration files. The binary uses the system GTK4 and libadwaita libraries; on Fedora, install them with:

```bash
sudo dnf install gtk4 libadwaita
```

Download the archive and its `.sha256` file from [GitHub Releases](https://github.com/SakibShahariar/copr-hub/releases), then verify and install them:

```bash
ARCHIVE="copr-hub-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz"
sha256sum -c "$ARCHIVE.sha256"
tar -xzf "$ARCHIVE"

install -Dm755 copr-hub/bin/copr-hub ~/.local/bin/copr-hub
install -Dm644 copr-hub/share/applications/io.github.SakibShahariar.CoprHub.desktop ~/.local/share/applications/io.github.SakibShahariar.CoprHub.desktop
install -Dm644 copr-hub/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml ~/.local/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml
install -Dm644 copr-hub/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg
```

Replace `X.Y.Z` with the release version. The source installer above remains available when you prefer to build locally.

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
