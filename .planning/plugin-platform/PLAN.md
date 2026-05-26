# Plugin Platform Plan

## Current Slice

- [x] Load built-in plugin formulas and system capsules from the filesystem.
- [x] Validate `plugin-formula/v1` and `system-capsule/v1` with schema files.
- [x] Expose registry snapshots, capabilities, and extension points to the UI.
- [x] Compile settings and resources as first-class registry contributions.
- [x] Keep existing business commands as the execution layer behind the registry.
- [x] Add formula/system-capsule manifest migration CLI.
- [x] Add architecture entrypoint synchronized with the implementation.
- [x] Add App Runtime lifecycle snapshot and commands.
- [x] Add event schema declarations and validation for runtime and registry events.

## Next Slice

- [x] Add plugin factory create/publish/repair bootstrap loop.
- [x] Add Node/Worker/Remote Extension Host executors and host-cycle validation.
- [x] Add stable Plugin API injection for Extension Host lifecycle contexts.
- [x] Add first-class Extension Tree Registry with mount and capability policy diagnostics.
- [x] Add first-class route contributions and a non-empty settings registry sample.
- [x] Add Event Bus stream events for correlated long-running output chunks.
- [x] Add manifest validation and policy diagnostics for platform compatibility.
- [x] Add hot-load/install/reload flows for external plugins.
- [x] Add permission broker and capability broker primitives.
- [x] Add Permission Core runtime decision records, deny-by-default checks and authenticated audit access.
- [x] Add persisted user consent records and runtime capability scope checks.
- [x] Add Permission Consent UI for listing, granting and revoking runtime consent records.
- [x] Add runtime approval requests for missing consent and a registry UI approve/deny flow.
- [x] Allow system capsules to expose extension points for policy, approval UI and audit sink plugins.
- [x] Add manifest-declared Permission Core policy chain and high-risk deny policy sample.
- [x] Add persisted approval for child plugin capability escalations.
- [x] Add MVP file/process/browser/clipboard/notification capability providers with audit records.
- [x] Add first-class capability provider registration with trust-level diagnostics.
- [x] Add sample macOS-only and Windows-only capability examples.
- [x] Add Phase 12 macOSOnly automation and winOnly Registry guarded sample plugins.
- [x] Add Git and AI tool contributions so sample plugins register command/tool/view.
- [x] Add Linux/Web clipboard and Windows notification provider coverage.
- [x] Add a read-only Web fallback sample plugin for native-provider-unavailable hosts.
- [x] Add declarative UI contract types for summary/detail/form/table/tree/graph/timeline/markdown/wizard.
- [x] Add Web/Tauri renderer preview and plain-text renderer adapter for CLI/TUI consumers.
- [x] Add verification DOM snapshot output for UI previews.
- [x] Compile command/tool/view `when` conditions into snapshots and show them in the registry UI.
- [x] Add publish gate evidence files, signature placeholders, registry version history, rollback and remote registry protocol draft.
- [x] Add a Permission Core audit sink child-plugin sample.
- [x] Add a Permission Core approval UI child-plugin sample.
- [x] Enforce local parent/child plugin enable lifecycle with cascade disable.
