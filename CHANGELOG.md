# Changelog

All notable changes to this project are documented here. It is generated from
[Conventional Commits](https://www.conventionalcommits.org) by
[git-cliff](https://git-cliff.org) — do not edit it by hand.

## [4.0.7](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.6...v4.0.7) — 2026-11-10

### 📦 Dependencies

- Bump tui-nuances to 0.1.11 ([`5ef228d`](https://github.com/sorinirimies/tui-file-explorer/commit/5ef228df5dd7ff202eff15241eff26cf762d09a5))
- Nightly cargo update 2026-11-10 ([`3127f2c`](https://github.com/sorinirimies/tui-file-explorer/commit/3127f2c82f139597d78dee0bdbf72326f45046b7))

## [4.0.6](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.5...v4.0.6) — 2026-11-10

### 📦 Build

- **changelog:** Silence git-cliff warnings ([`5acf96e`](https://github.com/sorinirimies/tui-file-explorer/commit/5acf96e223a6395465f24207493be0195fb63633))

## [4.0.5](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.4...v4.0.5) — 2026-11-10

### 🐛 Bug fixes

- **ci:** Dedupe releases and tolerate missing assets when pruning ([`21844cc`](https://github.com/sorinirimies/tui-file-explorer/commit/21844cc25d0f41fd579dd089b0aa1420cc7517f4))

## [4.0.4](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.3...v4.0.4) — 2026-11-10

### 📦 Dependencies

- Nightly cargo update 2026-11-09 ([`dd1d51e`](https://github.com/sorinirimies/tui-file-explorer/commit/dd1d51e4094b0fef000967d0f10027b50d74b2ba))
- Nightly cargo update 2026-11-08 ([`4201506`](https://github.com/sorinirimies/tui-file-explorer/commit/4201506fcd8c7362c0aeda915165750f721c5a91))

### 🔄 CI and release

- **release:** Ship tfe binaries via the house --desktop packaging ([`72b7bf4`](https://github.com/sorinirimies/tui-file-explorer/commit/72b7bf4214bebb42821ab637184d9469e01f532f))
- **release:** Ship tfe like GitKraft: tfe-<target> archives, .deb, .rpm and AppImage ([`e21a11c`](https://github.com/sorinirimies/tui-file-explorer/commit/e21a11c7dfdbf0573ef4242ad9f9bfed8ca5f9cf))
- **release:** Attach tfe binaries for every platform to the GitHub release ([`9d51a90`](https://github.com/sorinirimies/tui-file-explorer/commit/9d51a9069f3de2927bf3405cdddb9d92d921e899))

## [4.0.3](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.2...v4.0.3) — 2026-11-07

### 📦 Dependencies

- Upgrade dependencies 2026-11-07 ([`b69942f`](https://github.com/sorinirimies/tui-file-explorer/commit/b69942f948eca1512d5b2cacb14e85dc8ad67df6))
- Nightly cargo update 2026-11-06 ([`1204782`](https://github.com/sorinirimies/tui-file-explorer/commit/120478227589fb75aeab3dd2e4573a57e73847c6))

### 🔄 CI and release

- **gitea:** Check the MSRV on Gitea too, with the same command as GitHub ([`bf6938a`](https://github.com/sorinirimies/tui-file-explorer/commit/bf6938af730f8b15ece801bc05a176bf7dc088d0))
- Download archives to a file so curl retries a dropped transfer, instead of piping into tar ([`9aa4223`](https://github.com/sorinirimies/tui-file-explorer/commit/9aa4223d05d6db3c3fc1d4b2e0917dd724c6f76c))
- The Dependabot sweep asks for a rebase on conflicts and keeps going after a failed merge ([`b4d051c`](https://github.com/sorinirimies/tui-file-explorer/commit/b4d051cf22be9d34715592027e8b0e87366efde2))
- Merge green Dependabot PRs, majors included, with a nightly sweep; nightly commits skip ignored lockfiles ([`147508b`](https://github.com/sorinirimies/tui-file-explorer/commit/147508b534f18bf34479e1597349256bef16fa28))

## [4.0.2](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.1...v4.0.2) — 2026-11-05

### 📦 Dependencies

- Upgrade dependencies 2026-11-05 ([`cb40fd1`](https://github.com/sorinirimies/tui-file-explorer/commit/cb40fd10e149c22c59ae5f711130cf3576a7beee))

## [4.0.1](https://github.com/sorinirimies/tui-file-explorer/compare/v4.0.0...v4.0.1) — 2026-11-04

### 🐛 Bug fixes

- **git:** Match the repository root on Windows ([`d000781`](https://github.com/sorinirimies/tui-file-explorer/commit/d0007818cef343757ddc33e497475bacd269e31d))

### ♻️ Refactor

- Remove dead_code allows the build doesn't need; gate on unused dependencies ([`a16eda7`](https://github.com/sorinirimies/tui-file-explorer/commit/a16eda73d1fa7064d3f7bc1246e4e1be0b4122b7))

### 📚 Documentation

- Document the public API ([`a1f0f9b`](https://github.com/sorinirimies/tui-file-explorer/commit/a1f0f9b9d42cd15beb5a2e97481b52983d2478b9))
- Drop the per-repo agent rules skill; the shared skills cover it ([`e08154b`](https://github.com/sorinirimies/tui-file-explorer/commit/e08154b79a095999560ae943cfb1b7a08eb6e308))
- This repo's agent rules as a project skill in .agents/skills ([`7d9c921`](https://github.com/sorinirimies/tui-file-explorer/commit/7d9c921f8ffe74d5e612b5538dd88ee270f1322d))
- **readme:** Add the Audit and Fuzz badges ([`3530670`](https://github.com/sorinirimies/tui-file-explorer/commit/353067030b7f2842472809c571748970d23e42bc))
- Move agent rules from .zed/rules.md to AGENTS.md ([`82bdf43`](https://github.com/sorinirimies/tui-file-explorer/commit/82bdf433b5b2518a0e84d0fbdd23977598ffc50a))

### 🧪 Tests

- Run the POSIX shell install tests on unix only ([`81331cf`](https://github.com/sorinirimies/tui-file-explorer/commit/81331cff0861168bda18a7f6ef4b58c236356a92))

### 📦 Dependencies

- Upgrade dependencies 2026-11-04 ([`c552bb9`](https://github.com/sorinirimies/tui-file-explorer/commit/c552bb942a565ba9c33b550ebea7fd55db74f8af))

### 📦 Build

- Align tooling with the library template ([`5c3b193`](https://github.com/sorinirimies/tui-file-explorer/commit/5c3b19365e7c666c0176ff1e541b7e1258b57866))

### 🔄 CI and release

- Scan the git history for secrets with gitleaks ([`260cd7f`](https://github.com/sorinirimies/tui-file-explorer/commit/260cd7f1b2615b30af51cab2a9137d8ea6965b13))
- Bump action-gh-release and fetch-metadata to v3; docs: version 4 in the dependency snippets ([`e4c3964`](https://github.com/sorinirimies/tui-file-explorer/commit/e4c3964311a8507a890b039970510df303755f2d))

### 🔧 Chores

- Ignore .codegraph index ([`585fb7a`](https://github.com/sorinirimies/tui-file-explorer/commit/585fb7ae24cf32290f2d572cd77219577bfd4b00))
- Remove AGENTS.md ([`c0d8145`](https://github.com/sorinirimies/tui-file-explorer/commit/c0d81459c58be37ab34bc2d74419248ad2de0f04))

## [4.0.0](https://github.com/sorinirimies/tui-file-explorer/compare/v3.2.0...v4.0.0) — 2026-10-27

### ⚠️ Breaking changes

- Tui-nuances is the only theme source; replace Theme::all_presets and named constructors with Theme::presets ([`479e9ed`](https://github.com/sorinirimies/tui-file-explorer/commit/479e9ede451145d428a1b851dcb99359949ceb6b))

### ✨ Features

- Tui-nuances is the only theme source; replace Theme::all_presets and named constructors with Theme::presets ([`479e9ed`](https://github.com/sorinirimies/tui-file-explorer/commit/479e9ede451145d428a1b851dcb99359949ceb6b))

### 📚 Documentation

- Make all doc-tests real (remove no_run/ignore) ([`6ae6750`](https://github.com/sorinirimies/tui-file-explorer/commit/6ae675020f8eca5a38907a88bdcc90c8f9ac44f9))

### 📦 Dependencies

- Bump tui-nuances to 0.1.10 ([`89d06fa`](https://github.com/sorinirimies/tui-file-explorer/commit/89d06fa3193d9162497fee101d4fcceca262a199))

## [3.2.0](https://github.com/sorinirimies/tui-file-explorer/compare/v3.1.1...v3.2.0) — 2026-10-25

### ✨ Features

- **theme:** Source all built-in presets from tui-nuances 0.1.9; theme always available ([`cbb7547`](https://github.com/sorinirimies/tui-file-explorer/commit/cbb7547aa834b046482a87b912353a4adb55d902))

### 🧪 Tests

- **app:** Cover key dispatch for panels, editor, pane, debug scroll, modal ([`9190c8c`](https://github.com/sorinirimies/tui-file-explorer/commit/9190c8c80edb52227b115f3f1a56915e4fa7d127))

## [3.1.1](https://github.com/sorinirimies/tui-file-explorer/compare/v3.1.0...v3.1.1) — 2026-10-25

### ✨ Features

- **archive:** Extraction limits, cleanup on failure, zip 9, archive_list fuzz target ([`ebc38b1`](https://github.com/sorinirimies/tui-file-explorer/commit/ebc38b146668cdb26432b99379d0f306622df81c))

### 🐛 Bug fixes

- Cargo fmt; avoid char-boundary panic when truncating header path ([`7db8268`](https://github.com/sorinirimies/tui-file-explorer/commit/7db82681a250508a22ef1faff8a4a971cb3fdfb8))

### 🧪 Tests

- **ui:** Smoke tests for theme/editor/options panels ([`2c664fb`](https://github.com/sorinirimies/tui-file-explorer/commit/2c664fb864e7cf2e29db28b447c5322494a601af))

### 🔄 CI and release

- Bump actions/checkout from 4 to 7 (#6) ([`3d51038`](https://github.com/sorinirimies/tui-file-explorer/commit/3d51038c2b7fe7c1a63a38d5d0d26ff2b59b540d))
- Bump clechasseur/rs-clippy-check from 4 to 5 (#4) ([`daefd71`](https://github.com/sorinirimies/tui-file-explorer/commit/daefd7138d80e4a7ebeaf880570844121acdee70))
- Bump peter-evans/create-pull-request from 6 to 8 (#5) ([`7276649`](https://github.com/sorinirimies/tui-file-explorer/commit/7276649c03535184dfbfb07f718a91a519ca9d08))
- Bump softprops/action-gh-release from 1 to 3 (#3) ([`4606ea1`](https://github.com/sorinirimies/tui-file-explorer/commit/4606ea1b96d8ccc6f747b76a522eb994c475fc5b))
- **gitea:** Nightly workflow-action bumps ([`781656d`](https://github.com/sorinirimies/tui-file-explorer/commit/781656d2368a2dcc1f75cbc95f9fc1e2f2354f5e))
- Automerge major dependabot bumps too ([`4fcfe96`](https://github.com/sorinirimies/tui-file-explorer/commit/4fcfe96063218e417c84afabf516c5d0375fbff9))
- Automerge green PRs (workflow_run) + dependabot for actions ([`00ca18a`](https://github.com/sorinirimies/tui-file-explorer/commit/00ca18a402e0688ab90928373eb4557166256569))
- Nightly merges lockfile-only updates; bump + release only when a Cargo.toml requirement changes ([`d490ce3`](https://github.com/sorinirimies/tui-file-explorer/commit/d490ce388bb2d73504506ea693d05a5af0945184))
- Nightly dependency update pushes lockfile only, no version bump or release ([`5e5d890`](https://github.com/sorinirimies/tui-file-explorer/commit/5e5d8900f3565f22f19ed5839c53fb00df728a9f))

### 🔧 Chores

- Bump tui-nuances to 0.1.7 ([`850d92e`](https://github.com/sorinirimies/tui-file-explorer/commit/850d92ea3274678e11df41f3960c050c9c1a4330))
- **justfile:** Push/pull current branch instead of hardcoded main ([`61b5bef`](https://github.com/sorinirimies/tui-file-explorer/commit/61b5beff557c7153f6f36505b56a7cc1e940dac8))
- **justfile:** Skip unconfigured remotes in *-all recipes ([`74b1188`](https://github.com/sorinirimies/tui-file-explorer/commit/74b1188eaa7161b274ebbbe85582444521a3c66f))

## [3.1.0](https://github.com/sorinirimies/tui-file-explorer/compare/v2.3.0...v3.1.0) — 2026-10-14

### ⚠️ Breaking changes

- Fuzzy finder, bookmarks, undo/redo, bulk rename, git markers, archives, keymap.toml, async loading ([`a04f52d`](https://github.com/sorinirimies/tui-file-explorer/commit/a04f52df3f30a472daf821ca0adf1d6a66fc4774))

### ✨ Features

- Make theming an optional "theme" feature ([`64c730d`](https://github.com/sorinirimies/tui-file-explorer/commit/64c730d1af2c011add5c37f69e6a097213146e9f))
- Shared theming via tui-nuances ([`0e0abdb`](https://github.com/sorinirimies/tui-file-explorer/commit/0e0abdb5064b16d1771efeec1b819c76098da5b5))
- Fuzzy finder, bookmarks, undo/redo, bulk rename, git markers, archives, keymap.toml, async loading ([`a04f52d`](https://github.com/sorinirimies/tui-file-explorer/commit/a04f52df3f30a472daf821ca0adf1d6a66fc4774))

### 🐛 Bug fixes

- Repaint the whole UI after an external editor exits ([`7eb8316`](https://github.com/sorinirimies/tui-file-explorer/commit/7eb8316bbc82f9835ac976d46f04414f11951ccc))

### 📚 Documentation

- Re-record demos showing results only; reference tui-nuances version ([`37c2e3e`](https://github.com/sorinirimies/tui-file-explorer/commit/37c2e3e5afc3f393dbbd7ff19c1c66cd18d2fc73))
- **demo:** Add generated GIFs for the new feature tapes ([`9c4aef7`](https://github.com/sorinirimies/tui-file-explorer/commit/9c4aef72984a2939c1403f5b3add1d421593b632))
- **demo:** Add VHS tapes for sizes, fuzzy finder, bookmarks, undo/redo, bulk rename, git and archives ([`d8a47a3`](https://github.com/sorinirimies/tui-file-explorer/commit/d8a47a36829ae036c121b5577297bcbbaac247a1))
- Regenerate CHANGELOG.md after merging Gitea reconciliation ([`da27807`](https://github.com/sorinirimies/tui-file-explorer/commit/da2780724becadb7e3d2d868ecf020643b9e7d5e))
- Regenerate CHANGELOG.md now that v2.2.0/v2.3.0 tags exist on origin ([`2880ab9`](https://github.com/sorinirimies/tui-file-explorer/commit/2880ab954fee4fde6002bba3f117fe9920b4c588))

### 📦 Dependencies

- Nightly cargo update 2026-09-18 ([`73dbf19`](https://github.com/sorinirimies/tui-file-explorer/commit/73dbf19184250564c930e4b8fef0ad08d6bc4132))
- Nightly cargo update 2026-09-16 ([`e085b75`](https://github.com/sorinirimies/tui-file-explorer/commit/e085b750ac3886d4b8c3cc4adae67588594e45cd))
- Nightly cargo update 2026-09-14 ([`4f3883f`](https://github.com/sorinirimies/tui-file-explorer/commit/4f3883f9671ff2880850c1fb109438e5b84835a4))
- Nightly cargo update 2026-09-12 ([`d342dd9`](https://github.com/sorinirimies/tui-file-explorer/commit/d342dd95f50bef11202558f4a868927034dad09a))

### 🔄 CI and release

- Prebuilt Nushell, cargo-deny + cargo-audit security, cargo-fuzz targets; tui-nuances 0.1.4; fix PageDown huge page_size hang ([`483f3ab`](https://github.com/sorinirimies/tui-file-explorer/commit/483f3ab0fc6e804c9740e752c9ea49884c5e3531))
- **security:** Add cargo-audit workflow and just audit recipe ([`2096da5`](https://github.com/sorinirimies/tui-file-explorer/commit/2096da57295e6bceccf12fdcac0143c85227ef3c))
- Make the nightly dependency update robust and add MSRV, audit and feature checks ([`ff81f76`](https://github.com/sorinirimies/tui-file-explorer/commit/ff81f7620cfce76f18342d9bd5e78afef8c35867))

### 🔧 Chores

- Depend on tui-nuances 0.1.6 from crates.io ([`baac400`](https://github.com/sorinirimies/tui-file-explorer/commit/baac4008b8c59555ee25f0674dffec78237395cd))
- Depend on tui-nuances 0.1.6 ([`9dad171`](https://github.com/sorinirimies/tui-file-explorer/commit/9dad171743e88b83d03072c85b7a9717aaa21a47))
- Depend on tui-nuances 0.1.5 ([`231c9b6`](https://github.com/sorinirimies/tui-file-explorer/commit/231c9b6d4e1007e8000dc8b397c8c3cefe971439))

## [2.2.0](https://github.com/sorinirimies/tui-file-explorer/compare/v2.1.0...v2.2.0) — 2026-09-11

### ✨ Features

- Add adaptive action hint layouts ([`5d9e0f5`](https://github.com/sorinirimies/tui-file-explorer/commit/5d9e0f5ca7840e469933e41229c1ff6d7bfa910f))

## [2.1.0](https://github.com/sorinirimies/tui-file-explorer/compare/v2.0.3...v2.1.0) — 2026-09-10

### 🐛 Bug fixes

- Correct justfile remote names (gitea-microlab/starscream -> gitea/gitea_starscream) ([`0861c35`](https://github.com/sorinirimies/tui-file-explorer/commit/0861c3592a70e0fe7a64acf8c9e9340841bbfac3))

## [2.0.3](https://github.com/sorinirimies/tui-file-explorer/compare/v2.0.2...v2.0.3) — 2026-09-10

### ✨ Features

- Expand embeddable library API ([`6f91660`](https://github.com/sorinirimies/tui-file-explorer/commit/6f91660354aed0a8c76dfd9f0af447f15d5c92ec))

### 📚 Documentation

- Prioritize key previews ([`ac1fa4d`](https://github.com/sorinirimies/tui-file-explorer/commit/ac1fa4d608206dada7692fe34056693cfa870cba))
- Show all previews and clarify library crate ([`f745b1c`](https://github.com/sorinirimies/tui-file-explorer/commit/f745b1cdfd7d1dc9f99f2c5b62167c8f52543039))
- Clarify library embedding ([`1f70162`](https://github.com/sorinirimies/tui-file-explorer/commit/1f70162f5fc63f490510fe52c0cef8d847551805))
- Streamline README and normalize demo sizes ([`fd7a518`](https://github.com/sorinirimies/tui-file-explorer/commit/fd7a5185640c811ee424304bf33d12866563e9de))
- Add multi-pane demo recordings ([`45eeba4`](https://github.com/sorinirimies/tui-file-explorer/commit/45eeba4b9cf5cf21a1a48f4835b8e1c34952aab5))

### 📦 Dependencies

- Nightly cargo update 2026-09-10 ([`17ff0ef`](https://github.com/sorinirimies/tui-file-explorer/commit/17ff0ef8c4b78d1a100fb549f0c5c02d7c7f449b))

### 🔧 Chores

- Update dependencies to latest releases ([`c1c9bfe`](https://github.com/sorinirimies/tui-file-explorer/commit/c1c9bfefaa43cc593d85d3ead8a41a6bc34d9b0f))

## [2.0.2](https://github.com/sorinirimies/tui-file-explorer/compare/v2.0.0...v2.0.2) — 2026-09-08

### 🐛 Bug fixes

- Reconcile Cargo.toml/CHANGELOG.md with published v2.0.0 release ([`7a3c302`](https://github.com/sorinirimies/tui-file-explorer/commit/7a3c302f77e1f9b251fa0509a72dcc92d8d14b6a))

### ♻️ Refactor

- Split explorer.rs and ui.rs into submodules ([`4bed277`](https://github.com/sorinirimies/tui-file-explorer/commit/4bed277f92e7efa32b58b4168f206abc341e47f7))

### 📦 Dependencies

- Nightly cargo update 2026-08-20 ([`594f105`](https://github.com/sorinirimies/tui-file-explorer/commit/594f105445f31fdeb2a2267aca0d51cc080b59ad))

## [2.0.0](https://github.com/sorinirimies/tui-file-explorer/compare/v1.1.11...v2.0.0) — 2026-08-03

### ⚠️ Breaking changes

- Multi-pane layout, file/folder sizes, and snappier rendering ([`f30581e`](https://github.com/sorinirimies/tui-file-explorer/commit/f30581e255aa5c77a8d39daa1be064c1d5fa0caa))

### ✨ Features

- Multi-pane layout, file/folder sizes, and snappier rendering ([`f30581e`](https://github.com/sorinirimies/tui-file-explorer/commit/f30581e255aa5c77a8d39daa1be064c1d5fa0caa))
- Auto-merge nightly dependency updates ([`24c9761`](https://github.com/sorinirimies/tui-file-explorer/commit/24c9761db10f31419d0f588f453d2ade6fc5719e))

## [1.1.11](https://github.com/sorinirimies/tui-file-explorer/compare/v1.1.10...v1.1.11) — 2026-07-17

### 🐛 Bug fixes

- **mac:** Detect image protocol without stdin probe (Ghostty hang) ([`c9f71a8`](https://github.com/sorinirimies/tui-file-explorer/commit/c9f71a888534744c9eb5242e85ba09338bcf092f))
- **tests:** Annotate run-tests list type for nu 0.114 type checker ([`d3f6eb3`](https://github.com/sorinirimies/tui-file-explorer/commit/d3f6eb3f2303b4ba554304ac623ef5fb7ecf6e78))

### 📚 Documentation

- **readme:** Embed all demo GIFs in preview gallery ([`4681f57`](https://github.com/sorinirimies/tui-file-explorer/commit/4681f574c35a0a036947093715c4a306b23b6f2f))

### 💄 Style

- **examples:** Use Option::filter in dual_pane arg parsing ([`8312677`](https://github.com/sorinirimies/tui-file-explorer/commit/831267787231249c25f9ee927ff15f33ba358f4a))

## [1.1.10](https://github.com/sorinirimies/tui-file-explorer/compare/v1.1.9...v1.1.10) — 2026-07-10

### 🐛 Bug fixes

- **mac:** Allowlist graphics-capable terminals for image probe ([`fa3aae7`](https://github.com/sorinirimies/tui-file-explorer/commit/fa3aae791b5b2fcdd658fd29b5997e1323c34fda))

## [1.1.9](https://github.com/sorinirimies/tui-file-explorer/compare/v1.1.3...v1.1.9) — 2026-07-10

### 🐛 Bug fixes

- **vhs:** Restore lost demo GIF LFS objects and regenerate picture_preview ([`33b0f58`](https://github.com/sorinirimies/tui-file-explorer/commit/33b0f58e7e5ee5bb096d3155fdd72c3622241c01))
- **mac:** Skip from_query_stdio on Apple Terminal and non-tty stdout ([`79b96b1`](https://github.com/sorinirimies/tui-file-explorer/commit/79b96b14efadb984c71384976f114369e68473ad))

## [1.0.4](https://github.com/sorinirimies/tui-file-explorer/compare/v1.0.2...v1.0.4) — 2026-04-21

### 🐛 Bug fixes

- Fix formating, improve perf. cleanup ([`c9a1a33`](https://github.com/sorinirimies/tui-file-explorer/commit/c9a1a337d8076775c6ef3354ce8ffa705b4ffa42))

## [1.0.2](https://github.com/sorinirimies/tui-file-explorer/compare/v1.0.1...v1.0.2) — 2026-04-17

### 🐛 Bug fixes

- Use sort_by_key to satisfy clippy 1.95 unnecessary_sort_by lint ([`2b77144`](https://github.com/sorinirimies/tui-file-explorer/commit/2b77144f7154a893a44296789e1e5ab7ba873721))

## [0.9.1](https://github.com/sorinirimies/tui-file-explorer/compare/v0.9.0...v0.9.1) — 2026-03-25

### 🐛 Bug fixes

- Detect and correct --editor <file> misparse ([`0bcd75f`](https://github.com/sorinirimies/tui-file-explorer/commit/0bcd75fa22e7356e7cd2759e21921a3f4b97ab69))

## [0.9.0](https://github.com/sorinirimies/tui-file-explorer/compare/v0.8.8...v0.9.0) — 2026-03-25

### ✨ Features

- Tfe <file> opens directly in configured editor ([`82b5c22`](https://github.com/sorinirimies/tui-file-explorer/commit/82b5c22e8a8e05b77392da8e3a959fd9da49cc73))

### 📚 Documentation

- Document three API tiers in README — widget, dual-pane, full app ([`6cf25e4`](https://github.com/sorinirimies/tui-file-explorer/commit/6cf25e4d78db762ac89024a3ef24c7af9c2ad4ff))

## [0.8.6](https://github.com/sorinirimies/tui-file-explorer/compare/v0.8.5...v0.8.6) — 2026-03-24

### ♻️ Refactor

- Remove 'full' feature — library always exposes complete API ([`d638abb`](https://github.com/sorinirimies/tui-file-explorer/commit/d638abb225e4c48ed030a09f3d8b52ccda155824))

## [0.8.5](https://github.com/sorinirimies/tui-file-explorer/compare/v0.8.4...v0.8.5) — 2026-03-24

### ✨ Features

- Copy/move progress overlay using tui-slider ([`64b5ad9`](https://github.com/sorinirimies/tui-file-explorer/commit/64b5ad97be540e47f59b54bd0490f4ca94505ee1))

### 🐛 Bug fixes

- Clippy — remove unused Color import and redundant f64 cast ([`e1741da`](https://github.com/sorinirimies/tui-file-explorer/commit/e1741dafe717c8e928bc67064481431ca7b2f4c6))
- Paste feedback and symlink handling ([`5b32728`](https://github.com/sorinirimies/tui-file-explorer/commit/5b32728269dcf886d8379ea419820ef7ac0c0594))

## [0.8.4](https://github.com/sorinirimies/tui-file-explorer/compare/v0.8.3...v0.8.4) — 2026-03-24

### 🐛 Bug fixes

- Yank checks both panes for marks — handles dual-pane mark-then-tab-then-y workflow ([`68907d4`](https://github.com/sorinirimies/tui-file-explorer/commit/68907d436e5d99030d3579e724f662a154889898))

## [0.8.3](https://github.com/sorinirimies/tui-file-explorer/compare/v0.8.2...v0.8.3) — 2026-03-24

### 🐛 Bug fixes

- Multi-selection yank — y/x now copies/cuts all marked entries ([`f841683`](https://github.com/sorinirimies/tui-file-explorer/commit/f8416835032717a793810d122244a18ca6899988))

## [0.4.8](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.7...v0.4.8) — 2026-03-10

### 🐛 Bug fixes

- Rename options panel editor label from 'cycle' to 'open with' ([`9787b44`](https://github.com/sorinirimies/tui-file-explorer/commit/9787b444bd338d7d2ddfa687da3eae94a9318e07))

## [0.4.6](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.5...v0.4.6) — 2026-03-06

### 🐛 Bug fixes

- Cross-platform editor launch (Windows .cmd shims, Custom arg splitting) ([`5e51e22`](https://github.com/sorinirimies/tui-file-explorer/commit/5e51e220136cd619813e9a568a695798f2f42cf2))

## [0.4.5](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.4...v0.4.5) — 2026-03-06

### 🐛 Bug fixes

- Enter/l on a file opens editor instead of exiting TUI when editor is configured ([`2f11c38`](https://github.com/sorinirimies/tui-file-explorer/commit/2f11c3845a767b7a91d48d572e510767a72fdb42))

## [0.4.4](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.3...v0.4.4) — 2026-03-06

### 📚 Documentation

- Add agents.md and expand rules.md with Editor and hermetic-test patterns ([`49c8f21`](https://github.com/sorinirimies/tui-file-explorer/commit/49c8f215947cdd010b8d8b3cb9bffd73243a51ec))

## [0.4.3](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.2...v0.4.3) — 2026-03-06

### 🐛 Bug fixes

- Make auto_install_with shell-independent for hermetic tests ([`0a7f2b3`](https://github.com/sorinirimies/tui-file-explorer/commit/0a7f2b3dfb7f93f6acf306caccd725b48fb13cdc))

## [0.4.2](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.1...v0.4.2) — 2026-03-06

### ✨ Features

- Show version label in header bottom-right of active pane ([`578e1c3`](https://github.com/sorinirimies/tui-file-explorer/commit/578e1c3ee96f884f7b912a37476ad95bbb31592c))

## [0.4.1](https://github.com/sorinirimies/tui-file-explorer/compare/v0.4.0...v0.4.1) — 2026-03-06

### 🐛 Bug fixes

- Default cd_on_exit to true now that wrapper is auto-installed ([`85b10de`](https://github.com/sorinirimies/tui-file-explorer/commit/85b10dea1590cb9d21df04fb0c6d746e928f28e7))

## [0.4.0](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.9...v0.4.0) — 2026-03-06

### ✨ Features

- Auto-install shell wrapper on first run ([`6d84773`](https://github.com/sorinirimies/tui-file-explorer/commit/6d84773679304cbad53dc2eefe7d45a6897d8c9e))

## [0.3.9](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.8...v0.3.9) — 2026-03-06

### 🐛 Bug fixes

- Resolve zsh rc file from ZDOTDIR, .zshrc, and .zshenv fallback ([`ca213b4`](https://github.com/sorinirimies/tui-file-explorer/commit/ca213b41d2c6397cc554156e7f04a8cb2d76042d))
- Detect hand-written tfe wrappers to prevent duplicate installs ([`48c5da5`](https://github.com/sorinirimies/tui-file-explorer/commit/48c5da558910127b04e1d54696af0f59e7df4aae))

## [0.3.8](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.6...v0.3.8) — 2026-03-06

### ✨ Features

- Editor launch via e key (v0.3.7) ([`a79b82f`](https://github.com/sorinirimies/tui-file-explorer/commit/a79b82f2f51a37f8156dd3c41709eb0a0354bca4))

### 🐛 Bug fixes

- Silence clippy empty_line_after_doc_comments and derivable_impls ([`e9faffc`](https://github.com/sorinirimies/tui-file-explorer/commit/e9faffcea5a75915c424e31190c41d671c1117bf))

### 📚 Documentation

- Update README/lib docs and add dual_pane.tape ([`f6a5861`](https://github.com/sorinirimies/tui-file-explorer/commit/f6a5861566557e201fbf5d8cfcbe7524d3b6a760))

## [0.3.6](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.5...v0.3.6) — 2026-03-05

### 🔧 Chores

- Cleanup dead code, add 310 new tests, bump to v0.3.6 ([`228b085`](https://github.com/sorinirimies/tui-file-explorer/commit/228b08538e85c1eb7c0f04cd3f02de21c745530e))

## [0.3.5](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.4...v0.3.5) — 2026-03-05

### ✨ Features

- Make cd-on-exit opt-in via --cd / --no-cd flags ([`bbd4787`](https://github.com/sorinirimies/tui-file-explorer/commit/bbd4787d6854c149b1186405bdc6ba5a0d3576f5))

## [0.3.4](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.3...v0.3.4) — 2026-03-05

### 🐛 Bug fixes

- Cross-platform TUI rendering and shell integration ([`2c6eb68`](https://github.com/sorinirimies/tui-file-explorer/commit/2c6eb68869ab6080a0be2a2a7d7d976073fa3c5c))

## [0.3.3](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.2...v0.3.3) — 2026-03-05

### 🐛 Bug fixes

- Render TUI on /dev/tty so shell wrapper $() capture works ([`f11fe28`](https://github.com/sorinirimies/tui-file-explorer/commit/f11fe283fb58ede165f64d1322153df4f758bc5d))

## [0.3.2](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.1...v0.3.2) — 2026-03-05

### ✨ Features

- --init <shell> installs cd-on-exit wrapper automatically ([`6375a89`](https://github.com/sorinirimies/tui-file-explorer/commit/6375a89254e3a4c7b610c39b2d533b47fb100d43))

## [0.3.1](https://github.com/sorinirimies/tui-file-explorer/compare/v0.3.0...v0.3.1) — 2026-03-05

### ✨ Features

- Cd to browsed directory on dismiss ([`25192d1`](https://github.com/sorinirimies/tui-file-explorer/commit/25192d11be3aa0403991ed302e5a7e5603ff356c))

## [0.3.0](https://github.com/sorinirimies/tui-file-explorer/compare/v0.2.3...v0.3.0) — 2026-03-05

### ✨ Features

- Miller-columns navigation and split action bar ([`f8228fc`](https://github.com/sorinirimies/tui-file-explorer/commit/f8228fc646dde401397da7b41827a84939d993f0))

### 📚 Documentation

- Update README key bindings for Miller-columns navigation ([`0e55d1a`](https://github.com/sorinirimies/tui-file-explorer/commit/0e55d1a7a86c32dd83290ffecf0dc2dc02d8e826))

## [0.2.3](https://github.com/sorinirimies/tui-file-explorer/compare/v0.2.2...v0.2.3) — 2026-03-05

### 🐛 Bug fixes

- Arrow keys scroll the list; only Enter/l descend or confirm ([`4384bdf`](https://github.com/sorinirimies/tui-file-explorer/commit/4384bdfb8b8b730b28c3ee8f500bfcf008e33fdb))

## [0.2.2](https://github.com/sorinirimies/tui-file-explorer/compare/v0.2.1...v0.2.2) — 2026-03-05

### 🐛 Bug fixes

- Prevent crashes at navigation boundaries ([`8f1032d`](https://github.com/sorinirimies/tui-file-explorer/commit/8f1032d49c49e019a5e8643a34fa1400c3028dd8))

### 📚 Documentation

- Update README for v0.2.0 DualPane API ([`3462610`](https://github.com/sorinirimies/tui-file-explorer/commit/346261077f1cf7f7c8cc0b84db220a9bfb0d8dee))

## [0.2.1](https://github.com/sorinirimies/tui-file-explorer/compare/v0.2.0...v0.2.1) — 2026-03-05

### ✨ Features

- Add DualPane widget to the library ([`2cf3649`](https://github.com/sorinirimies/tui-file-explorer/commit/2cf36496ae3b7b540ebba694437d69b3a9e6bce4))

## [0.1.10](https://github.com/sorinirimies/tui-file-explorer/compare/v0.1.9...v0.1.10) — 2026-03-04

### 📚 Documentation

- Mandate commit + release after every implementation and test cycle ([`99aeb16`](https://github.com/sorinirimies/tui-file-explorer/commit/99aeb16c66e22edd12eac41cf78434b588debb33))

## [0.1.9](https://github.com/sorinirimies/tui-file-explorer/compare/v0.1.8...v0.1.9) — 2026-03-04

### 🐛 Bug fixes

- Preserve last_dir_right when exiting in single-pane mode ([`f985f6a`](https://github.com/sorinirimies/tui-file-explorer/commit/f985f6abcf31347c946131f3b9869e8b3b4c9e5e))

## [0.1.8](https://github.com/sorinirimies/tui-file-explorer/compare/v0.1.7...v0.1.8) — 2026-03-04

### ♻️ Refactor

- Introduce AppOptions struct to fix too_many_arguments lint ([`9aa8466`](https://github.com/sorinirimies/tui-file-explorer/commit/9aa8466ced189a701377fe280205afe78ff3f0f7))

## [0.1.4](https://github.com/sorinirimies/tui-file-explorer/compare/v0.1.3...v0.1.4) — 2026-02-25

### ✨ Features

- Add file_ops demo GIF, tape, and README wiring ([`8dbce45`](https://github.com/sorinirimies/tui-file-explorer/commit/8dbce4548a22b0e6df09059a8cf4787c95d42042))
- Incremental search, sort modes, and expanded icon set ([`e5e79c9`](https://github.com/sorinirimies/tui-file-explorer/commit/e5e79c9149aa344d8e425115779802875a602b20))

### 📚 Documentation

- Overhaul README, add VHS demo GIFs via Git LFS ([`e66fbf1`](https://github.com/sorinirimies/tui-file-explorer/commit/e66fbf1941511e3027d4a6f16a954ed7404779a3))

### 💄 Style

- Rewrite all VHS tapes to minimal single-line comment style with Catppuccin Mocha theme ([`a02f513`](https://github.com/sorinirimies/tui-file-explorer/commit/a02f5137268eba8d0f7d42acecde1d648463920d))

## [0.1.3](https://github.com/sorinirimies/tui-file-explorer/compare/v0.1.2...v0.1.3) — 2026-02-25

### 🐛 Bug fixes

- Make `just release <version>` fully non-interactive ([`6db05a9`](https://github.com/sorinirimies/tui-file-explorer/commit/6db05a9bb7960a2c180ca4213343b5c536e0ea41))
- Repair version-bump and release-workflow trigger pipeline ([`b44b1c7`](https://github.com/sorinirimies/tui-file-explorer/commit/b44b1c78a98630125d50fc5fa3c8e62344e387fb))

## [0.1.2](https://github.com/sorinirimies/tui-file-explorer/releases/tag/v0.1.2) — 2026-02-25


