# AIO Plugin Platform

This repository now treats built-in business features as plugin capsules.

See `docs/architecture.md` for the architecture entrypoint, stack boundary and minimum verification commands.

## Current Scope

- `system-capsules/` describes immutable platform features such as dashboard, auth, RBAC, dictionary and common action permissions.
- `plugins/asset-suite/` describes the asset parent plugin and its child plugins.
- Rust now discovers those formulas from the filesystem at startup, compiles menu permissions, command metadata, settings, resources, capabilities, extension points and plugin registry snapshots, and keeps the existing command implementations as the execution providers.
- `platform.event-bus` now exists as a system capsule, and the Rust host keeps an audited event bus with publish/subscribe history, request/reply support, correlated stream chunks and runtime event snapshots.
- `platform.runtime` now exists as a system capsule, and the Rust host keeps an audited runtime snapshot with start/stop/reload/workspace/session lifecycle records that are also published to the event bus.
- `platform.permission-core` now exists as a system capsule. The Rust host evaluates runtime permission requests with deny-by-default checks for source identity, declared capability, platform support and granted consent, then records permission decisions in an authenticated audit log.
- Permission consent is now persisted in SQLite under `permission_consents`; runtime checks compare requested scope against declared scopes and stored consent before broker execution. Built-in system capability consent is seeded for existing enabled users, while plugin capabilities remain deny-by-default until explicitly granted.
- The registry page exposes current user consent records and can grant or revoke consent by source, capability and scope. Capability and audit rows can prefill the grant form.
- Missing runtime consent now creates a pending record in `permission_approval_requests`. The registry page can approve or deny those requests; approval writes the matching consent record, while denial keeps the capability blocked.
- `platform.permission-core` also exposes `permission.policy`, `permission.approvalUI` and `permission.auditSink` extension points so policy, approval UI and audit sink plugins can extend the base without replacing it. System capsules can now act as extension-tree parents.
- Permission policy plugins can now contribute manifest-declared `deny` or `warn` policies through `contributes.policies`. Runtime broker authorization loads the current registry policy chain and applies deny policies after built-in manifest, platform and consent checks, so policies can narrow but not broaden access.
- `plugins/governance/audit-file-sink/` demonstrates a child audit sink mounted under Permission Core, `plugins/governance/approval-workbench/` demonstrates a child approval UI contribution, and `plugins/governance/high-risk-deny-policy/` demonstrates a child policy plugin that blocks high-risk process and filesystem-write targets.
- `plugins/asset-suite/children/openai-assistant/` demonstrates the AI workflow sample: a child plugin that previews page context and invokes the assistant-backed chat path through the existing command bridge.
- `platform.capability-broker` now exists as a system capsule, and the broker exposes audited `browser.openDirectory`, `fs.read`, `fs.write`, allowlisted `process.exec`, `browser.openUrl`, clipboard read/write and notification send providers through both typed commands and a unified `capability_invoke` JSON dispatch path.
- Capability provider registration is now a first-class registry contribution. Formulas and system capsules can declare `contributes.capabilityProviders`, each provider records capability, kind, platform matrix, entry, fallback and `trustLevel`, and the registry emits diagnostics when a native provider is not `trusted-provider` or `platform`.
- Capability Broker commands, including `open_data_dir` and `capability_invoke`, now pass through Permission Core before native execution.
- `plugins/platform-samples/macos-clipboard/` and `plugins/platform-samples/windows-notification/` provide platform-specific sample plugins that demonstrate macOS clipboard and Windows notification capability routing.
- `plugins/platform-samples/macos-automation/` and `plugins/platform-samples/windows-registry/` cover the Phase 12 macOSOnly automation and winOnly Registry samples as guarded provider descriptors; they prove platform matrix and approval fallback metadata without enabling high-risk native execution by default.
- Built-in provider coverage includes macOS/Windows/Linux native clipboard paths, macOS/Windows/Linux notification paths, and Web clipboard/notification provider metadata with browser API fallbacks when no desktop command bridge is available.
- `plugins/platform-samples/web-fallback/` provides a read-only Web/Remote fallback sample that renders the same declarative view contract without requesting native providers.
- The local registry under `data_dir/plugin-registry/` now persists `installed.json`, `lock.json`, `audit.jsonl` and `child-capability-approvals.json`, and the runtime snapshot merges enabled external plugin directories from there.
- The local registry now also keeps immutable version history under `history/`, signature placeholders under `signatures/`, and a `remote-registry.protocol.json` draft for the future remote registry contract.
- Rust and the web bridge now expose install, enable, disable, uninstall, rollback, child capability approve/revoke and local-state commands for that registry. Disabling an installed parent plugin now recursively disables its installed child subtree, and a child plugin cannot be re-enabled while its local parent is disabled.
- The plugin registry page now shows both the built-in snapshot and the local registry state.
- The plugin registry page now shows compiled permission policies, pending permission approvals and persisted permission consent records.
- The plugin registry page now exposes the draft Verification Runner so a generated plugin capsule can be checked from the Web/Tauri workbench and can write `diagnostics.json` plus `verification.json`.
- `aio-plugin verify-draft --write` also writes a deterministic `ui-preview.formula-view-preview.dom.html` file and references it from `diagnostics.uiPreview.domSnapshots`, so UI preview evidence works in CLI, Web, Desktop and CI contexts without requiring screenshot infrastructure.
- The plugin registry page now exposes the publish gate and rollback controls. The publish gate writes package-level `lock.json`, `signature.json`, `audit.jsonl`, `verification.json` and checks the remote registry protocol draft before `publish-local` installs the package.
- A `reload` hot path is available from Rust, the web bridge and the registry page.
- The repository also includes `schemas/plugin-formula.v1.schema.json`, `schemas/system-capsule.v1.schema.json`, `schemas/plugin-draft.v1.schema.json`, `schemas/plugin-diagnostics.v1.schema.json`, `schemas/plugin-publish-gate.v1.schema.json`, `schemas/plugin-signature-placeholder.v1.schema.json` and `schemas/remote-registry-protocol.v0.schema.json` plus a small `aio-plugin` CLI for validate/inspect/compile/migrate flows.
- `platform.plugin-factory` now exists as a system capsule, `aio-plugin create-from-prompt` can generate a `plugin-draft/v1` bundle with `formula.json`, `permission-plan.md`, `context-pack.md`, `PLAN.md`, `README.md`, `src/index.ts` and `tests/smoke.test.ts`, `aio-plugin verify-draft --write` can generate `diagnostics.json` and `verification.json`, `aio-plugin publish-gate` can generate publish evidence, `aio-plugin publish-local` runs that gate before installing into the local registry, `aio-plugin rollback-local` can restore a prior content hash, and `aio-plugin repair-from-diagnostics` can apply formula-first repairs from `diagnostics.json`.
- `platform.extension-host` now exists as a system capsule, and the Rust host exposes lifecycle operations for `load`, `activate`, `deactivate`, `reload`, `dispose` and `snapshot`. The host supervisor validates local Node/Worker/Browser entry files, executes trusted Node and Worker lifecycle hooks, calls Remote/Container HTTP lifecycle endpoints declared by `entry.remote`, and tracks runtime state/logs.
- Event declarations now carry per-event schema IDs. Formula `events` entries accept either plain event names or `{ event, schema }` objects, and the registry records the schema alongside each publish/subscribe declaration.
- Extension Host lifecycle contexts now receive `context.api` with `host.describe()`, `host.snapshot()`, `capabilities.invoke()`, `events.publish()`, `events.request()` and `events.stream()`. The lifecycle runner records capability and event requests for traceability; sensitive native execution still remains behind Permission Core and Capability Broker.
- The registry snapshot now includes an `extensionTree` model with plugin tree nodes and mount records. It compiles parent chains, extension point mounts, merged child contributions, effective child capabilities and capability-escalation diagnostics for child plugins; persisted child-capability approvals turn approved escalations into effective capabilities.
- `plugins/git-suite/` and `plugins/git-suite/children/github-provider/` provide the first non-asset parent/child plugin tree example: Git Suite exposes remote-provider and commit-policy extension points, while GitHub Provider mounts into the remote-provider point with audited `network.fetch` and `browser.openUrl` capabilities.
- `git-suite` and `asset.openai-assistant` now register tool contributions as well as commands and views, so the MVP command/tool/view registration path is non-empty and visible in the registry snapshot.
- Command, tool and view `when` conditions are compiled into registry snapshots and shown in the plugin registry UI.
- Route contributions and settings contributions are compiled into the registry snapshot as first-class entries; Git Suite now provides a concrete settings schema and route record so these registries are non-empty.
- The plugin registry page includes a thin Web/Desktop renderer adapter preview for `summary-list`, `detail`, `form`, `table`, `tree`, `graph`, `timeline`, `markdown` and `wizard` view contracts. Tauri desktop uses the same Vue adapter, and `@vben/plugins/view-renderer` exposes a plain-text adapter for future CLI/TUI output.

## Stack Decision

Use TypeScript plus Rust.

- TypeScript owns the formula contract, frontend types, registry inspection utilities and future Node/Web plugin host code.
- Rust owns the Tauri host, SQLite-backed platform state, Permission Core, Capability Broker, native providers and audited command execution.

Pure TypeScript is not the right default for this app because the existing product already depends on local native capabilities, SQLite migrations and Tauri command boundaries.
