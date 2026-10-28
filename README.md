# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting project metadata, and managing configured COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

## Current functionality

- Search the public COPR API for projects by name.
- Display project names, owners, descriptions, and optional homepage links.
- Open project details with package names, build targets, reported build states, and package versions; inspect every build target for a package in a dedicated details window.
- Save and unsave projects locally, then revisit them from the Saved tab.
- Read enabled and disabled COPR repository definitions from `/etc/yum.repos.d`, including DNF5 repository enablement overrides.
- Enable or disable a COPR repository after explicit confirmation, using `pkexec` and DNF's config-manager rather than running the application as root.
- List installed packages whose recorded source repository is a COPR repository, using DNF's read-only `repoquery` command.
- Install packages from project details and remove installed COPR packages after explicit confirmation. DNF runs through `pkexec`; the app itself never runs as root.
- Record recent project searches, saved-project changes, package operations, and successful repository changes in a local activity history capped at 100 entries, with local timestamps shown in the Activity tab.
- Show feedback for empty results, timeouts, and network/API failures.
- Run blocking network, filesystem, and package-manager operations away from the GTK main thread.
- Use native GTK/libadwaita widgets and the active system theme. Accent colors are not hard-coded, allowing Matugen palette changes to flow through the existing GTK theme.
- Include a desktop entry, SVG icon, AppStream metadata, and per-user install/uninstall scripts.

COPR build status is informational and is **not** a security or trust assessment. Package names exposed by a COPR project may not always correspond to installable binary RPM names; DNF reports resolution and transaction failures to the app.

## Build and run on Fedora

Install native development dependencies:

```bash
sudo dnf install gcc pkgconf-pkg-config gtk4-devel libadwaita-devel
```

Install Rust using [rustup](https://rustup.rs/), then run:

```bash
cargo fmt --all -- --check
cargo test
cargo check
cargo run
```

For a strict lint pass:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

## Install for the current user

From the repository checkout:

```bash
sh scripts/install-local.sh
```

This builds the release binary and installs it under `~/.local`; it does not require root privileges. To uninstall the launcher and application metadata:

```bash
sh scripts/uninstall-local.sh
```

Uninstalling preserves local saved projects and activity history.

## Local data

- Activity history: `$XDG_DATA_HOME/copr-hub/activity.json` (usually `~/.local/share/copr-hub/activity.json`).
- Saved projects: `$XDG_DATA_HOME/copr-hub/favorites.json`.

These files remain local to the current user account and are not uploaded.

## Project identity

- Display name: **Copr Hub**
- Application ID: `io.github.SakibShahariar.CoprHub`
- Repository: <https://github.com/SakibShahariar/copr-hub>

## License

Copr Hub is licensed under the MIT License. See [LICENSE](LICENSE).
