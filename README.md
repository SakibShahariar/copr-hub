# Copr Hub

[![CI](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Linux](https://img.shields.io/badge/platform-Linux-informational)

**Discover, inspect, and manage Fedora COPR projects from a native Linux desktop app.**

Copr Hub combines COPR search, build and upstream information, saved-project monitoring, and repository/package management in a GTK4 and libadwaita interface that follows your system theme.

## Features

- **Project discovery:** Search COPR, inspect build status and Fedora targets, and review recent builds.
- **Upstream intelligence:** Compare package versions with GitHub/GitLab releases and spot potentially outdated projects.
- **Project monitoring:** Save and compare projects, track build-health changes, and receive optional notifications.
- **Repository and package management:** Check Fedora compatibility, manage COPR repositories, inspect installed packages, and handle package operations.
- **History and offline access:** Keep local activity history, backups, and cached project information available between sessions.

Copr Hub uses `pkexec` for privileged operations rather than running as root. COPR build-health and freshness indicators are informational, not security assessments.

See the [features and limitations guide](docs/FEATURES.md) for detailed functionality, security considerations, and offline behavior.

## Install on Fedora

### Build from source

Install dependencies and [Rust](https://rustup.rs/), then run:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
git clone https://github.com/SakibShahariar/copr-hub.git
cd copr-hub
sh scripts/install-local.sh
```

The installer builds a release binary and installs the launcher under `~/.local` without root privileges. To uninstall the launcher and application metadata:

```bash
sh scripts/uninstall-local.sh
```

Your saved projects and activity history are preserved.

### Prebuilt release

Download the x86_64 Linux archive and its SHA-256 file from [GitHub Releases](https://github.com/SakibShahariar/copr-hub/releases). Verify and extract it:

```bash
ARCHIVE="copr-hub-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz"
sha256sum -c "$ARCHIVE.sha256"
tar -xzf "$ARCHIVE"
```

Install the extracted binary and desktop files:

```bash
install -Dm755 copr-hub/bin/copr-hub ~/.local/bin/copr-hub
install -Dm644 copr-hub/share/applications/io.github.SakibShahariar.CoprHub.desktop ~/.local/share/applications/io.github.SakibShahariar.CoprHub.desktop
install -Dm644 copr-hub/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml ~/.local/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml
install -Dm644 copr-hub/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg
```

Replace `X.Y.Z` with the release version. The binary uses system GTK4 and libadwaita libraries (`sudo dnf install gtk4 libadwaita` on Fedora).

## Build and test

Run from the source tree with `cargo run`. Core checks:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo check
cargo clippy --all-targets --all-features -- -D warnings
```

## Local data and privacy

Saved projects, activity history, monitoring state, project snapshots, and search/release caches stay in your user account's XDG directories. They are not uploaded, and Copr Hub has no telemetry feature. Review JSON exports before sharing them.

See [Local data and privacy](docs/FEATURES.md#local-data-and-privacy) for exact paths and storage details.

## Project

- **Application ID:** `io.github.SakibShahariar.CoprHub`
- **License:** [MIT](LICENSE)
- **Documentation:** [Features, offline behavior, security, and limitations](docs/FEATURES.md)
