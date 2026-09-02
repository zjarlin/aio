# Refactor One Path

Refactor the single file or directory path supplied after this command. Resolve `~` and use the absolute path. Preserve observable behavior, inspect the complete feature boundary, and leave the repository buildable.

## Process

1. Read every applicable `AGENTS.md`, inspect `git status`, and preserve existing edits.
2. Inspect callers, callees, source roots, manifests, generated boundaries, registrations, documentation, and focused tests.
3. Run a smallest practical baseline check before editing.
4. Organize by feature first, then explicit responsibilities. Update all repository callers and configuration references directly.
5. Remove only proven-unused code. Do not add deprecated wrappers, forwarding shells, aliases, or compatibility layers.
6. Keep transport, models, services, implementations, adapters, assistants, constants, and enums in their proper role boundaries. Extract an SPI only for a real cross-boundary extension contract.
7. Verify focused behavior, formatting, affected builds, stale references, and `git diff --check`.

Use the repository's own architecture as the authority. Report pre-existing failures and functional defects separately from structural changes. Ask for clarification only when the supplied path is missing, unreadable, or ambiguous.
