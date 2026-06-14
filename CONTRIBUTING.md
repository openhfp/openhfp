# Contributing to OpenHFP

Thanks for your interest. OpenHFP is in early design; the most valuable contributions
right now are review of the specification and the de-risk spikes (see the roadmap in the
[README](README.md)).

## Ground rules

- **Language: English only.** All code, comments, commit messages, issues, PRs and
  documentation in this repository are in English. Localized end-user documentation, if
  introduced, lives in language-specific folders and never replaces the English source.
- **The two load-bearing principles** (see README) are the acceptance test for any change:
  attribution-not-security, and open HTML/CSS/JS with no UX limits. A change that violates
  either does not belong in v1.
- **The spec and the reference implementation must agree.** The conformance corpus
  (`conformance/`) is the contract; CI must stay green.

## Development

This is a monorepo with two workspaces:

- **Rust** (`crates/`): `cargo build`, `cargo test`, `cargo fmt`, `cargo clippy`.
- **JS/TS** (`packages/`): npm workspaces — `npm install`, then per-package scripts.

## Commits and PRs

- Keep history clean and readable; one logical change per commit.
- Reference the relevant spec section or roadmap phase where it helps.
- Sign-off is not required, but commits should be attributable.

## License

By contributing you agree that your contributions are licensed under the
[MIT License](LICENSE).
