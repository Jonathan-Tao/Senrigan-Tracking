# Senrigan agent rules

Senrigan is a local one-camera VTuber tracker. See [`README.md`](README.md) for
the product and [`misc-planning/file_architecture.md`](misc-planning/file_architecture.md)
for the stack, layout, owned numbers, and gates.

This is a public repository under GPL-3.0-only. It is a hobby and
research-engineering effort by one maintainer. Outside readers can read every
line, so write for a stranger. Do not write for a compliance process that does
not exist.

## Build and checks

- Run tasks through `just`. Run `just` to list the targets.
- Run `just ci` before you report work as done. It runs formatting, check,
  tests, and lint.
- Keep `cargo clippy --workspace --all-targets -- -D warnings` clean. Fix the
  code instead of adding an `allow` attribute. Justify an unavoidable `allow` on
  the line above it.
- Format Rust with `cargo fmt`. Do not hand-format around the formatter.
- Run Python model and evaluation work through `uv`. Ordinary users never run
  it. The setup UI is Rust and builds with the rest of the workspace.
- Ordinary checks must pass without a GPU. Hardware and provider runs are
  manual. Put their machine-readable results under `benchmarks/`.

## Code and tests

- Use the smallest clear implementation. Prefer simple code that a new reader
  can debug over layered robustness. Add a file, type, trait, wrapper, or helper
  only for multiple callers or a net reduction in production complexity.
- A comment that defends the shape of the code is a defect report. Rewrite the
  code until the comment is unnecessary, then delete the comment. Comments state
  intent, units, frames, ownership, and timing, never excuses for a workaround.
- Justify every complex construct in seven words or less. State the reason, for
  example "external data, untrusted" or "output clock, measured". Delete the
  construct when no such reason exists.
- Write the bare minimum code that supports a current feature. Delete the rest.
- Change only what the task requires. Do not reformat or reorder untouched code.
  Remove replaced code instead of retaining dead paths. Keep a maintained line
  only when it builds or runs the live path, replay, export, evaluation, or
  packaging path; enforces an artifact or data contract; tests one of those
  paths under these rules; or documents a user-facing contract the code
  implements.
- Do not add helpers, fields, parameters, arguments, or dependencies without a
  current caller. Historical experiments may remain as notes and compact results
  without running against current APIs. Do not retain compatibility shims for
  them.
- Keep the bare minimum test set. Write a test only when its failure would ship
  broken motion, a wrong published number, or a corrupt artifact, and no
  end-to-end replay gate already catches it. Delete every other test.
- Test only a wanted observable behavior or outcome, a runtime or artifact
  contract, generation output, or a non-trivial algorithm. Do not write
  regression tests. Do not preserve behavior only because it existed or a bug
  exposed it. Do not test parameters, defaults, developer constants, fixture
  contents, incidental values, config echoes, internal call sequences,
  implementation structure, or expected-to-change details. Remove a test when
  normal tuning or refactoring should change its expected value. Do not enlarge
  production code or expose internals for tests.
- Test the boundary a value can cross, never the constructor that guards it. A
  validated type cannot hold an invalid value, so asserting that its
  constructor rejects one tests an `if` statement. Test the paths that build
  the value without the constructor instead: `Deserialize` impls, wire
  decoding, and arithmetic that returns a new value. A derive that silently
  replaces a checked impl must fail a test.
- Do not restate a constant in an assertion. A test that repeats a constant's
  literal value fails when someone edits either copy and cannot say which one
  is right.
- A contract landing still owes the test that
  `misc-planning/implementation_landings.md` rule 8 requires. Satisfy it with a
  decode test, or with an invariant a hand-written table can plausibly violate,
  such as a duplicate or overlapping entry.
- Trust owned contracts, configuration, and schemas inside the workspace. Do not
  add validation, recovery, abstractions, or compatibility layers for
  hypothetical users or misuse. Validate camera data, model files, VRM files,
  receiver configuration, and any value that could make indexing, memory access,
  or arithmetic unsafe. Let a fixed startup contract mismatch fail at startup.
- Experiment code never ships on maintained surfaces. It gets no `just` target,
  no crate module, no UI control, and no command-line verb. Keep it in an
  `experiments-scratch/` directory in the owning tool. When an experiment wins,
  promote it by replacing the old path, not by adding a second one.
- Add a `just` target only for a command run constantly. Document setup,
  research, and one-off commands in the owning README instead.
- Do not add a plugin system, daemon boundary, network interface, or inference
  trait until a real second implementation or a deterministic test double needs
  it.

## Reviews and audits

Apply these rules when you report problems instead of fixing them.

- Report a problem only when you can name what it breaks: motion the performer
  sees, a replay result, a published number, or an artifact a user loads. A
  difference on its own is not a problem.
- State the mechanism, then check it. Read the data structure, the call site,
  and the session duration before you claim that work, memory, or error grows.
- Read the code at HEAD before you report a missing check. Confirm that the
  guard is absent and that no caller already supplies it.
- Run each gate through its own `just` target. A failure produced by another
  toolchain is not a finding.
- State what each fix costs. Name the re-record, the re-export, the retrain, or
  the recalibration when the fix needs one.
- Rank findings by the time they cost. Do not mark every finding critical.
- Prefer ten findings that pay for their fix over ninety that do not.
- Do not report license text, dependency manifest accuracy, hypothetical misuse,
  or tests that this document bans.

