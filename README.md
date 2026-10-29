# Copr Hub

Copr Hub is a native Linux desktop application for discovering Fedora COPR projects, inspecting project metadata, and managing configured COPR repositories. It is built with **Rust, GTK4, and libadwaita**, following GNOME conventions.

## Current functionality

- Search the public COPR API for projects by name, sort current results by project name or owner, and filter displayed results locally by project name, owner, or description without repeating the network request.
- Display project names, owners, descriptions, and optional homepage links.
- When a COPR project has a recognized GitHub or GitLab repository as its homepage, look up the latest stable upstream release and show its release tag/version, title, publication date, and release link in project details. Successful release lookups are cached for 15 minutes; if a refresh fails, cached release metadata up to 24 hours old can still be shown with an explicit stale-cache notice. A stable release with a known publication date older than 365 days is marked as over one year old; this is a freshness hint, not proof that a project is abandoned. A package is flagged as “Upstream newer” only when its name matches the repository slug and both upstream and RPM versions use supported numeric syntax; ambiguous versions are left unclassified. Unrecognized or missing upstream URLs are reported as unavailable rather than guessed.
- Open project details with package names, build targets, reported build states, and package versions; inspect every build target for a package in a dedicated details window. A build-health summary groups succeeded, failed, in-progress, and unknown states by Fedora target (up to five targets), helping reveal architecture/release-specific failures without treating build state as a security assessment. Project details also show the latest project-build age and state when COPR provides timestamps, flagging builds at least 90 days old as “No recent project build.” Missing timestamps are shown as unavailable rather than guessed. Project details also include an update-intelligence summary counting packages whose matched upstream release is newer than the reported package version, failed and in-progress build targets, and stale upstream/build signals; comparisons using cached release metadata are explicitly marked as potentially out of date, and unsupported or missing version metadata is left unclassified. The package list can be filtered to all packages, packages with newer upstream releases, failed builds, or in-progress builds; its local search also matches package versions, target names, and reported build states.
- Save and unsave projects locally, filter saved projects by name, owner, or description, and revisit them from the Saved tab; Saved and Activity refresh automatically when opened. Saved metadata is validated and the on-disk list is capped at 10 MiB so oversized API data or imports cannot make the saved list unreadable. Export saved projects to a portable JSON backup and import backups by merging new entries without duplicating or replacing existing saved projects; imports that would exceed the storage cap are rejected without changing existing data.
- Read enabled and disabled COPR repository definitions from `/etc/yum.repos.d`, including DNF5 repository enablement overrides.
- Enable or disable a COPR repository after explicit confirmation, using `pkexec` and DNF's config-manager rather than running the application as root.
- List installed packages whose recorded source repository is a COPR repository, using DNF's read-only `repoquery` command; filter the list locally by package name, version, or repository.
- Install packages from project details and remove installed COPR packages after explicit confirmation. DNF runs through `pkexec`; the app itself never runs as root. Package removals use DNF's cache-only mode to avoid refreshing remote repositories, but can still fail offline if cached metadata or packages are insufficient.
- Record recent project searches, saved-project changes, package operations, and successful repository changes in a local activity history capped at 100 entries, with local timestamps shown in the Activity tab; search the history locally, export it to portable JSON, import history backups by merging newer entries without duplicating existing events, and clear it explicitly after confirmation.
- Handle temporary COPR, GitHub, and GitLab API failures with a single bounded retry for rate limits, server errors, connection failures, and timeouts. GitHub `403` responses are retried only when rate-limit headers identify throttling; rate-limit/server responses can provide a `Retry-After` delay capped at three seconds, otherwise the app uses a short backoff. Existing search results stay visible while a refresh is running or fails, with feedback for empty results, missing projects, and other network/API errors.
- Open project homepage links only when COPR metadata provides an HTTP or HTTPS URL.
- Run blocking network, filesystem, and package-manager operations away from the GTK main thread.
- Use native GTK/libadwaita widgets and the active system theme. Accent colors are not hard-coded, allowing Matugen palette changes to flow through the existing GTK theme.
- Include a desktop entry, SVG icon, AppStream metadata, and per-user install/uninstall scripts.

COPR build status and freshness are informational and are **not** a security or trust assessment. The 90-day freshness threshold is a heuristic, not a claim that a project is abandoned or unsafe; timestamps that COPR does not provide are reported as unavailable. Package names exposed by a COPR project may not always correspond to installable binary RPM names; DNF reports resolution and transaction failures to the app.

## Offline behavior

Copr Hub can open without an internet connection, and its local features remain available:

- Saved projects and activity history are stored on the device and can be viewed, edited, exported, or imported offline.
- The Repositories tab reads local DNF repository configuration files.
- The Installed tab queries the local DNF database; listing installed packages does not itself require downloading repository metadata.
- Search results already displayed remain visible when a search or refresh fails during the current app session.

Live COPR searches, fresh project build details, upstream release lookups, and package operations that need remote repositories require connectivity or usable local DNF metadata/packages. The current search-result list is not persisted between launches, and upstream release caching is in memory only; release metadata cached during a session can be used as a stale fallback for up to 24 hours if a refresh fails, but it is not available after restarting the app. Cached metadata is identified as potentially out of date. Offline support is therefore partial rather than a full offline mirror of COPR.

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

These files remain local to the current user account and are not uploaded. The app writes them atomically with owner-only (`0600`) file permissions, including when replacing an older file with broader permissions. It also syncs the containing directory after replacement to improve persistence across abrupt shutdowns.

## Project identity

- Display name: **Copr Hub**
- Application ID: `io.github.SakibShahariar.CoprHub`
- Repository: <https://github.com/SakibShahariar/copr-hub>

## License

Copr Hub is licensed under the MIT License. See [LICENSE](LICENSE).
