# AIO Plugin Platform Architecture

This document is the architecture entrypoint for the AIO plugin platform. It tracks the implemented direction from the full plugin platform plan and points to the concrete contracts, runtime modules and verification commands in this repository.

## Stack Decision

AIO uses TypeScript plus Rust.

- TypeScript owns plugin contracts, frontend API bindings, declarative view models, Web/Desktop renderer adapters and future Node/Web plugin host code.
- Rust owns the Tauri host, SQLite-backed platform state, App Runtime, registry compiler, Extension Host supervisor, Permission Core, Capability Broker, native providers, audit records and CLI automation.

Pure TypeScript is not the default because this product needs local filesystem, process, clipboard, notification, SQLite and Tauri command boundaries. Rust keeps those sensitive operations behind auditable platform services.

## Runtime Layers

The platform is layered so ordinary plugins never call native services directly.

```text
Shells
  Web / Tauri Desktop / CLI-TUI adapters
    -> Registry Hub
      -> Extension Host
        -> Stable Plugin API
          -> Permission Core
            -> Capability Broker
              -> Platform Services / Native Providers
```

## Core Contracts

- `schemas/plugin-formula.v1.schema.json` is the source of truth for ordinary and child plugin formulas.
- `schemas/system-capsule.v1.schema.json` gives platform modules metadata parity without making Permission Core, Capability Broker, Registry Hub, Event Bus or Extension Host ordinary uninstallable plugins.
- `schemas/plugin-ui-view.v1.schema.json` defines the declarative UI contract rendered by Web/Desktop and CLI/TUI adapters.
- `schemas/plugin-diagnostics.v1.schema.json` defines verification diagnostics, including deterministic UI DOM snapshots for CI-friendly preview evidence.
- `schemas/events/platform-event-envelope.v1.schema.json` defines the default event envelope, while `schemas/events/platform-runtime-lifecycle.v1.schema.json` and `schemas/events/registry-reloaded.v1.schema.json` cover the runtime and registry lifecycle records emitted by the core.

## Registry Hub

Rust discovers `plugins/` and `system-capsules/`, validates formulas, then compiles a registry snapshot with:

- commands, views, menus, permissions and capabilities;
- tools, settings, resources and extension points;
- capability providers with platform matrix and trust level diagnostics;
- extension-tree nodes, mounts, parent chains and child capability escalation diagnostics;
- policy records, events, system capsules and plugin summaries.

The Web/Tauri registry page renders this snapshot and exposes local registry state, runtime lifecycle, permission consent, approval requests, publish gate and rollback flows.

## Extension Host

`platform.extension-host` supervises Node, Worker/Browser and Remote lifecycle entries. Each lifecycle receives a stable `context.api` with:

- `host.describe()` and `host.snapshot()`;
- `capabilities.invoke()`;
- `events.publish()`, `events.request()` and `events.stream()`.

The lifecycle runner records capability and event requests for traceability. Sensitive native execution still goes through Permission Core and Capability Broker.

## Permission And Capability Boundary

Permission Core is platform-owned and deny-by-default. It validates source identity, declared capabilities, scopes, platform support, user consent and manifest-declared policy plugins. Policy plugins may narrow access but cannot broaden it.

Capability Broker is the only sensitive native execution path. It provides audited dispatch for filesystem, process, browser URL, clipboard and notification capabilities, and records traceable audit outcomes for allowed, denied, pending and failed requests.

Clipboard and notification provider coverage is explicit across shells: macOS/Windows/Linux use native command providers where available, while Web uses browser-scoped `navigator.clipboard` and `Notification` fallbacks when the desktop command bridge is not present.

## Plugin Tree

Parent plugins declare extension points. Child plugins mount through `parent.mount`, keep their own identity, and contribute commands, views, menus, capabilities and providers with parent-chain traceability.

Child plugins do not inherit parent permissions automatically. New capabilities outside the parent policy require explicit platform approval, and disabling an installed parent disables its child subtree.

## Online Bootstrap

`platform.plugin-factory` implements the natural-language-to-plugin loop:

```text
prompt
  -> formula draft
  -> permission plan
  -> capsule files
  -> smoke test
  -> verification diagnostics
  -> DOM preview snapshot
  -> repair loop
  -> publish gate
  -> local registry install
```

Generated capsules include `formula.json`, `src/index.ts`, `permission-plan.md`, `context-pack.md`, `PLAN.md`, `README.md` and `tests/smoke.test.ts`.

## Package And Registry Flow

The local plugin registry under the app data directory persists:

- `installed.json`;
- `lock.json`;
- `audit.jsonl`;
- `child-capability-approvals.json`;
- immutable `history/`;
- placeholder `signatures/`;
- `remote-registry.protocol.json` for the future remote registry contract.

The publish gate writes verification, lock, signature and audit evidence before local install. Rollback restores a previous content hash from version history.

## Verification Commands

Use these as the minimum implementation gate:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo run --manifest-path src-tauri/Cargo.toml --bin aio-plugin -- validate
npm_config_cache=/private/tmp/aio-npm-cache npx --yes pnpm@9.15.1 -F @vben/web-ele run typecheck
```

For bootstrap evidence, generate and verify a draft:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin aio-plugin -- create-from-prompt \
  --prompt "Generate a table view plugin" \
  --id workspace.example \
  --display-name "Example" \
  --output-dir /private/tmp/aio-plugin-example \
  --force

cargo run --manifest-path src-tauri/Cargo.toml --bin aio-plugin -- verify-draft \
  --source-path /private/tmp/aio-plugin-example \
  --write
```

The second command writes `diagnostics.json`, `verification.json` and `ui-preview.formula-view-preview.dom.html`.