## Prose

Apply these ASD-STE100-style rules to all maintained documents:

- Lead with the action or conclusion. Use one name per thing and one meaning per
  word. Do not rotate synonyms.
- Prefer short, common words. Remove filler, hype, marketing adjectives, asides,
  empty framing, and unexplained emphasis. Replace vague claims with measurements
  or constraints.
- Use active voice and direct verbs. Avoid stacked auxiliaries and `-ing` main
  verbs when a simple tense works.
- Give one instruction per sentence. Keep sentences short and direct.
- Use complete grammar. Do not use contractions or semicolons. Do not omit
  articles or demonstratives.
- Use the imperative for instructions. Put conditions before commands.
- Do not alter code or literal technical strings unless requested.
- Return only requested document text without a preamble, summary, or closing
  remark.
- Define acronyms once. Keep units and dates. Separate facts from recommendations.
- Link to the document that owns a command or capability instead of repeating it.
- Do not restate code-owned values such as defaults, thresholds, or counts. State
  the command or link to the source.
- Use the glossary names in `misc-planning/overall_ideal.md`. Do not rotate
  performer, site, spatial model, first useful release, or rotation and IK
  baseline.
- Put new numeric gates only in `misc-planning/file_architecture.md`.
- State what the project does not solve. Depth ambiguity, occlusion, and hard
  cases stay in the documents. Do not remove a stated limitation to make the
  project sound finished.

## READMEs

- Write for a new user who found the repository today. Explain each command's
  purpose, timing, prerequisites, and required context.
- Lead with purpose, prerequisites, setup, and common use. Put architecture and
  implementation details later.
- Include only information needed to use, configure, troubleshoot, or understand
  the software. Link to the README that owns a workflow instead of repeating it.
- Document data flow, message and schema types, rates, clocks, coordinate
  frames, and units. Include dated, measured tuning or benchmark results when
  they guide operation or establish performance.
- Name the hardware, operating system, and camera used for any reported number.

## Rust

- Target the workspace `edition` and `rust-version` in the root `Cargo.toml`. Do
  not raise either one as a side effect of another change.
- Follow standard Rust naming. `cargo fmt` and Clippy own layout and idiom.
- Use fixed-width integers for wire formats, storage, hardware, and values whose
  width matters. Use `usize` for indices and lengths.
- Do not use `unsafe`. When a foreign function interface or a measured hot path
  forces it, keep the block minimal, state the invariant in a `// SAFETY:`
  comment, and give the caller a safe wrapper.
- Return `Result` with a `thiserror` type from library crates. Keep failures
  typed and distinguishable. Do not collapse distinct causes into one string.
- Do not `unwrap`, `expect`, panic, or index without a bound on the capture,
  estimator, retarget, or output path. Reserve those for tests and for startup
  code where the failure is a contract mismatch.
- Put values that must stay finite or in range behind the validated types in
  `crates/types`. Do not pass raw `f32` across a crate boundary when a validated
  type exists.
- Name the destination frame before the source frame in transforms. State the
  frame and the unit in the name or in a comment.
- Avoid mutable global state. Own state in one place. The estimator corrects
  root and contact state once, and later stages consume it.
- Give each crate and module a `//!` doc comment that states what it owns. Give
  public items a `///` doc comment when intent or edge behavior is not clear.
  Document errors, empty results, and degenerate results.
- Use rustdoc, not Doxygen. Do not write `@file`, `@brief`, `@author`, or
  `@date` banners. Do not write `@param` or `@return` tags. Use the rustdoc
  sections `# Errors`, `# Panics`, and `# Safety` when they apply. Git owns
  authorship and dates. The signature owns parameter names and types.
- Order every file so a new reader meets it top down. Put the most user-facing
  type or function first, then what it calls, then that layer's dependencies.
  The deepest helper ends up last. Order items public before private.
- Name repeated or opaque literals. Add a unit comment only when the name and
  type do not convey the unit. Add provenance only when it is known and relevant.
- Use numbered dividers (`// 1. ...`) in long functions.
- The license is declared once in the workspace `Cargo.toml`. Do not add
  per-file license banners.

## Repository boundaries

- Respect the crate dependency direction `types` ← (`engine`, `perception`,
  `avatar`) ← `app`. `avatar` uses shared math and contracts from `types`. It
  does not call capture or the user interface.
- Do not add a crate until build times or dependencies demonstrate a useful
  boundary. Module boundaries inside a crate are enough before then.
- Keep the engine as the owner of session state. The `iced` view holds a
  read-only snapshot. The UI never blocks the output clock and never sits
  between the estimator and an output adapter.
- Do not commit build artifacts, model weights, datasets, caches, credentials,
  or generated experiment outputs. Keep notes and compact results. Ship a compact
  runtime manifest beside each artifact.
- Do not commit webcam footage of a person without recorded consent. Keep
  committed replays small and documented. Reference larger or private suites by
  a stable local dataset identifier.
- Every model configuration identifies its checkpoint, source, terms, and
  restrictions. A research-only artifact can serve a permitted local experiment.
  It cannot enter a public bundle unless its terms permit distribution and the
  intended use.
- Third-party code, model weights, datasets, and avatar files keep their own
  terms. Leave upstream license headers and notices in place. Do not audit
  external license metadata, versions, or manifests.
