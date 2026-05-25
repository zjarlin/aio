# AIO Plugin Platform

This repository now treats built-in business features as plugin capsules.

## Current Scope

- `system-capsules/` describes immutable platform features such as dashboard, auth, RBAC, dictionary and common action permissions.
- `plugins/asset-suite/` describes the asset parent plugin and its child plugins.
- Rust now discovers those formulas from the filesystem at startup, compiles menu permissions, command metadata, capabilities, extension points and plugin registry snapshots, and keeps the existing command implementations as the execution providers.
- `platform.event-bus` now exists as a system capsule, and the Rust host keeps an audited event bus with publish/subscribe history, request/reply support and runtime event snapshots.
- `platform.permission-core` now exists as a system capsule. The Rust host evaluates runtime permission requests with deny-by-default checks for source identity, declared capability, platform support and granted consent, then records permission decisions in an authenticated audit log.
- Permission consent is now persisted in SQLite under `permission_consents`; runtime checks compare requested scope against declared scopes and stored consent before broker execution. Built-in system capability consent is seeded for existing enabled users, while plugin capabilities remain deny-by-default until explicitly granted.
- `platform.permission-core` also exposes `permission.policy`, `permission.approvalUI` and `permission.auditSink` extension points so policy, approval UI and audit sink plugins can extend the base without replacing it. System capsules can now act as extension-tree parents, and `plugins/governance/audit-file-sink/` demonstrates a child audit sink mounted under Permission Core.
- `platform.capability-broker` now exists as a system capsule, and the broker exposes audited `browser.openDirectory`, `fs.read`, `fs.write`, allowlisted `process.exec`, `browser.openUrl`, clipboard read/write and notification send providers through both typed commands and a unified `capability_invoke` JSON dispatch path.
- Capability Broker commands, including `open_data_dir` and `capability_invoke`, now pass through Permission Core before native execution.
- `plugins/platform-samples/macos-clipboard/` and `plugins/platform-samples/windows-notification/` provide platform-specific sample plugins that demonstrate macOS clipboard and Windows notification capability routing.
- The local registry under `data_dir/plugin-registry/` now persists `installed.json`, `lock.json` and `audit.jsonl`, and the runtime snapshot merges enabled external plugin directories from there.
- The local registry now also keeps immutable version history under `history/`, signature placeholders under `signatures/`, and a `remote-registry.protocol.json` draft for the future remote registry contract.
- Rust and the web bridge now expose install, enable, disable, uninstall, rollback and local-state commands for that registry.
- The plugin registry page now shows both the built-in snapshot and the local registry state.
- The plugin registry page now exposes the draft Verification Runner so a generated plugin capsule can be checked from the Web/Tauri workbench and can write `diagnostics.json` plus `verification.json`.
- The plugin registry page now exposes the publish gate and rollback controls. The publish gate writes package-level `lock.json`, `signature.json`, `audit.jsonl`, `verification.json` and checks the remote registry protocol draft before `publish-local` installs the package.
- A `reload` hot path is available from Rust, the web bridge and the registry page.
- The repository also includes `schemas/plugin-formula.v1.schema.json`, `schemas/system-capsule.v1.schema.json`, `schemas/plugin-draft.v1.schema.json`, `schemas/plugin-diagnostics.v1.schema.json`, `schemas/plugin-publish-gate.v1.schema.json`, `schemas/plugin-signature-placeholder.v1.schema.json` and `schemas/remote-registry-protocol.v0.schema.json` plus a small `aio-plugin` CLI for validate/inspect/compile flows.
- `platform.plugin-factory` now exists as a system capsule, `aio-plugin create-from-prompt` can generate a `plugin-draft/v1` bundle with `formula.json`, `permission-plan.md`, `context-pack.md`, `PLAN.md`, `README.md`, `src/index.ts` and `tests/smoke.test.ts`, `aio-plugin verify-draft --write` can generate `diagnostics.json` and `verification.json`, `aio-plugin publish-gate` can generate publish evidence, `aio-plugin publish-local` runs that gate before installing into the local registry, `aio-plugin rollback-local` can restore a prior content hash, and `aio-plugin repair-from-diagnostics` can apply formula-first repairs from `diagnostics.json`.
- `platform.extension-host` now exists as a system capsule, and the Rust host exposes lifecycle operations for `load`, `activate`, `deactivate`, `reload`, `dispose` and `snapshot`. The host supervisor validates local Node/Worker/Browser entry files, executes trusted Node and Worker lifecycle hooks, calls Remote/Container HTTP lifecycle endpoints declared by `entry.remote`, and tracks runtime state/logs.
- The registry snapshot now includes an `extensionTree` model with plugin tree nodes and mount records. It compiles parent chains, extension point mounts, merged child contributions, effective child capabilities and capability-escalation diagnostics for child plugins.
- `plugins/git-suite/` and `plugins/git-suite/children/github-provider/` provide the first non-asset parent/child plugin tree example: Git Suite exposes remote-provider and commit-policy extension points, while GitHub Provider mounts into the remote-provider point with audited `network.fetch` and `browser.openUrl` capabilities.
- Command, tool and view `when` conditions are compiled into registry snapshots and shown in the plugin registry UI.
- The plugin registry page includes a thin Web/Desktop renderer adapter preview for `summary-list`, `detail`, `form`, `table`, `tree`, `graph`, `timeline`, `markdown` and `wizard` view contracts. Tauri desktop uses the same Vue adapter, and `@vben/plugins/view-renderer` exposes a plain-text adapter for future CLI/TUI output.

## Stack Decision

Use TypeScript plus Rust.

- TypeScript owns the formula contract, frontend types, registry inspection utilities and future Node/Web plugin host code.
- Rust owns the Tauri host, SQLite-backed platform state, Permission Core, Capability Broker, native providers and audited command execution.

Pure TypeScript is not the right default for this app because the existing product already depends on local native capabilities, SQLite migrations and Tauri command boundaries.
