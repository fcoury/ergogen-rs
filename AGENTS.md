# AGENTS.md

Project-specific instructions for agents working in this repo.

## Commit Message Guidelines

- Use Angular-style commit messages (e.g., `feat: ...`, `chore: ...`) and include a short detail summary in the body.

## Code hygene

- After each feature implementation, run `cargo check`, `cargo fmt --all` and `cargo clippy`.
  - Fix all the reported issues.
- Ensure all new and existing tests pass with `cargo test`.
