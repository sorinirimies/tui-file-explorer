---
name: tui-file-explorer-rules
description: Rules for work in the tui-file-explorer repository that the code can't show: constraints, ownership and must-keep behaviour. Use before changing, fixing, reviewing or extending code here, together with the rust-in-projects skill.
---

# tui-file-explorer: rules the code can't show

Hard constraints for this repository. The general conventions (orientation, tests,
scripting, releasing) are in the `rust-in-projects` skill; this file holds only what is
specific to tui-file-explorer.

* **`lib.rs` holds only** `pub mod`, `pub use` and crate docs.
* **Binary-only code** stays out of the library: the two-pane app,
  `Editor`, CLI flags and persistence.
* **Shell code:** `shell_init.rs` owns shell detection and rc-file writes.
  `main.rs` prints the post-TUI "source your rc file" notice; `shell_init.rs`
  never does.
* **Private items:** macros (`handle_input_mode!`, `render_input_footer!`) stay
  module-private; `scroll_offset` and `status` stay private.
* **Colours** come only from `palette.rs`.
* **Tests** cover every `ExplorerOutcome` and `Modal` variant.
