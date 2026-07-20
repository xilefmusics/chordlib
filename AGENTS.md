## AI Agent Quality Gates

These rules are **mandatory** for any AI agent changing this repo.

### Required checks (definition of done)

- **Formatting**: Format edited Rust code with `cargo fmt --all`, then run
  `cargo fmt --all -- --check`.

- **Clippy clean**: Run `cargo clippy --all-features -- -D warnings`.
  - Code must build with **zero Clippy warnings**.
  - If some pre‑existing/externally caused warnings remain, minimize new ones and document what’s left and why.

- **Tests pass**: Run `cargo test --verbose`.
  - If anything fails, either fix and re-run, or list failing tests and why they weren’t fixed.

- **Type check**: `cargo check` (or equivalent CI check) must succeed.

- **Docs build**: `cargo doc --no-deps` must succeed for modified crates.

### When touching dependencies or critical code

- **Dependencies**: If you change `Cargo.toml` / `Cargo.lock`, prefer to run:
  - `cargo audit` (vulns) and, if configured, `cargo deny` (policy).
  - If they report issues you can’t fix, document which remain and why.
- **No unused deps**: Don’t leave unused crates in `Cargo.toml`. If `cargo udeps` is used, don’t add new unused deps (or explain clearly why they remain).
- **Unsafe / perf‑sensitive code**:
  - Where already used in this repo, prefer `cargo miri test` for `unsafe` and `cargo bench` (or project benchmarks) for perf‑critical changes. Avoid regressions.

These extra tools are not required for every tiny change, but **never break existing CI/tooling** and prefer to run them when working in their area.

GitHub Actions CI runs the following Cargo commands, in this order, on every
push and pull request targeting `main`. Agents must run all three exactly:

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --verbose
```

### What every agent must report

In the final message for any task, explicitly state:

- Whether tests were run and passed.
- Whether Clippy was run and produced zero warnings.
- Whether any additional Rust checks above were run, and if not, why they weren’t needed.

### Commit message rules (strict)

All commits to this repo **must** use this format:

- **Subject line**:
  - Format: `type(scope): short summary`
  - `type` (one of): `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `chore`, `build`, `ci`.
  - `scope` is optional but recommended; use a short, lowercase identifier (e.g. `inputs`, `html`, `types`, `cli`).
  - Summary in **imperative mood**, max ~72 characters, no trailing period.

- **Body (optional but recommended)**:
  - Wrap at ~72 columns.
  - Explain **what** and especially **why**, not just restating the diff.
  - Use paragraphs or bullet points; leave one blank line after the subject.

- **Footer (optional)**:
  - Use for issue references and breaking changes.
  - Examples:
    - `Refs #123` / `Fixes #123`
    - `BREAKING CHANGE: description of the breaking behavior`

- **General rules**:
  - English only; no emoji in the subject.
  - One logical change per commit where practical.
  - Do **not** bypass hooks; if a hook fails, fix the issue and re-commit instead of forcing.
