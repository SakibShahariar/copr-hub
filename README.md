# Copr Hub

[![CI](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Linux](https://img.shields.io/badge/platform-Linux-informational)

**Discover, inspect, and manage Fedora COPR projects from a native Linux desktop app.**

Copr Hub combines COPR search, build and upstream information, saved-project monitoring, and repository/package management in a GTK4 and libadwaita interface that follows your system theme.

## Features

### Discover and inspect

- Search public COPR projects, sort results, and filter locally by name, owner, or description.
- Inspect package versions, Fedora build targets, build states, and a package-by-target matrix.
- Review recent build records, available timestamps, and direct COPR build links.
- Compare package versions with stable GitHub or GitLab releases when a project links to a supported upstream repository.
- See build-age and upstream-freshness hints. Uncertain package matches and unsupported version formats are not guessed.

### Track projects

- Save projects locally and run on-demand health checks.
- Optionally monitor saved projects while the app is open (disabled by default; checks every six hours).
- Get change-only notifications for newly failed builds, changed stable upstream releases, or projects crossing the configured no-build threshold.
- Compare two or three saved projects by package count, build-target state, and latest-build freshness.
- Export/import saved projects, preview imports, and optionally schedule local JSON backups while the app is running.

### Manage repositories and packages

- Check reported Fedora build targets before enabling a COPR repository. Incompatible targets require an explicit **Enable anyway** choice.
- Inspect repository definitions and installed package origins reported by DNF.
- Enable, disable, or remove supported COPR repository definitions with confirmation.
- Find installed COPR packages, check enabled repositories for available upgrades, and inspect direct dependency impact.
- Install or remove packages from project details with explicit confirmation.

Privileged actions use `pkexec` and DNF; Copr Hub itself does not run as root. Removing a repository definition does not uninstall packages installed from it.

### History and offline access

Search, export, import, or clear a local activity history of recent searches and changes. Saved projects, history, repository configuration, installed-package listings, and cached project details remain available offline where possible. Cached data is labeled because it may be stale; live searches and fresh remote data require connectivity.

**Trust matters:** COPR projects are third-party repositories. Build health, Fedora compatibility, and upstream freshness are not security assessments. Review a project's maintainer, source, and build history before enabling an unfamiliar repository.

See the [features and limitations guide](docs/FEATURES.md) for detailed behavior, security notes, offline limits, and local data paths.

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
