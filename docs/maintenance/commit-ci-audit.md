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
- Checkpoint SHA: `674d5dbe8b13842e653c618b2d2db978e5cc0997`
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

## Red-cross investigation log

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
| 2026-11-12 | `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d` → `674d5dbe8b13842e653c618b2d2db978e5cc0997` | Reviewed 4 commits: 2 green, 2 red; both red runs were diagnosed as a flawed new test assertion and followed by a green correction. |
| 2026-11-12 | `0f2aff926d2b7e455b97097b2d1b1f99303dc4a2` → `6c62ba5682cbd557b89e1e7af8bd42b3f708d09d` | Reviewed 2 commits: 1 red, 1 green; the red test expectation was corrected and final CI passed. |
