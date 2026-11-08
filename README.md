# Copr Hub

[![CI](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml/badge.svg)](https://github.com/SakibShahariar/copr-hub/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
![Linux](https://img.shields.io/badge/platform-Linux-informational)

**Discover, understand, and manage Fedora COPR projects from a native Linux desktop app.**

Copr Hub brings COPR project search, build information, upstream release checks, saved-project monitoring, and repository/package actions into one GTK4 and libadwaita application. It follows your active system theme and keeps your saved projects and activity data on your machine.

## Explore and understand projects

- **Find COPR projects:** Search the public COPR service, sort results, and filter locally by project name, owner, or description.
- **Inspect build health:** Review package versions, Fedora build targets, build states, and a package-by-target matrix. Missing target data is shown as unavailable—not misreported as a failure.
- **Review recent builds:** Open a project timeline with recent build records, timestamps when available, and direct links to COPR builds.
- **Check upstream releases:** For projects that link to supported GitHub or GitLab repositories, compare package versions with the latest stable upstream release. Unsupported version formats and uncertain package matches are not guessed.
- **Use freshness as a hint:** See stale-build and upstream-age indicators, with configurable build-age thresholds for saved-project checks. These signals describe freshness, not security or project trust.

## Keep track of projects

- Save projects locally and open them later without searching again.
- Run on-demand health checks or optionally monitor saved projects while Copr Hub is open. Monitoring is disabled by default and checks every six hours.
- Receive change-only desktop notifications for newly failed builds, changed stable upstream releases, or projects crossing the configured no-build threshold. The first check establishes a baseline.
- Compare two or three saved projects side by side using package counts, build-target states, and latest-build freshness.
- Export and import saved projects as JSON, preview imports before confirming, and optionally keep a rolling set of local backups while the app is running.

## Manage repositories and packages

- Check a project's reported Fedora build targets against your Fedora release before enabling its COPR repository. Incompatible targets require an explicit **Enable anyway** choice; unknown compatibility is not guessed.
- Inspect enabled and disabled COPR repository definitions and the installed package origins reported by DNF.
- Enable, disable, or remove supported COPR repository definitions with confirmation.
- Find installed packages from COPR repositories, check enabled repositories for available upgrades, and inspect direct dependency impact.
- Install or remove packages from project details with explicit confirmation.

Privileged repository and package operations use `pkexec` and DNF. Copr Hub itself does not run as root. Removing a repository definition does not uninstall packages previously installed from it.

## Activity and offline use

Copr Hub keeps a searchable local activity history of recent searches, saved-project changes, package operations, and successful repository changes. You can export, import, or clear that history.

Previously saved projects, activity, repository configuration, and installed-package listings remain available offline where they rely on local data. Project details, recent build records, upstream metadata, and the last successful search can also be restored from local caches. Cached information is clearly marked because it may be out of date. Live searches, fresh remote data, and operations that need remote DNF metadata still require connectivity.

**Copr Hub is not a package security scanner.** COPR projects are third-party repositories. Build health, Fedora compatibility, and upstream freshness do not prove that a project or its packages are safe. Review the maintainer, source, build history, and project information before enabling an unfamiliar repository.

For the complete behavior, security notes, data paths, and limitations, see the [features and limitations guide](docs/FEATURES.md).

## Install on Fedora

### Build and install from source

Install the build dependencies:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Install [Rust with rustup](https://rustup.rs/), then clone and install:

```bash
git clone https://github.com/SakibShahariar/copr-hub.git
cd copr-hub
sh scripts/install-local.sh
```

The per-user installer builds the release binary and installs the launcher under `~/.local`; it does not require root privileges. To uninstall the launcher and application metadata:

```bash
sh scripts/uninstall-local.sh
```

Your saved projects and activity history are preserved when uninstalling.

### Install a prebuilt release

Tagged releases provide an x86_64 Linux archive and a SHA-256 checksum. Download both from [GitHub Releases](https://github.com/SakibShahariar/copr-hub/releases), then verify and extract the archive:

```bash
ARCHIVE="copr-hub-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz"
sha256sum -c "$ARCHIVE.sha256"
tar -xzf "$ARCHIVE"
```

Install the binary and desktop integration files:

```bash
install -Dm755 copr-hub/bin/copr-hub ~/.local/bin/copr-hub
install -Dm644 copr-hub/share/applications/io.github.SakibShahariar.CoprHub.desktop ~/.local/share/applications/io.github.SakibShahariar.CoprHub.desktop
install -Dm644 copr-hub/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml ~/.local/share/metainfo/io.github.SakibShahariar.CoprHub.metainfo.xml
install -Dm644 copr-hub/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg ~/.local/share/icons/hicolor/scalable/apps/io.github.SakibShahariar.CoprHub.svg
```

Replace `X.Y.Z` with the release version. The prebuilt binary uses the system GTK4 and libadwaita libraries; on Fedora, install them with:

```bash
sudo dnf install gtk4 libadwaita
```

## Build and test

To run the application from the source tree:

```bash
cargo run
```

Run the project's core checks:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo check
cargo clippy --all-targets --all-features -- -D warnings
```

## Local data and privacy

Saved projects, activity history, monitoring state, project snapshots, and release/search caches are stored in your user account's XDG data, config, and cache directories. They are not uploaded, and Copr Hub has no telemetry feature. JSON exports are ordinary files, so review their contents before sharing.

See [Local data and privacy](docs/FEATURES.md#local-data-and-privacy) for exact file paths and storage details.

## Project

- **Application ID:** `io.github.SakibShahariar.CoprHub`
- **License:** [MIT](LICENSE)
- **Detailed documentation:** [Features, offline behavior, security, and limitations](docs/FEATURES.md)
