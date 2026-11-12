# Perplexity Audit Triage

Scope: the 99 audit issues in #50–#148. This file records the A–E category for every currently open audit issue. Categories describe implementation and verification readiness, not severity.

- **A — Fully actionable:** can be implemented and verified with source-level tests and CI.
- **B — Implementation ready, validation pending:** final acceptance needs a live GNOME/UI or target-system check.
- **C — Environment-dependent:** reliable implementation or verification needs a specific runtime/system interaction.
- **D — External dependency:** blocked by an upstream API contract or third-party capability.
- **E — Owner decision:** requires a product-policy choice, clarification, or confirmation that a finding should be closed as not planned.

## Open issues

| Issue | Category | Evidence / next step |
|---:|:---:|---|
| #50 | B | Repository action controls need duplicate-operation protection and live GNOME validation. |
| #51 | B | Repeated repository-removal confirmation/operations require UI lifecycle verification. |
| #52 | B | Disable package-row actions during privileged operations; verify DNF completion behavior. |
| #53 | B | Move favorites mutation off the GTK thread and verify Project Details feedback. |
| #54 | B | Prevent duplicate dependency windows and verify modal/focus behavior. |
| #55 | B | Guard compatibility lookups and verify control restoration on every path. |
| #56 | A | Shared monitoring guard and deterministic concurrency tests; implementation in progress. |
| #57 | A | Serialize scheduled backup checks and test concurrent invocations; implementation in progress. |
| #58 | B | Disable Import during chooser/confirmation/import and verify cancellation paths. |
| #59 | B | Disable Import/Export during chooser and file operations; verify native chooser behavior. |
| #60 | B | Replace label-text parsing with explicit loading state; verify repository action gating. |
| #61 | B | Move favorites file I/O to a worker and verify UI responsiveness and save feedback. |
| #62 | B | Move activity import/export/clear I/O off the GTK thread and verify refresh behavior. |
| #63 | B | Duplicate scope of #62; track clear-history behavior with #62 and verify its response path. |
| #64 | B | Separate user-facing errors from diagnostics; verify visible copy on each page. |
| #65 | A | Report alert count rather than project count; deterministic summary test. |
| #66 | B | Add scan progress and verify it remains responsive during a live scan. |
| #67 | B | Replace debug-formatted manual-monitor errors with user-facing guidance. |
| #68 | B | Include a concise scheduled-backup failure reason; verify desktop notification content. |
| #69 | A | Log legacy-cache cleanup failures while treating a missing cache as normal. |
| #70 | B | Separate search success from activity-recording failure feedback; verify Discover presentation. |
| #71 | E | Decide which metadata fields local Discover filtering should include. |
| #72 | B | Refresh Project Details without losing scroll position; requires live UI verification. |
| #73 | E | Decide whether a broad UI-module refactor is worth its churn. |
| #75 | C | Transaction preview/confirmation must be tested against real Fedora DNF behavior. |
| #76 | C | Missing-cache recovery depends on actual DNF metadata and offline/online behavior. |
| #77 | E | Decide how much raw DNF/pkexec output should be shown versus retained for diagnostics. |
| #82 | B | Clarify that DNF reports recorded install origin, not necessarily the current provider. |
| #83 | B | Surface candidate version and repository in Installed rows; verify layout. |
| #84 | B | Distinguish exact 100-item results from truncated output and show the limit in dependency details. |
| #85 | A | Timeout handling can be tested with controlled child processes. |
| #86 | A | Explicit command environments can be tested with fake executables. |
| #87 | C | Safe cancellation needs process lifecycle handling and interactive validation. |
| #89 | B | Preserve skipped-file details and surface a nonfatal repository-scan warning. |
| #90 | A | Fake DNF/RPM executables or an injected runner can cover command paths in CI. |
| #91 | B | COPR pagination exists, but a useful Load more flow needs UI state and interaction testing. |
| #92 | E | Decide whether one bounded retry is worth an additional request during outages. |
| #93 | A | Respect bounded Retry-After values and skip retry when the server asks for a longer delay. |
| #94 | E | thread::sleep is consistent with the current blocking HTTP client; decide whether async migration is desired. |
| #95 | E | Decide the trade-off between monitoring detail/freshness and COPR request volume. |
| #96 | A | Replace message-substring classification with typed error kinds and unit tests. |
| #99 | B | Clarify compatibility wording so one Fedora target is not read as blanket package support. |
| #100 | B | Include architecture in compatibility interpretation and verify unknown/partial states. |
| #101 | D | Complete build-state aliases depend on COPR's documented/observed status vocabulary. |
| #103 | E | sort_by is stable; the claimed instability is unsupported by the current implementation. |
| #104 | B | Expose omitted build-target counts or provide a scrollable/paged summary. |
| #105 | E | Optional GitHub token support needs approval for credential storage, scopes, and auth UX. |
| #106 | A | Apply the same bounded Retry-After policy to upstream release requests. |
| #107 | E | Blocking sleep is consistent with reqwest::blocking; decide whether async migration is desired. |
| #108 | E | Current upstream cache keys already include the normalized host; confirm any additional distinction needed. |
| #109 | E | LRU versus insertion-time eviction is a cache-policy choice; obtain usage evidence first. |
| #110 | E | Choose negative-result cache TTL and whether explicit refresh bypasses it. |
| #111 | A | Make persistent-cache load failures diagnosable with malformed/oversized fixtures. |
| #112 | D | GitLab's release API has no dedicated prerelease boolean; revisit if upstream adds one. |
| #113 | B | Tighten the GitHub release label to describe stable-release semantics. |
| #114 | B | Explain why version comparison is unavailable; verify Project Details presentation. |
| #115 | E | Decide whether to add an RPM EVR parser dependency or retain narrow comparison support. |
| #116 | E | Caller already slices ISO timestamps to YYYY-MM-DD; specify any other required date formats. |
| #117 | E | Decide whether the 365-day stale-release threshold should be configurable. |
| #119 | B | Distinguish missing from corrupt monitoring state, preserve the file, and show a nonfatal warning. |
| #120 | B | Separate alert delivery from persistence carefully and expose a persistent warning. |
| #121 | E | Decide whether removed-project baselines should be retained, and for how long. |
| #122 | E | Choose stale-alert cooldown/hysteresis behavior. |
| #123 | E | Decide whether stale cached release data may advance monitoring baselines or trigger alerts. |
| #124 | A | Leading v/V prefixes are normalized; keep suffixes distinct unless a different release-equivalence policy is chosen. CI is pending. |
| #125 | E | Current code suppresses the latest-build alert when new failed targets already explain the failure. |
| #126 | E | Activity changes are recorded only after monitoring state saves successfully; the described contradictory-history path is absent. |
| #127 | B | Distinguish oversized monitoring state from missing state, preserve the file, and surface recovery guidance. |
| #128 | B | Preserve corrupt monitoring settings instead of silently replacing them with defaults. |
| #129 | E | Decide whether config and state need transactional coordination or documented recovery is sufficient. |
| #130 | E | Current settings copy says the threshold applies to new observations; decide whether immediate re-evaluation is desired. |
| #132 | B | Pass the validated import snapshot through confirmation so preview counts cannot drift. |
| #133 | E | Decide whether import should update metadata for existing saved projects. |
| #134 | E | Duplicate removal is intentional cleanup; decide whether UI wording or preservation is preferred. |
| #135 | E | Decide whether an extra in-app overwrite confirmation is needed beyond native save-dialog behavior. |
| #137 | A | Sanitize control/invisible-format characters in descriptions at import/API boundaries and test fixtures. |
| #138 | E | Decide whether the smaller import limit is a deliberate safety bound. |
| #140 | E | Profile favorites lookup before adding cache/invalidation complexity. |
| #141 | B | Preserve corrupt favorites data and require explicit recovery consent; verify Saved-page flow. |
| #142 | B | Separate concise user-facing errors from technical diagnostics. |
| #143 | E | atomic_write returns the underlying I/O error and callers include it; decide whether presentation needs changing. |
| #146 | B | Preserve corrupt activity history and offer recovery without silently discarding user data. |

## Previously closed audit findings

These closed findings were checked against their acceptance lists: #74, #78, #79, #80, #81, #88, #97, #98, #102, #118, #131, #136, #139, #144, #145, #147, and #148. Their recorded criteria are checked and the relevant CI runs are green, except #124 which remains open pending CI.

## Work status

- #79–#81 and #88: implementation and tests are on `master`; the combined CI run is green and their acceptance lists are checked.
- #136 and #139: validation and tests are on `master`; the formatting/test CI run is green and their acceptance lists are checked.
- #124: leading v/V normalization is implemented; CI is still running. Suffixes remain distinct.
- #56, #57, #65, and #69: implementation and deterministic tests are on `master`; CI verification is pending.
- Remaining Category A issues stay open until their focused implementation, tests, and CI verification are complete.
