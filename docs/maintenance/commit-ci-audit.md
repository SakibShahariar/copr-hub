# Commit CI Audit Ledger

Persistent repository-specific record of commit-level CI checks and red-cross investigations.

## How to use this ledger

1. Read this file before auditing commit checks.
2. Check only commits newer than the checkpoint, plus older **red**, **pending**, or explicitly unresolved entries.
3. Record exact commit SHAs and observed results; never infer from missing badges or empty API responses.
4. Preserve failures and append diagnosis, fix, rerun, and final-outcome evidence.
5. Advance the checkpoint only after reviewing the range; carry unresolved entries forward.

## Result meanings

| Result | Meaning |
|---|---|
| **green** | Completed successful check/run directly observed. |
| **red** | Completed failing check/run directly observed. |
| **pending** | Check/run exists but is unfinished. |
| **unavailable** | Available evidence did not expose a result; not a pass or failure. |
| **not applicable** | Workflow configuration confirms no relevant check is triggered. |

## Checkpoint

- Repository: `SakibShahariar/copr-hub`
- Default branch: `master`
- Checkpoint SHA: `001900350150fcc37746701597566511f30cb074`
- Initialized: 2026-11-02
- Baseline: latest branch tip queried through combined commit-status endpoint; empty status list recorded as **unavailable**, not green/red. Older history was not retroactively audited during setup.
- Next session: review commits after `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d`, including the next substantive commit that updates this ledger.

## Initial baseline

| Commit SHA | Subject | CI result | Checked | Follow-up |
|---|---|---|---|---|
| `cf6d142db537f0bf13a2e098015cbbabad90285d` | Default-branch tip at ledger initialization | unavailable | 2026-11-02 | Combined status lookup returned no entries. Revisit if new run or visible failure evidence appears. |

## Reviewed commits since the previous checkpoint

