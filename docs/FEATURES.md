# Features and behavior

This document describes Copr Hub's detailed functionality. The [README](../README.md) is intentionally kept short as the project landing page.

## Discover projects

- Search the public COPR API by project name.
- Sort results by project name or owner and filter locally by name, owner, or description without repeating the request.
- Keep existing results visible while a refresh runs or fails, with feedback for empty results, missing projects, and API/network errors.
- Show project names, owners, descriptions, and optional homepage links. Open homepage links only when COPR metadata provides an HTTP or HTTPS URL.
- Run blocking network, filesystem, and package-manager work away from the GTK main thread.

## Project and package intelligence

- Inspect package names, build targets, reported build states, and package versions; open a dedicated details window to inspect every target for a package.
- Summarize succeeded, failed, in-progress, and unknown build states by Fedora target (up to five targets).
- Show a package-by-target build matrix with reported target state and package version; missing package/target combinations are marked explicitly instead of being treated as failures.
- Show a chronological timeline of up to 20 recent project build records, including package/version, reported state, submission/start/end timestamp when available, configured chroots, and a direct COPR build link.
- The timeline uses actual records returned by COPR. COPR may prune older build records, so the timeline is a recent history view, not a complete lifetime audit log; unavailable timestamps are not guessed.
- Show latest project-build age and state when COPR provides timestamps. Builds at least 90 days old receive a “No recent project build” hint; missing timestamps remain unavailable.
- Recognize GitHub or GitLab repositories provided as project homepages and look up their latest stable upstream release, including version/tag, title, publication date, and link.
- Cache successful upstream release lookups for 15 minutes in memory and persist them locally. Cached metadata up to 24 hours old can be used after restart or when offline, with a stale-cache notice.
- Mark a known stable release older than 365 days as over one year old. This is a freshness hint, not proof of abandonment.
- Flag “Upstream newer” only when a package name matches the repository slug and both upstream and RPM versions use supported numeric syntax. Unsupported or ambiguous versions remain unclassified.
- Show an update-intelligence summary of newer upstream releases, failed/in-progress build targets, and stale upstream/build signals. Cached comparisons are marked as potentially out of date.
- Filter packages by all packages, newer upstream releases, failed builds, or in-progress builds. Local package search also matches versions, target names, and build states.
- A build-health summary is informational, not a security or trust assessment.

## Saved projects and health checks

- Save and unsave projects locally; filter saved projects by name, owner, or description.
- Open a saved project's details or run an on-demand health check against current COPR build data.
- Optionally monitor saved projects while the app is open. Checks run at startup (after a short delay) and then every six hours; there is no always-running daemon.
- Send desktop notifications only when a later check detects a newly failed target/build, a changed stable upstream release, or a project crossing the 90-day no-build threshold. The first successful check establishes a baseline and does not notify about pre-existing conditions.
- Monitoring is disabled by default and can be enabled or disabled with the checkbox on the Saved projects page. Checks run off the GTK main thread and stop after a connection failure rather than timing out on every remaining project.
- Store monitoring comparison state locally so unchanged conditions do not repeatedly notify. Notification delivery depends on the desktop notification service.
- Health checks summarize package count, build freshness, target states, and failed targets. Each check is recorded in local activity history.
- Saved and Activity views refresh when opened.
- Validate saved metadata and cap the saved-project file at 10 MiB to help prevent oversized data from making the list unreadable.
- Export saved projects to portable JSON. Import backups by merging new entries without duplicates or replacing existing saved projects; reject imports that exceed the storage cap without changing existing data.

## Compare saved projects

- Select two or three saved COPR projects and compare their live package counts, succeeded/failed/in-progress/unknown build-target totals, and latest-build freshness side by side.
- Each project is fetched independently; a failed lookup is labeled unavailable instead of preventing other projects from being compared.
- Comparison data is fetched on demand and is not presented as a security or trust ranking.

## Manage repositories and packages

