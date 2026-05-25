# Plugin Platform Plan

## Current Slice

- [x] Load built-in plugin formulas and system capsules from the filesystem.
- [x] Validate `plugin-formula/v1` and `system-capsule/v1` with schema files.
- [x] Expose registry snapshots, capabilities, and extension points to the UI.
- [x] Keep existing business commands as the execution layer behind the registry.

## Next Slice

- [x] Add plugin factory create/publish/repair bootstrap loop.
- [x] Add Node/Worker/Remote Extension Host executors and host-cycle validation.
- [x] Add first-class Extension Tree Registry with mount and capability policy diagnostics.
- [x] Add manifest validation and policy diagnostics for platform compatibility.
- [x] Add hot-load/install/reload flows for external plugins.
- [x] Add permission broker and capability broker primitives.
- [x] Add Permission Core runtime decision records, deny-by-default checks and authenticated audit access.
- [x] Add persisted user consent records and runtime capability scope checks.
- [x] Allow system capsules to expose extension points for policy, approval UI and audit sink plugins.
- [x] Add MVP file/process/browser/clipboard/notification capability providers with audit records.
- [x] Add sample macOS-only and Windows-only capability examples.
- [x] Add declarative UI contract types for summary/detail/form/table/tree/graph/timeline/markdown/wizard.
- [x] Add Web/Tauri renderer preview and plain-text renderer adapter for CLI/TUI consumers.
- [x] Compile command/tool/view `when` conditions into snapshots and show them in the registry UI.
- [x] Add publish gate evidence files, signature placeholders, registry version history, rollback and remote registry protocol draft.
- [x] Add a Permission Core audit sink child-plugin sample.