| Commit SHA | Subject | CI | Date | Workflow run(s) |
|---|---|---|---|---|
| `40d4289dd22ed43c1ee932bc7a03a2104782b4a7` | docs: add persistent commit CI audit ledger | **green** | 2026-11-02 | [41456599107](https://github.com/SakibShahariar/copr-hub/actions/runs/41456599107) |
| `f0a1e3dde5315adb32a0dd1d90d42b460e85605c` | docs: document incremental commit CI auditing | **green** | 2026-11-02 | [41456605056](https://github.com/SakibShahariar/copr-hub/actions/runs/41456605056) |
| `da33d1c9f282ae2bd253176cdf7123a77ff8f1a9` | docs: define issue triage categories | **green** | 2026-11-03 | [41573404861](https://github.com/SakibShahariar/copr-hub/actions/runs/41573404861) |
| `e14ae4be0182c07eebba447497810b4e043f7825` | docs(agents): define administrative commit hygiene | **green** | 2026-11-03 | [41598051449](https://github.com/SakibShahariar/copr-hub/actions/runs/41598051449) |
| `356a1e0e6d11063bfb1a46fbd6fa94715741de2e` | docs(agents): clarify acceptance criteria evidence | **green** | 2026-11-04 | [41864350051](https://github.com/SakibShahariar/copr-hub/actions/runs/41864350051), [42455174461](https://github.com/SakibShahariar/copr-hub/actions/runs/42455174461), [42455174462](https://github.com/SakibShahariar/copr-hub/actions/runs/42455174462) |
| `d66b0bd58abbe1de6a2b7a777f0d694094cc7e2c` | docs: improve Copr Hub README | **green** | 2026-11-08 | [42510920191](https://github.com/SakibShahariar/copr-hub/actions/runs/42510920191) |
| `8639df2e511a35ae6d039cf8a6fa8d2c9c5d51c3` | docs: refine Copr Hub README length | **green** | 2026-11-08 | [42510954764](https://github.com/SakibShahariar/copr-hub/actions/runs/42510954764) |
| `f0834cdc48fcc7be28bff6145855f07190ae0fc1` | docs: streamline Copr Hub feature overview | **green** | 2026-11-08 | [42511490180](https://github.com/SakibShahariar/copr-hub/actions/runs/42511490180) |
| `53568e1df2f21cc88b8cf46b2f09a6d77b94faad` | ui: constrain page content for a cleaner GNOME layout | **green** | 2026-11-12 | [43228309963](https://github.com/SakibShahariar/copr-hub/actions/runs/43228309963) |
| `f019e083e03cd36f1d9ecbbb346d8de0942dba60` | ui: use readable sans-serif typography in the app | **red** | 2026-11-12 | [43228400051](https://github.com/SakibShahariar/copr-hub/actions/runs/43228400051) |
| `b55d350d7179d89f7a28fd46f39c33f8949b368e` | ui(saved): group monitoring and backup preferences | **red** | 2026-11-12 | [43228560507](https://github.com/SakibShahariar/copr-hub/actions/runs/43228560507) |
| `8162394345e315668400b9ed91d969dab9c76cc1` | fix(ui): disambiguate GTK display accessor | **red** | 2026-11-12 | [43228650122](https://github.com/SakibShahariar/copr-hub/actions/runs/43228650122) |
| `e31cf6182ded5bfbbb2b9f7ba98bd5435361828a` | style(saved): format preferences row | **red** | 2026-11-12 | [43228730416](https://github.com/SakibShahariar/copr-hub/actions/runs/43228730416) |
| `1eb12fe1eab4a251391f7fb52f629303f61b3f20` | ui: use adaptive Libadwaita view switcher and toolbar | **red** | 2026-11-12 | [43228826142](https://github.com/SakibShahariar/copr-hub/actions/runs/43228826142) |
| `7c59c47ab4d020e8af898097eae3373f283741d2` | fix(saved): use supported Adw action rows for switches | **red** | 2026-11-12 | [43229052048](https://github.com/SakibShahariar/copr-hub/actions/runs/43229052048) |
| `08ec865137c1d34e552f35a37f06a43899c3cc90` | fix(ui): retain compatible GTK4 application shell | **red** | 2026-11-12 | [43229061450](https://github.com/SakibShahariar/copr-hub/actions/runs/43229061450) |
| `b438e3940e9fc5c7ed3b6ef725c11db67d06582f` | fix(saved): place monitoring status below its preference group | **red** | 2026-11-12 | [43229156662](https://github.com/SakibShahariar/copr-hub/actions/runs/43229156662) |
| `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a` | style(saved): format preference row | **green** | 2026-11-12 | [43229315027](https://github.com/SakibShahariar/copr-hub/actions/runs/43229315027) |
| `4eaa0ed930ebbba24d1160fee28ec34a3aedb697` | ui(saved): add a focused empty state and trim redundant copy | **red** | 2026-11-12 | [43229525173](https://github.com/SakibShahariar/copr-hub/actions/runs/43229525173) |
| `e01b77ab5ac4c598bf9635fb94fcf8f3d6dc7675` | fix(saved): format and wrap the empty-state page | **green** | 2026-11-12 | [43229747307](https://github.com/SakibShahariar/copr-hub/actions/runs/43229747307) |
| `9780d267753c2b6a0fe78904aab2485f50a6030c` | feat(installed): add update filter chips | **green** | 2026-11-12 | [43231290653](https://github.com/SakibShahariar/copr-hub/actions/runs/43231290653) |
| `47bcc14867b3ecbc2ee231c7c79912544c0252dd` | style(repositories): group row actions in a popover | **red** | 2026-11-12 | [43231503966](https://github.com/SakibShahariar/copr-hub/actions/runs/43231503966) |
| `fb2d289687234ed015eaef37166ce8d24828f9c3` | fix(repositories): format action popover layout | **green** | 2026-11-12 | [43231693602](https://github.com/SakibShahariar/copr-hub/actions/runs/43231693602) |
| `0ed984a7d17b6c1a3f337cf4a445adf95f4ffc9e` | style(details): modernize project header and actions | **green** | 2026-11-12 | [43231799291](https://github.com/SakibShahariar/copr-hub/actions/runs/43231799291) |
| `ff645c3fab9642bfbe59536d86368697f53b4aa0` | style(repositories): group enabled and disabled entries | **green** | 2026-11-12 | [43231951835](https://github.com/SakibShahariar/copr-hub/actions/runs/43231951835) |
| `02af15228eceb7e08afae8050b842032fc227455` | style(discover): use a compact project search header | **green** | 2026-11-12 | [43232017116](https://github.com/SakibShahariar/copr-hub/actions/runs/43232017116) |
| `3518b38503bfb606a7c1872d5b4bcff0a643a6df` | style(saved): group bulk actions in a menu | **green** | 2026-11-12 | [43232145573](https://github.com/SakibShahariar/copr-hub/actions/runs/43232145573) |
| `6786e2b9a50979ca4961ed3b05bfda39bb58656c` | feat(details): add refresh action and last-checked status | **red** | 2026-11-12 | [43232500254](https://github.com/SakibShahariar/copr-hub/actions/runs/43232500254) |
| `d5a4cd06376cb10b3a96d352c6bd4401f3033d71` | style(installed): highlight available package updates | **red** | 2026-11-12 | [43232579432](https://github.com/SakibShahariar/copr-hub/actions/runs/43232579432) |
| `3162168d2c8d02fa33fb57a98162b0ebfeca5326` | fix(details): format refresh action | **red** | 2026-11-12 | [43232992297](https://github.com/SakibShahariar/copr-hub/actions/runs/43232992297) |
| `2a96076780ddc8b4802832c03387daee95482e94` | fix(details): preserve URL validation return type | **green** | 2026-11-12 | [43233244021](https://github.com/SakibShahariar/copr-hub/actions/runs/43233244021) |
| `e3a65b60e15c24f57a3f21da63391c3fc85e534b` | feat(installed): surface update status and audit CI history | **green** | 2026-11-12 | [43233834076](https://github.com/SakibShahariar/copr-hub/actions/runs/43233834076) |
| `b9020080e0764ed5856bc154ba07744dc5b442dc` | fix(search): use COPR full-text project search | **green** | 2026-11-12 | [43235018143](https://github.com/SakibShahariar/copr-hub/actions/runs/43235018143) |
| `420c45ce7059905ee780ef7845234e69a35dbf9b` | fix(search): deserialize paginated COPR results | **green** | 2026-11-12 | [43235761345](https://github.com/SakibShahariar/copr-hub/actions/runs/43235761345) |
| `33f9d69f0dfb3f7df341dee59f8814276e5cc05b` | feat(search): rank COPR results by relevance | **red** | 2026-11-12 | [43238490566](https://github.com/SakibShahariar/copr-hub/actions/runs/43238490566) |
| `7025808bc620933d7af3b77647f6e024524284dd` | fix(search): keep fuzzy ranking portable and formatted | **red** | 2026-11-12 | [43238694601](https://github.com/SakibShahariar/copr-hub/actions/runs/43238694601) |
| `4d9bde02883b2722ccb1d9b5695722abae4bfc42` | style(search): apply rustfmt to relevance selector | **red** | 2026-11-12 | [43238811094](https://github.com/SakibShahariar/copr-hub/actions/runs/43238811094) |
| `c8158cbce2cc3d5b2aee546470e1847fad595da6` | fix(search): preserve unknown COPR match ordering | **red** | 2026-11-12 | [43238987455](https://github.com/SakibShahariar/copr-hub/actions/runs/43238987455) |
| `dea90eab197986c187ab1f97b3f03b2576bd4c71` | style(search): format edit distance calculation | **red** | 2026-11-12 | [43239093538](https://github.com/SakibShahariar/copr-hub/actions/runs/43239093538) |
| `eac293aacc18a9af60a298245dcf619e7bfc3674` | test(search): cover short-query fuzzy threshold | **red** | 2026-11-12 | [43239215341](https://github.com/SakibShahariar/copr-hub/actions/runs/43239215341) |
| `3f43212386c5e1d14b80d653fcef149090982221` | test(search): compare equally ranked project names | **green** | 2026-11-12 | [43239361770](https://github.com/SakibShahariar/copr-hub/actions/runs/43239361770) |
| `0f2aff926d2b7e455b97097b2d1b1f99303dc4a2` | docs(search): describe relevance ranking and audit CI | **green** | 2026-11-12 | [43239871757](https://github.com/SakibShahariar/copr-hub/actions/runs/43239871757) |
| `e7500338ca2107aa9ea027b9ace4628d9c3d639a` | fix(search): keep fuzzy matches behind explicit COPR hits | **red** | 2026-11-12 | [43240371763](https://github.com/SakibShahariar/copr-hub/actions/runs/43240371763) |
| `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d` | test(search): assert fuzzy tier without outranking explicit hits | **green** | 2026-11-12 | [43240592812](https://github.com/SakibShahariar/copr-hub/actions/runs/43240592812) |
| `dfcab23347a9b0338ccb9fc05a1cebb7a91d741a` | fix(ui): restore time imports and escape COPR markup | **green** | 2026-11-12 | [43255879522](https://github.com/SakibShahariar/copr-hub/actions/runs/43255879522) |
| `a53ecf247cf99405623383ca3e77486b4e6e41a1` | fix(repositories): show installed COPR package names | **red** | 2026-11-12 | [43258764703](https://github.com/SakibShahariar/copr-hub/actions/runs/43258764703) |
| `0e3df7c2bf431b01d8ca2d5e5159ca12d6bd7b1e` | fix(system): delimit DNF5 package query output | **red** | 2026-11-12 | [43258923528](https://github.com/SakibShahariar/copr-hub/actions/runs/43258923528) |
| `674d5dbe8b13842e653c618b2d2db978e5cc0997` | test(system): assert non-COPR origins are parsed | **green** | 2026-11-12 | [43258995937](https://github.com/SakibShahariar/copr-hub/actions/runs/43258995937) |
| `0fab3a5e1e070e8b9de0dd5397d741754402e956` | fix(ui): constrain repository details popover | **red** | 2026-11-12 | [43260961537](https://github.com/SakibShahariar/copr-hub/actions/runs/43260961537) |
| `1aede0427807d1a5af2843400d8d7ae728c7d433` | fix(ui): restore repository action menu interaction | **green** | 2026-11-12 | [43265148583](https://github.com/SakibShahariar/copr-hub/actions/runs/43265148583) |
| `9e494ccfb22cedb3b29902967ce99a8b3c4fc35` | fix(ui): use menu button popover lifecycle | **green** | 2026-11-12 | [43265577679](https://github.com/SakibShahariar/copr-hub/actions/runs/43265577679) |
| `5ba6ee70407fd7311590fc5750dcd52237fa18ca` | fix(ui): make repository menu keyboard-focusable | **green** | 2026-11-12 | [43267194014](https://github.com/SakibShahariar/copr-hub/actions/runs/43267194014) |
| `6943da45c62e766d2de3e19eac4d93fea5f5276f` | fix(installed): offer safe DNF5 permission repair | **red** | 2026-11-12 | [43271812234](https://github.com/SakibShahariar/copr-hub/actions/runs/43271812234) |
| `92b84bf6ff0489cc0579cca1cdba1a02d8cafb42` | fix(repositories): expose DNF permission repair where needed | **red** | 2026-11-12 | [43272140474](https://github.com/SakibShahariar/copr-hub/actions/runs/43272140474) |

## Red-cross investigation log

- `6943da45c62e766d2de3e19eac4d93fea5f5276f` — **Check formatting** rejected the fixed `chmod` argument list and long permission-candidate assertions in `src/system.rs`. These were corrected in `92b84bf6ff0489cc0579cca1cdba1a02d8cafb42`, but that commit still failed formatting on the metadata-helper signature; the final correction is in the current follow-up. Both runs skipped tests, compilation, and Clippy.

- `f019e083e03cd36f1d9ecbbb346d8de0942dba60` — **Run unit tests** failed because `window.display()` was ambiguous between GTK traits. The accessor was explicitly disambiguated in `8162394345e315668400b9ed91d969dab9c76cc1`; the later saved-page batch passed in `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a`.
- `b55d350d7179d89f7a28fd46f39c33f8949b368e`, `8162394345e315668400b9ed91d969dab9c76cc1`, `08ec865137c1d34e552f35a37f06a43899c3cc90`, and `b438e3940e9fc5c7ed3b6ef725c11db67d06582f` — **Check formatting** failed on successive formatting changes in `src/ui/saved_page.rs`. The resulting saved-page batch passed at `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a`.
- `e31cf6182ded5bfbbb2b9f7ba98bd5435361828a` — **Run unit tests** failed because the pinned libadwaita Rust bindings do not expose `adw::SwitchRow`. Replaced with `gtk::Switch` inside `adw::ActionRow`; subsequent compatibility work passed at `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a`.
- `1eb12fe1eab4a251391f7fb52f629303f61b3f20` — **Run unit tests** failed because `adw::SwitchRow` and `adw::ToolbarView` were unavailable in the pinned bindings. The switch rows and application shell were rewritten with supported widgets in `7c59c47ab4d020e8af898097eae3373f283741d2` and `08ec865137c1d34e552f35a37f06a43899c3cc90`; the final compatible UI batch passed at `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a`.
- `7c59c47ab4d020e8af898097eae3373f283741d2` — **Run unit tests** still failed because `adw::ToolbarView` was unavailable. Removed that unsupported shell API in `08ec865137c1d34e552f35a37f06a43899c3cc90`; later green validation at `a696c91b8aa8bf2a4f177b2ff9c1f79429e1df3a`.
- `4eaa0ed930ebbba24d1160fee28ec34a3aedb697` — **Check formatting** failed on the saved-project empty-state description. Formatting was corrected in `e01b77ab5ac4c598bf9635fb94fcf8f3d6dc7675`, whose CI passed.
- `47bcc14867b3ecbc2ee231c7c79912544c0252dd` — **Check formatting** failed on the repository-file label in the new actions popover. Formatting was corrected in `fb2d289687234ed015eaef37166ce8d24828f9c3`, whose CI passed.
- `6786e2b9a50979ca4961ed3b05bfda39bb58656c`, `d5a4cd06376cb10b3a96d352c6bd4401f3033d71`, and `3162168d2c8d02fa33fb57a98162b0ebfeca5326` — **Run unit tests** failed because a helper insertion accidentally removed the `bool` return type from `is_web_url`. Restored the signature in `2a96076780ddc8b4802832c03387daee95482e94`; its CI run passed.
- `33f9d69f0dfb3f7df341dee59f8814276e5cc05b` and `7025808bc620933d7af3b77647f6e024524284dd` — **Check formatting** rejected the multiline relevance dropdown expression. Applied the exact rustfmt layout in `4d9bde02883b2722ccb1d9b5695722abae4bfc42`.
- `4d9bde02883b2722ccb1d9b5695722abae4bfc42` and `c8158cbce2cc3d5b2aee546470e1847fad595da6` — **Check formatting** rejected the edit-distance substitution expression. Applied rustfmt's multiline layout in `dea90eab197986c187ab1f97b3f03b2576bd4c71`.
- `dea90eab197986c187ab1f97b3f03b2576bd4c71` and `eac293aacc18a9af60a298245dcf619e7bfc3674` — **Run unit tests** exposed two flawed test assumptions: different suffix lengths were not equal relevance scores, and the short-query fixture was actually a valid prefix. Corrected the short-query fixture in `eac293aacc18a9af60a298245dcf619e7bfc3674` and used equal-length suffixes in `3f43212386c5e1d14b80d653fcef149090982221`; the final run passed all 104 tests.
- `92b84bf6ff0489cc0579cca1cdba1a02d8cafb42` — **Check formatting** expected the DNF metadata helper signature on one line. Corrected in `001900350150fcc37746701597566511f30cb074`; that commit's CI result is pending/unavailable.
- `001900350150fcc37746701597566511f30cb074` — **Check compilation** failed: the package-removal callback in `src/ui/installed_page.rs` omitted the newly required fourth `repair_permissions` argument. Fixed in the current follow-up by threading the button through the installed-row renderer and refresh callback.
- `0fab3a5e1e070e8b9de0dd5397d741754402e956` — **Check formatting** failed because rustfmt expected the repository-file label construction on one line. The formatting was corrected in `1aede0427807d1a5af2843400d8d7ae728c7d433`; CI completed green at run `43265148583`. The later menu-lifecycle and keyboard-focus commits also completed green CI at runs `43265577679` and `43267194014`, although live GNOME activation remains unverified.
- `a53ecf247cf99405623383ca3e77486b4e6e41a1` and `0e3df7c2bf431b01d8ca2d5e5159ca12d6bd7b1e` — **Run unit tests** failed because a new parser test incorrectly expected non-COPR repository origins to be discarded. The parser intentionally records safe origins for all repositories; the repository UI looks up only the COPR ID it is displaying. Corrected the test to assert that the Fedora-origin row is parsed in `674d5dbe8b13842e653c618b2d2db978e5cc0997`; its CI passed all 106 tests and all remaining checks.
- `e7500338ca2107aa9ea027b9ace4628d9c3d639a` — **Run unit tests** found one stale test expectation after moving fuzzy matches below explicit description matches. Updated the test to assert the fuzzy tier directly in `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d`; its CI run passed all 104 tests.

All failures in this reviewed range have a subsequent fix and green CI validation. No pending or unavailable checks remain in the range.

## Session history

| Date | Reviewed range / checkpoint | Result summary |
|---|---|---|
| 2026-11-02 | Initial checkpoint: `cf6d142db537f0bf13a2e098015cbbabad90285d` | Latest commit queried; empty status response recorded as unavailable. Earlier history not retroactively audited. |
| 2026-11-12 | `cf6d142db537f0bf13a2e098015cbbabad90285d` → `2a96076780ddc8b4802832c03387daee95482e94` | Reviewed 31 commits: 18 green, 13 red; every red result was diagnosed and followed by a green fix/validation commit. |
| 2026-11-12 | `2a96076780ddc8b4802832c03387daee95482e94` → `420c45ce7059905ee780ef7845234e69a35dbf9b` | Reviewed 3 commits; all 3 had completed green CI runs. |
| 2026-11-12 | `420c45ce7059905ee780ef7845234e69a35dbf9b` → `3f43212386c5e1d14b80d653fcef149090982221` | Reviewed 7 commits: 1 green, 6 red; all six red results were diagnosed and followed by a green fix/validation commit. |
| 2026-11-12 | `3f43212386c5e1d14b80d653fcef149090982221` → `0f2aff926d2b7e455b97097b2d1b1f99303dc4a2` | Reviewed 1 documentation commit; CI completed green. |
| 2026-11-12 | `674d5dbe8b13842e653c618b2d2db978e5cc0997` → `5ba6ee70407fd7311590fc5750dcd52237fa18ca` | Reviewed 4 commits: 1 red and 3 green. The formatting failure was corrected by the next commit and all subsequent menu commits passed CI; live repository-menu activation remains unverified in a GNOME session. |
| 2026-11-12 | `5ba6ee70407fd7311590fc5750dcd52237fa18ca` → `92b84bf6ff0489cc0579cca1cdba1a02d8cafb42` | Reviewed 2 commits; both had formatting failures. The first failure was partially corrected in the second, which still had one rustfmt mismatch; a follow-up correction is pending CI. |
| 2026-11-12 | `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d` → `674d5dbe8b13842e653c618b2d2db978e5cc0997` | Reviewed 4 commits: 2 green, 2 red; both red runs were diagnosed as a flawed new test assertion and followed by a green correction. |
| 2026-11-12 | `0f2aff926d2b7e455b97097b2d1b1f99303dc4a2` → `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d` | Reviewed 2 commits: 1 red, 1 green; the red test expectation was corrected and final CI passed. |


## Follow-up CI review after `92b84bf`

This section records every direct-to-master commit after the previous ledger table checkpoint. Failed runs remain visible; subsequent green runs are recorded separately rather than erasing the failures.

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `001900350150fcc37746701597566511f30cb074` | style(system): apply rustfmt to DNF repair checks | red (#43272320693) |
| `5edc0d528dd0195462a2a0f25d733cbc171ce31d` | fix(installed): refresh after package removal | green (#43281811968) |
| `f8a44a86d1ce4ce5d77c27869bfba2d57091cf8c` | fix(system): repair all DNF5 state file permissions | red (#43284052569) |
| `c31c9f6b631b87e286036043945bf0c246ebba34` | fix(system): reuse safe state-file repair predicate | red (#43284181983) |
| `7b8d781fc71e8d6e982283f524ea0818801c83dc` | style(system): format DNF5 permission repair | green (#43284328026) |
| `5c1b8d2120d8dcb91cd22f7ba201a3f632e66d25` | fix(ui): show DNF permission repair for state-file errors | red (#43286341476) |
| `cffeecfb7b0686f6d1474a6f7bd4423809af3d62` | fix(system): remove obsolete DNF repair detector | red (#43286597530) |
| `694d3ceb3c399a58a7de9e2e4952bebf0081d2b4` | fix(repositories): open actions in a transient window | red (#43286692375) |
| `11d2009fd4b3d341326ebeaf1df06012c7b4079f` | fix(system): remove unused DNF repair helper | red (#43287307194) |
| `23397aa8251445a0da44b9dfa373f524c0a59944` | test(system): remove obsolete DNF repair test | green (#43290410800) |
| `bfdbf4162eb52466dc6a6a5523204ceb9eb5909b` | build: enable libadwaita 1.5 APIs | red (#43293426900) |
| `bbde160410b8dd847e4eaa22f49e7c74be8a18c0` | fix(ui): use AdwDialog for repository actions | red (#43293434496) |
| `3c37342692819c59468a3f596850022d6da8bb63` | fix(ui): replace deprecated view switcher title | green (#43294420743) |
| `3f915ba1710ca011e062e1c37f846b39c06edff0` | fix(storage): stabilize persisted project identities | red (#43305091795) |
| `96980e912d476d207d3882c68bf2f1c1789abc07` | fix(monitoring): persist legacy key migrations | red (#43305207520) |
| `0bd8d8e07b79eda3c00fa6fb9532ffb393128220` | style(storage): format stable identity helpers | red (#43305351573) |
| `98fa296215df8feccaf6ce69a8f4bb3e3445234e` | fix(storage): satisfy lint for identity migration | green (#43305656097) |
| `889a8cbd0a3e9de068d6d4f6c65965d3f2542786` | test(storage): verify legacy project data migration | red (#43305999862) |
| `2fd9207c59615e502ef0d1777719a161e7e64f59` | fix(snapshots): import cache path type | green (#43306263103) |
| `c76cb08bd1e9e7c696670e3b94fc635a732598b8` | fix(storage): serialize local state updates | red (#43306863585) |
| `c65fbe29ddf73f954ab0b2cfa6aa345b48dec3a3` | style(storage): format serialized state access | red (#43307495721) |
| `2505919e14117b75ec542e1adaa97924ccbb5931` | style(storage): clean serialized access formatting | green (#43307784906) |
| `32ddbe8751c6339259b8fc58957b03b255aea237` | fix(copr): validate project identity before API use | red (#43308349486) |
| `e762dfc9e9cfd503c848ef63f6aca413e7d7d8a7` | style(copr): format validated project actions | red (#43308623871) |
| `ef0b22334b449b058cdb450a0e2519ea8092e8f4` | fix(system): harden DNF command and repair paths | red (#43308762100) |
| `6ab705c407bb7ff7e729ea04b24fba3df45fe881` | fix(copr): preserve canonical project URL path | red (#43308942614) |
| `1a5805201dadeb0316deb6376a4f95f9c8572b0a` | style(system): format DNF safety checks | red (#43309059831) |
| `d7b8126472af4f77e4a766bba32cc7b70d28f1da` | fix(system): apply hard-link guard to repair checks | green (#43309372136) |
| `40d786a9d992ce6f649bef2db710e8ad91ff2bcd` | fix(activity): report imported history truncation | red (#43309715050) |
| `3fb030b5f3f1c17b776badeba996fc1e4a3d0830` | fix(activity): expose history import limit | green (#43309778896) |
| `273754f1b195fca4a88367ababcd6827720b7547` | fix(copr): distinguish active build freshness | green (#43310364215) |
| `62b5253ddf53f9a425845bf4276b009780e2f713` | fix(favorites): validate imported project metadata | red (#43310577379) |
| `0498cafae15417c8fd019a4e353dcb4bc83e29f3` | style(favorites): format homepage validation | green (#43311216725) |
| `17509b440a8a4632cd5646ed4c0454a47dbc8f34` | fix(system): validate and parse repository metadata | red (#43311340332) |
| `21fd0d7b410d9ea1a8e0fe0124a331328f990357` | fix(system): recover poisoned installed-origin cache | red (#43311547523) |
| `74639a3b740456b6b475035f3d2f370b4b3b7665` | style(system): format repository parser and cache recovery | green (#43311697355) |
| `e25878c33490c9e7d81f3a339db51a4a8192f2a8` | fix(monitoring): normalize upstream tag prefixes | green (#43311914968) |
| `2dbf7a493ba90437466f431e38907c224168dc55` | fix(concurrency): prevent overlapping background checks | green (#43312173623) |
| `7b986a96394e7177bdbf3eb9c264c9d4b3102757` | fix(http): respect bounded Retry-After values | green (#43312264923) |
| `d8eaeb73a8808d0523ce5dd7fda09a0a34beb855` | fix(data): sanitize project descriptions and report cache errors | red (#43312483121) |
| `38456f52205b71b1c99d40a68dfeae465bdaf91c` | test(data): exercise real invisible Unicode characters | red (#43312565211) |
| `a49f4bd7fe338f45cced6cf8615f7d73db2c50e6` | fix(upstream): evict release cache entries by recent use | red (#43312774389) |
| `3483814cfacb3e9d5aad89bbfa873f4decb4385c` | style(data): format cache and sanitizer changes | red (#43312911903) |
| `6cd7f67e143dee17579e42645c09123d8518b1ae` | style(upstream): format LRU cache and finalize audit triage | red (#43313080637) |
| `87b6b58d56bbc2d3fd18cb822d1d515f97ab6325` | fix(upstream): align LRU cache entry types | red (#43313147939) |
| `838ab901794a4e2e2abf111dc831f6592426db22` | fix(copr): classify monitoring failures with typed errors | red (#43313223557) |
| `eda369dee2edb53fa1e24e3149bbdb9356b0fc6f` | fix(tests): repair typed errors and cache fixtures | red (#43313494166) |
| `c3b97f00f4c727689d668c89b8ca6e23186d86b0` | test(system): isolate command environment and exercise fake DNF | red (#43313616285) |
| `7b8d876f1a5e4678c2828761d912e54c22179f16` | fix(favorites): borrow sanitized project during validation | red (#43313715966) |
| `5be8ddb69a40ee29a48d45f125f1e00ae34c3a94` | test(system): cover fake DNF RPM and pkexec commands | red (#43313933095) |
| `babdcfa434e91b658f3696db1042f608e7e970cb` | style(system): format fake command test | red (#43314139710) |
| `046fdd59f115f9a0732132a358ccb9e79a386b48` | fix(data): validate favorites and touch cached releases | red (#43314382227) |
| `dae4088e3d250e325c3565908c3edbea769b1653` | fix(favorites): remove needless project borrows | green (#43314601853) |
| `b10324701778c063f5ea1bce984278dd03ce95d3` | fix(activity): move history file operations off GTK thread | red (#43315198540) |

### Failure diagnosis and recovery

- The DNF permission-repair and repository-action sequence had several red runs while obsolete helpers, tests, and libadwaita APIs were being replaced. The focused follow-up commits restored the DNF repair tests and migrated the repository dialog to supported libadwaita APIs; `3c37342` is green.
- The COPR search, package-origin, and monitoring changes had intermediate formatting, compile, and test failures. Follow-up fixes are recorded above; the combined validation run `74639` is green for the repaired package-origin/monitoring changes.
- The Perplexity Category A batch initially exposed formatting, type, test-fixture, and Clippy failures across the sanitizer, persistent cache, typed error, and command-environment work. The follow-ups corrected the Unicode fixtures, LRU cache entry types, test imports/borrows, and rustfmt output. The final Category A head `dae4088` passed formatting, all 133 unit tests, compilation, and Clippy.
- Activity-history I/O offloading is the next Category B change. Its current commit is not included in this historical section yet; record its CI outcome in the next ledger update.

### Checkpoint

- Reviewed through: `dae4088e3d250e325c3565908c3edbea769b1653` (green).
- Current branch at this ledger update: `b10324701778c063f5ea1bce984278dd03ce95d3`.
- Next review: commits after `b10324701778c063f5ea1bce984278dd03ce95d3`, including the pending activity-history I/O change.


## Follow-up review after `b103247`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `4665aa5c7b6486da61158efb1b2d35fa93a7233d` | fix(favorites): move project save operations off GTK thread | red (#43315405910) |
| `b9a905e2ef351e84bfdfe574e496d1dfb0eaeb15` | fix(activity): remove duplicate refresh callback tail | red (#43315508834) |
| `bba155e21e7dbe8d513a8e94e58286da370a359c` | style(activity): format asynchronous refresh signature | red (#43315704917) |
| `72e89e8d7f5e858d3b7a202a5a524a67accb4e55` | fix(activity): report structured import summary | red (#43315919045) |
| `f61e49ce618eae22fd1dc1d228481df8b024020b` | fix(activity): clone export filters before background work | green (#43316104818) |

### Activity and favorites follow-up diagnosis

- `b103247` first exposed a duplicated tail from the replaced activity clear callback; `b9a905e` removed it, then rustfmt required the refresh signature to be collapsed in `bba155e`.
- `72e89e8` exposed that `ActivityImportSummary` is structured data, not a displayable integer; `f61e49c` now reports its added/incoming/duplicate/dropped fields and clones chooser filters before moving them to the worker.
- `4665aa5` and its follow-ups moved favorites save/remove plus activity-history import/export/clear/load into background workers. The final follow-up `f61e49c` is green; live GNOME responsiveness and feedback still require user validation.
- The repository-origin guard issue `#60` is being fixed with explicit state instead of user-facing label parsing. This commit's CI result will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `f61e49ce618eae22fd1dc1d228481df8b024020b` (green).
- Next review: commits after `f61e49ce618eae22fd1dc1d228481df8b024020b`.


## Follow-up review after `f61e49c`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `e4de71bcb622ec17cc7515fef3708f625b6d66f6` | fix(repositories): guard actions with explicit origin state | red (#43316460232) |
| `1a69bb4c2d730b80357e068426bec3db69612512` | style(repositories): format origin state test | red (#43316671836) |
| `9b409f6fb18dbef1158ea0b04c1dff4e6f002718` | style(repositories): match origin state test formatting | green (#43317002401) |

### Repository-origin state follow-up

- `e4de71b` introduced the explicit repository-origin state and focused state test; the first run failed only because the assertion formatting did not match rustfmt.
- `1a69bb4` corrected the first formatting diff but rustfmt required the alternate chained-assert layout; `9b409f6` matches rustfmt and passed formatting, tests, compilation, and Clippy.
- Category A is complete again. #60 is closed after green CI. Activity/favorites operations (#61/#62) and chooser re-entry prevention (#59) remain Category B pending live GNOME validation.
- This commit's CI result will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `9b409f6fb18dbef1158ea0b04c1dff4e6f002718` (green).
- Next review: commits after `9b409f6fb18dbef1158ea0b04c1dff4e6f002718`.


## Follow-up review after `9b409f6`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `081fbc2de2f4587012aa72bfb8421e482d8caf19` | fix(activity): prevent repeated chooser launches | green (#43317277814) |

### Chooser re-entry follow-up

- `081fbc2` passed formatting, unit tests, compilation, and Clippy. It keeps Activity Import/Export disabled from chooser launch through cancellation or background-worker completion.
- Saved-project Import is being updated to use the same lifecycle: disable at chooser launch, remain disabled through preview and confirmation, and restore on cancel, preview failure, or import completion. This commit's CI result will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `081fbc2de2f4587012aa72bfb8421e482d8caf19` (green).
- Next review: commits after `081fbc2de2f4587012aa72bfb8421e482d8caf19`.


## Follow-up review after `081fbc2`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `434b34f65b6ccfbb6153e33b302cc2a99c30d8fc` | fix(favorites): prevent repeated import dialogs | green (#43317501194) |

### Repository duplicate-action protection

- `434b34f` passed formatting, unit tests, compilation, and Clippy. Saved-project Import stays disabled from chooser launch through preview, confirmation, cancellation, and import completion.
- This commit adds per-row button state to repository Enable/Disable/Remove flows. It keeps the initiating action disabled during COPR compatibility lookup and confirmation, restores it on cancellation or operation failure, and relies on repository refresh to replace it after success. CI outcome will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `434b34f65b6ccfbb6153e33b302cc2a99c30d8fc` (green).
- Next review: commits after `434b34f65b6ccfbb6153e33b302cc2a99c30d8fc`.


## Follow-up review after `434b34f`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `86c6d6c4b6acdec32b83de7451814160b2a21ca9` | fix(repositories): prevent duplicate repository actions | green (#43318053837) |

### Repository action follow-up

- `86c6d6c` passed formatting, unit tests, compilation, and Clippy. Repository Enable/Disable/Remove actions now disable the originating button through compatibility lookup, confirmation, and privileged DNF work, restoring it on cancellation or failure.
- The next package-action change extends the same lifecycle to Installed-page Remove and Project Details Install. Its CI result will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `86c6d6c4b6acdec32b83de7451814160b2a21ca9` (green).
- Next review: commits after `86c6d6c4b6acdec32b83de7451814160b2a21ca9`.


## Follow-up review after `86c6d6c`

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| `bcc213e3158a06f257cdb220c7fbd4dae9ac335a` | fix(packages): prevent duplicate package operations | green (#43318379172) |

### Package and dependency action follow-up

- `bcc213e` passed formatting, unit tests, compilation, and Clippy. Installed-page Remove and Project Details Install buttons are disabled through confirmation and DNF completion, then restored on cancellation/failure or refreshed on success.
- This commit prevents duplicate dependency windows by disabling the originating Dependencies button until its modal window closes. CI outcome will be recorded in the next ledger update.

### Checkpoint

- Reviewed through: `bcc213e3158a06f257cdb220c7fbd4dae9ac335a` (green).
- Next review: commits after `bcc213e3158a06f257cdb220c7fbd4dae9ac335a`.

## Follow-up review after bcc213e

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 4ec170f32ca0bbc334d926682ba0ae74236550f6 | fix(packages): prevent duplicate dependency windows | green (#43318570535) |
| 1e32e174c7e8c94fe66c97ba2f2b51f388aa82b6 | fix(storage): preserve damaged user data for recovery | red (#43362403596) |
| f70d87cebc190ffb979465f6a59812b5a6f5637a | fix(favorites): preserve corrupted saved-project data | red (#43362429792) |
| 259acbb6bd2729ddfcda9ac885b7c39a24466ad8 | fix(activity): preserve corrupted history for recovery | red (#43362450488) |
| be390b3ae0583b800f799b0b443defa507a7d65b | fix(saved): offer recovery for damaged project data | red (#43362491757) |
| 2eb1edfccfa07a0a2b69298111d33a9982df23f0 | fix(activity): offer recovery for damaged history | red (#43362510560) |
| 93983b0d208b8622a59ca561edbebbdebf6e302d | fix(activity): keep recovery action at module scope | red (#43362534199) |
| 963d3c3b11b7402f9852303d195f66f9112bd04a | test(data): keep preserved corruption backups intact | red (#43362557972) |
| 7adfb19cbe8aab1d2fea865aa422de7df45935a4 | test(data): keep preserved corruption backups intact | red (#43362562620) |
| 0d87c9a4e01a13cc88f660c33c1d535c679e6ace | style(storage): format corruption preservation helper | red (#43362621079) |
| 313d2fbd79484e0a0e275a7037604f31036ae8f5 | style(favorites): format recovery test timestamps | red (#43362632740) |
| c91993692d192d9b9c830a502ae34b09b30fdee2 | style(activity): format recovery validation and tests | red (#43362771934) |
| 892a8b88d2792d084383571e7333a30ee3980f87 | style(activity): format recovery action boundaries | red (#43362785860) |
| 2e4b6cb3841c56099dd6358368db7b9972a0c076 | style(activity): format recovery test timestamps | red (#43362931195) |
| c7b966987341a11ef1b1a4a73754d837c29be595 | style(favorites): format recovery test timestamps | green (#43362937433) |

### Damaged local-data recovery follow-up

- The intermediate red runs above failed only at cargo fmt --all -- --check; their formatting differences were corrected in subsequent commits. No test, compilation, or Clippy failure remained on the final recovery commit.
- #141 and #146 now preserve invalid favorites/activity files under timestamped sibling .corrupt-* names only after explicit user confirmation. The original bytes are retained; the app then starts with an empty list/history. Valid files are refused by the recovery path.
- Final validation at c7b9669 passed formatting, all 139 unit tests, cargo check --all-targets, and Clippy. Live GNOME recovery and manual-restore validation remains outstanding.

### Checkpoint

- Reviewed through: c7b966987341a11ef1b1a4a73754d837c29be595 (green).
- Next review: commits after c7b966987341a11ef1b1a4a73754d837c29be595.

## Follow-up review after c7b9669

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 4b3d267bdb3991917a6f5fe132879e847bfbffc0 | fix(monitoring): improve manual scan and backup feedback | red (#43363293716) |

- The 4b3d267 run failed only at cargo fmt --all -- --check. The reported rustfmt differences are corrected in the follow-up formatting commit; tests, compilation, and Clippy had not run on this failed attempt.
- The monitoring progress indicator, human-readable worker failure, and actionable scheduled-backup error guidance are implemented; final CI verification is pending.

## Follow-up review after d77d2ab

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| d77d2abc592d63a4e3e9bed366f5d39cd5657349 | style(monitoring): format progress and backup guidance | green (#43363490031) |

- The progress indicator and backup-guidance changes passed formatting, all 140 unit tests, cargo check --all-targets, and Clippy.
- #66–#68 are implemented; live GNOME animation, error feedback, and notification delivery remain to be verified.
- The next review starts after d77d2abc592d63a4e3e9bed366f5d39cd5657349.
 
## Follow-up review after d77d2ab

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| fd06ca0e8cc42fc1a738b8568890f41f8017c105 | fix(discover): separate activity persistence warnings | green (#43363655724) |

- The Discover feedback change passed formatting, all 140 unit tests, cargo check --all-targets, and Clippy.
- #70 now keeps search-result feedback independent from activity-history persistence errors; the separate warning and detailed stderr diagnostic remain to be checked in a live GNOME session.

### Checkpoint

- Reviewed through: fd06ca0e8cc42fc1a738b8568890f41f8017c105 (green).
- Next review: commits after fd06ca0e8cc42fc1a738b8568890f41f8017c105.
 
## Follow-up review after fd06ca0

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| a6bd4107930e14aaae451151697fdabe7409c608 | fix(project-details): refresh content in place | red (#43363943203) |

- The a6bd410 run passed formatting, all 140 unit tests, and cargo check --all-targets. Clippy failed on one needless double borrow in the new in-place refresh helper; that borrow is removed in the follow-up.
 
## Follow-up review after fd06ca0

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| a6bd4107930e14aaae451151697fdabe7409c608 | fix(project-details): refresh content in place | red (#43363943203) |
| 0b7617839fbac2e5842b2512a943a7dca9982d60 | fix(project-details): remove redundant window borrow | green (#43364104258) |

- The first in-place refresh attempt passed formatting, all 140 tests, and compilation but Clippy found one needless double borrow. The follow-up removed it and passed all CI checks, including Clippy.
- #72 now refreshes the existing details window and restores the prior scroll position. Live GNOME validation remains pending.
 
## Follow-up review after 0b76178

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 81904756ffa30adc3a2951cb27998020afaf6994 | feat(discover): paginate COPR search results | red (#43364383819) |

- The 8190475 run failed only at cargo fmt --all -- --check. The reported line-wrapping differences are corrected in the follow-up formatting commit; tests, compilation, and Clippy were skipped.
 
## Follow-up review after 0b76178

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| ff4f3ac957390ae09451aa230e2b3206c51eed5a | style(discover): format paginated search requests | red (#43364552967) |

- The ff4f3ac run passed formatting, all 142 unit tests, and cargo check --all-targets. Clippy found only the now-unused legacy search_projects wrapper; the UI and tests use search_projects_page directly, so the wrapper is removed and its empty-query test now covers the paginated API.
 
## Follow-up review after 0b76178

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| a45dfcca4a7b73201124141ed985fd2212f59757 | fix(discover): remove unused search wrapper | green (#43364761455) |

- The a45dfcc follow-up passed formatting, all 142 unit tests, cargo check --all-targets, and Clippy. The pagination implementation is now compiled and lint-clean; live COPR search behavior remains to be verified.
 
## Follow-up review after a45dfcc

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 1264f16dab1d76867caae8cc962914da7ce59365 | fix(discover): deduplicate paginated project results | red (#43364911453) |

- The 1264f16 run failed only at cargo fmt --all -- --check because rustfmt wanted the helper signature on one line. Formatting is corrected in the follow-up; tests, compilation, and Clippy were skipped.
 
## Follow-up review after 1b7af70

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 1b7af707099c58ef3aca1f2493500e650788d8ee | style(discover): format project deduplication helper | green (#43365049415) |

- The pagination/deduplication follow-up passed formatting, all 143 unit tests, cargo check --all-targets, and Clippy.
- #91 now has offset pagination, a Load more control, retry-safe error handling, stable local sorting/filtering, and duplicate suppression across pages. Live COPR search validation remains pending.
 
## Follow-up review after 1b7af70

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| c248cf2dce0d0b36d8011e75b5634a5f96c89f09 | fix(ui): clarify package and build target summaries | red (#43365495750) |

- The c248cf2 run failed only at cargo fmt --all -- --check. Rustfmt requested line wrapping in bounded_lines and the repository-origin UI; those changes are corrected in the follow-up. Tests, compilation, and Clippy were skipped.
- Corrected the prior pagination ledger entry: run 43365049415 passed 143 tests, not 144.
 
## Follow-up review after ce487ce

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| ba9a2c9b0a2a7d00fbb3b85dcdc05a9f2b42a25d | fix(system): bound external command execution | red (#43365845693) |
| ca21e861caef5670a7f0b070595ff67d903fc0d0 | test(system): cover command timeouts | red (#43365866794) |

- Both command-timeout commits failed only at cargo fmt --all -- --check. Their formatting differences are corrected in this follow-up; no tests or compilation ran on those attempts.
- The bounded-command work is preserved unchanged apart from rustfmt formatting.
 
## Follow-up review after ca21e86

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| aa5ebf2b41e7ce5b4d214dc9542f55b2be1cda71 | fix(repositories): report unreadable configuration files | red (#43366114771) |

- The aa5ebf2 run passed formatting but failed compilation because the new recovery test lacked SystemTime/UNIX_EPOCH imports. It also exposed two missing mutable bindings in the concurrent command-timeout reader after its output cap was removed. Both are corrected in this follow-up.


## Follow-up review after 15a66db

| Commit SHA | Subject | CI result / workflow run |
|---|---|---|
| 15a66db56dcde0ace6743e4f91a1f55534c09b3a | fix(system): correct timed command readers and recovery test | red (#43366267733) |

- Formatting, unit tests, and compilation passed. Clippy rejected test-only `SystemTime`/`UNIX_EPOCH` imports at module scope; they are now scoped to the test module.
- Review also found that a child can exit while a descendant keeps inherited stdout/stderr pipes open. Reader completion is now covered by the same deadline, with a regression test for this case.