- Read enabled and disabled COPR repository definitions from `/etc/yum.repos.d`, including DNF5 repository enablement overrides.
- Enable or disable a repository after explicit confirmation, using `pkexec` and DNF's config-manager rather than running the app as root.
- Remove supported standard Fedora COPR repositories after separate confirmation. This removes repository configuration only; it does not uninstall packages installed from that source. Unrecognized repository IDs are not offered the removal action.
- List installed packages whose recorded source is a COPR repository using DNF's read-only `repoquery` command.
- Install packages from project details and remove installed COPR packages after explicit confirmation. DNF runs through `pkexec`; the application itself never runs as root.
- Package removal uses DNF cache-only mode to avoid refreshing remote repositories, but can fail offline if cached metadata or packages are insufficient.

## Activity history

- Record recent project searches, saved-project changes, package operations, and successful repository changes in a local history capped at 100 entries.
- Search activity locally, view local timestamps, export history to portable JSON, merge history backups without duplicating existing events, or clear history after confirmation.

## Network and offline behavior

- Handle temporary COPR, GitHub, and GitLab API failures with one bounded retry for rate limits, server errors, connection failures, and timeouts.
- Retry GitHub `403` responses only when rate-limit headers indicate throttling. Rate-limit/server responses can use a `Retry-After` delay capped at three seconds; otherwise a short backoff is used.
- Saved projects, activity history, repository configuration, and installed-package listings remain available offline to the extent they rely on local data.
- Persist per-project snapshots of the last successful package/target monitor response, recent build history (up to 20 records), and upstream release metadata. The project details view shows cached data immediately, refreshes it in the background, and labels cached timestamps and snapshots older than six hours as stale. Snapshots are retained for up to 180 days and limited to 4 MiB per project.
- When a live COPR or upstream request fails, use the last valid project snapshot when available rather than replacing it with an empty result. Cached data is informational and may no longer match the live project.
- Optional monitoring also refreshes project snapshots and recent build history on its six-hour cycle, so previously saved projects can be useful offline even if their details page has not been opened recently.
- Persist the last successful search query and up to 25 results in `$XDG_CACHE_HOME/copr-hub/last-search.json` (usually `~/.cache/copr-hub/last-search.json`) and restore them on startup for up to 30 days. Restored results are labeled as potentially stale; live searches still require connectivity. Only the most recent successful search is cached.
- Live COPR searches, fresh build details, upstream release lookups, and package operations requiring remote repositories need connectivity or usable local DNF metadata/packages.
- Upstream release cache files are size-limited and entries older than 24 hours are discarded. Cached information is explicitly labeled as potentially stale.

## Appearance and packaging

- Native GTK4/libadwaita widgets use the active system theme; accent colors are not hard-coded, so Matugen palette changes can flow through the existing GTK theme.
- Includes a desktop entry, SVG icon, AppStream metadata, and per-user install/uninstall scripts.

## Local data and privacy

Copr Hub keeps its local data under the current user's XDG directories and does not upload these files:

- Activity history: `$XDG_DATA_HOME/copr-hub/activity.json` (usually `~/.local/share/copr-hub/activity.json`).
- Saved projects: `$XDG_DATA_HOME/copr-hub/favorites.json`.
- Search results cache: `$XDG_CACHE_HOME/copr-hub/last-search.json` (usually `~/.cache/copr-hub/last-search.json`).
- Upstream release cache: `$XDG_CACHE_HOME/copr-hub/upstream-releases.json` (usually `~/.cache/copr-hub/upstream-releases.json`).
- Project snapshots: `$XDG_CACHE_HOME/copr-hub/projects/` (hashed project filenames; cached build/target data and timestamps).
- Monitoring preference: `$XDG_CONFIG_HOME/copr-hub/monitoring.json`.
- Monitoring change baseline: `$XDG_DATA_HOME/copr-hub/monitoring-state.json`.

Files are written atomically with owner-only (`0600`) permissions, including when replacing an older file with broader permissions. The containing directory is synced after replacement to improve persistence across abrupt shutdowns.

## Limitations to keep in mind

- Build and freshness indicators are heuristics, not security assessments. A 90-day-old build does not prove that a project is abandoned or unsafe.
- COPR may omit timestamps. When timestamps are missing, the app reports them as unavailable rather than guessing.
- Project package names do not always match installable binary RPM names; DNF reports package-resolution and transaction failures.
- Offline support is partial, not a full local mirror of COPR. Live searches, refreshes, repository changes, and package operations that require network metadata remain unavailable offline.
