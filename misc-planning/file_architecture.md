# Commodity-Camera Avatar Tracking

## Repository, API, Runtime-Layer, and Modular-ML Architecture

**Revision 1.2 — Rust-only companion software architecture and engineering-policy specification**  
**Status:** Proposed implementation architecture  
**Revision focus:** First-party Rust only, Cargo-first builds, a strict frontend/backend process boundary, modular ML, minimal current-feature code, outcome-based tests, isolated experiments, and explicit repository boundaries.  
**Companion documents:**

- `Commodity_Camera_Avatar_Tracking_Design_Person_Agnostic.md`
- `Commodity_Camera_Avatar_Tracking_Technical_Architecture_Research_Cited.md`

> **North star:** A user selects a quality mode and camera count, imports an avatar, performs a few guided poses, and gets convincing face, hand, and full-body animation. The product must work headlessly, on Windows and Linux, with inexpensive webcams, and without coupling its user interface to its tracking implementation.

> **Software-architecture thesis:** Treat the tracker as a headless realtime product with a versioned API, not as a desktop UI that happens to contain tracking code. Treat every learned component as a replaceable implementation of a semantic task contract, not as a tensor-shaped dependency that leaks through the system.

> **Scope of this document:** This specification defines repository layout, process topology, public APIs, internal interfaces, runtime packet contracts, state ownership, plugin boundaries, model packaging, training-code organization, testing, observability, and the rules that make the system composable. The companion technical architecture remains authoritative for pose-estimation algorithms, latency targets, calibration mathematics, fusion, temporal reasoning, contacts, face, hands, guitar tracking, and avatar retargeting.

> **Language decision:** All first-party shipping and runtime code is Rust. The project may call external C ABI libraries such as ONNX Runtime or operating-system APIs through narrowly owned Rust adapters, but it does not contain first-party C++ source, expose a C++ ABI, or require the frontend to link Rust backend crates. Cargo is the initial authoritative Rust build system.

> **Engineering-policy source:** The implementation rules in Sections 0, 3, 4, 20, 22, 23, 28, and 31 adapt the maintainer-provided repository philosophy to this public, cross-platform, ML-heavy project. They are project policy rather than research claims.

---

# Table of contents

- [0. Decision policy and architectural invariants](#0-decision-policy-and-architectural-invariants)
  - [0.2 Engineering operating policy](#02-engineering-operating-policy)
  - [0.3 Trust and validation model](#03-trust-and-validation-model)
- [1. Architecture at a glance](#1-architecture-at-a-glance)
- [2. Process topology](#2-process-topology)
- [3. Repository structure](#3-repository-structure)
- [4. Dependency rules and layer ownership](#4-dependency-rules-and-layer-ownership)
- [5. Public API architecture](#5-public-api-architecture)
- [6. Public resources and service functions](#6-public-resources-and-service-functions)
- [7. Event, telemetry, preview, and pose streams](#7-event-telemetry-preview-and-pose-streams)
- [8. Frontend architecture](#8-frontend-architecture)
- [9. Backend application layer](#9-backend-application-layer)
- [10. Realtime engine and pipeline graph](#10-realtime-engine-and-pipeline-graph)
- [11. Runtime packets and semantic contracts](#11-runtime-packets-and-semantic-contracts)
- [12. State ownership and persistence](#12-state-ownership-and-persistence)
- [13. Modular ML architecture](#13-modular-ml-architecture)
- [14. Model packages and model registry](#14-model-packages-and-model-registry)
- [15. Multi-camera composition](#15-multi-camera-composition)
- [16. Avatar import, profiling, and retargeting composition](#16-avatar-import-profiling-and-retargeting-composition)
- [17. OS- and GPU-neutral inference layer](#17-os--and-gpu-neutral-inference-layer)
- [18. Plugin architecture](#18-plugin-architecture)
- [19. Recording, deterministic replay, and debugging](#19-recording-deterministic-replay-and-debugging)
- [20. Training, synthetic data, and evaluation repository](#20-training-synthetic-data-and-evaluation-repository)
- [21. Configuration and quality profiles](#21-configuration-and-quality-profiles)
- [22. Build system, CI, packaging, and updates](#22-build-system-ci-packaging-and-updates)
- [23. Testing strategy](#23-testing-strategy)
- [24. Observability and latency attribution](#24-observability-and-latency-attribution)
- [25. Security, privacy, and trust boundaries](#25-security-privacy-and-trust-boundaries)
- [26. End-to-end workflows](#26-end-to-end-workflows)
- [27. Composability matrix](#27-composability-matrix)
- [28. Explicit anti-patterns](#28-explicit-anti-patterns)
- [29. Implementation sequence](#29-implementation-sequence)
- [30. Interface sketches](#30-interface-sketches)
- [31. Architecture-decision record policy](#31-architecture-decision-record-policy)
- [32. Reference ledger](#32-reference-ledger)

---

# 0. Decision policy and architectural invariants

Normative software decisions are labeled `RA-###`. Each cites a software-architecture paper, an ML-systems paper, a tracking paper from the companion architecture, or an official protocol specification. The citation supports the design principle; it does not guarantee that a particular library implementation will meet the product's latency or reliability targets. Project engineering rules in Sections 0.2 and 0.3 are maintainer policy and therefore do not receive paper citations.

## 0.1 Architectural invariants

The following rules are not temporary preferences. Early prototypes must preserve them so that prototype shortcuts do not become permanent coupling.

1. The backend is fully usable without a graphical frontend.
2. The frontend imports no backend Rust crate and calls no in-process backend object. It uses only the versioned public service contract.
3. The UI, CLI, tests, third-party clients, and optional remote controller use the same versioned application API.
4. The realtime output path never waits for the UI, preview renderer, API client, disk writer, or network consumer.
5. Learned modules communicate through canonical semantic observations. Model-specific latent tensors do not become public contracts.
6. Site State, Avatar Profile, Performer State, Temporal State, and Runtime State have different owners and lifetimes.
7. No ordinary guest causes runtime gradient updates or persistent person-specific model weights.
8. Additional cameras create additional observations; they do not require a different public API or a hard-coded two-camera network topology.
9. Vendor-specific GPU APIs remain behind an inference-backend interface.
10. Third-party extensions cannot bypass final physical, temporal, privacy, or output-safety guards.
11. Every stage can be exercised from recorded inputs without connected cameras.
12. Every public and internal cross-process contract is versioned.
13. A maintained abstraction, API field, configuration option, helper, or dependency has a current reader.
14. Replaced production paths are removed. The repository does not retain dead alternatives or speculative compatibility shims.
15. Experiment code stays outside maintained runtime, CLI, API, packaging, and task-runner surfaces until it replaces a production path.
16. Tests protect observable product behavior, runtime or artifact contracts, non-trivial algorithms, generation output, or published metrics.
17. First-party fixed startup contracts fail fast. External data is validated at its trust boundary.
18. Modularity is measured by localized replacement and lower production complexity, not by interface count.
19. All first-party shipping code is Rust, TypeScript, Python, schemas, shader source, or packaging code. The repository contains no first-party C++.
20. Rust `unsafe` is contained at measured foreign-function, OS, SIMD, or GPU boundaries. Safe domain and tracking crates do not inherit foreign ownership rules.
21. Cargo is the authoritative Rust build. Bazel is absent until an approved measured-need decision replaces that policy.

### RA-001 — Make the backend a headless product

The authoritative tracking system shall run as a headless daemon. The desktop UI is one replaceable client. Decomposing around information-hiding boundaries rather than the visible sequence of UI actions reduces the number of modules that must change when the interface, engine, or model implementation changes. [A01]

### RA-002 — Put all frontend/backend interaction behind a versioned service boundary

No frontend package may import backend Rust crates, read the backend database, access live GPU buffers, or invoke camera drivers. The frontend communicates only through documented APIs and standard media/pose streams. A process boundary must be treated as a real failure and latency boundary rather than disguised as a local object call. [A05] [A08]

### RA-003 — Hide likely-to-change decisions behind modules

Module boundaries shall follow likely sources of change: camera acquisition, calibration, spatial perception, fusion, temporal estimation, contact solving, avatar import, retargeting, output protocols, inference providers, storage, and UI. They shall not merely mirror one current end-to-end algorithm. This follows Parnas's information-hiding criterion for modular decomposition. [A01]

### RA-004 — Keep semantic contracts narrow and model-independent

Cross-module contracts shall describe observations such as keypoints, rays, covariance, body pose, contacts, expressions, and avatar transforms. They shall not expose backbone names, feature-map layouts, framework tensor classes, or model-specific normalization constants. ML systems are especially vulnerable to boundary erosion, undeclared consumers, and glue-code debt. [A18]

### RA-005 — Separate control, event, media, and realtime tracking planes

Configuration commands, progress events, preview media, debug data, and final tracking output have different reliability and latency needs. They shall use separate channels and queue policies. Staged event-driven systems and tail-latency research support explicit stages, admission control, and differentiated handling rather than one shared request path. [A02] [A03]

## 0.2 Engineering operating policy

These rules control implementation work across Rust, TypeScript, Python, schemas, model packages, data tooling, documentation, and release automation. They override the tendency to interpret a modular architecture as permission to add layers before they are needed.

### Implement the smallest current solution

- Write the smallest clear implementation that supports a current product, runtime, training, evaluation, packaging, or deployment need.
- Prefer direct concrete code over a wrapper, factory, base class, generic helper, or plugin seam when there is one caller and one implementation.
- Add a file, type, interface, adapter, helper, configuration field, public endpoint, command, dependency, or compatibility layer only when it has a current reader.
- A current reader is shipping code, a maintained development path, a required artifact contract, a release gate, or an operator-facing workflow. A possible future use is not a reader.
- Remove the replaced path. Do not leave `legacy`, `v2`, `new`, or `experimental` alternatives wired into production after a decision is made.
- Before the first stable public contract, replace provisional interfaces instead of carrying compatibility shims. After a stable release, preserve only the compatibility window stated by the public contract.
- Change only what the task requires. Do not reformat, reorder, rename, or generalize untouched code without a concrete reason.

### Create abstractions only at real seams

An abstraction is justified when at least one condition holds:

1. An external or untrusted boundary must be contained.
2. Two current production implementations exist.
3. Multiple current callers require the same behavior.
4. The abstraction removes more production complexity than it adds.
5. A stable semantic boundary is required between offline and runtime code.
6. A measured hot path needs provider or platform substitution.
7. Deterministic replay must replace a physical or nondeterministic source.

Do not create an interface solely because a class might have another implementation later. Do not create a fake implementation solely to justify an interface. Use concrete leaf classes and private helpers inside a real module boundary.

### Keep complexity explainable

- Prefer code that a maintainer can read, measure, and debug over layered defensive machinery.
- Any construct that is materially more complex than the local norm needs a reason of seven words or fewer in review or an Architecture Decision Record. Good reasons include `external payload, untrusted`, `hot path, measured`, `cross-process contract`, and `GPU lifetime constraint`.
- If the reason cannot be stated plainly, simplify the construct.
- A comment that excuses confusing code is a defect report. Rewrite the code. Keep comments for intent, coordinate frame, units, ownership, timing, lifetime, constraints, and non-obvious algorithm choices.
- Do not narrate the code, preserve change history in comments, or restate types and function names.

### Keep experiments off maintained surfaces

- Put experiments under an ignored `experiments-scratch/` directory owned by the relevant subsystem.
- Do not add experiment-only HTTP endpoints, CLI verbs, UI controls, root task-runner commands, package modules, launch options, or model-registry entries.
- Record compact notes, measured results, seeds, and artifact hashes. Do not keep experiment infrastructure alive against current APIs.
- When an experiment wins, promote it by replacing the old maintained path. Do not ship both paths indefinitely.
- A notebook may explain an experiment. It is never the authoritative generator, trainer, exporter, evaluator, or packager.

### Keep tests outcome-based and small

- Add a test when its failure would ship visibly broken tracking, violate a runtime or artifact contract, break generation or packaging, or publish a wrong benchmark, and no higher-level gate already catches it.
- Test a wanted observable behavior, a boundary contract, a non-trivial algorithm, deterministic generation output, or a published metric.
- Do not test private call order, internal class layout, developer constants, configuration echoes, fixture contents, incidental defaults, or values expected to change during normal tuning.
- Do not expose internals or enlarge production APIs for tests.
- Do not preserve accidental behavior merely because a bug exposed it. Add a bug test only when the corrected observable behavior is part of the product contract and is not already covered.
- Remove a test when normal refactoring or model tuning should change its expected value.
- Prefer one end-to-end replay or release gate that catches the user-visible failure over many narrow tests that duplicate it.

### Keep root automation scarce

The root task runner contains only commands used constantly on the maintained build, check, replay, package, or release path. Setup, maintenance, data acquisition, research, and one-off migration commands belong in the README of the tool that owns them. The root task runner must list its commands and arguments without requiring source inspection.

### Review findings by concrete cost

- Report a problem only when the broken behavior, contract, training result, release artifact, or published number can be named. A difference alone is not a defect.
- State the failure mechanism, then check the data structure, caller, queue policy, lifetime, and run duration before claiming growth, leakage, or instability.
- Read the current code before reporting a missing guard. Confirm that no caller or boundary already supplies it.
- Run the relevant gate in the environment that owns it. A failure caused by another toolchain is not a product finding.
- State the cost of a fix, including retraining, regenerating data, re-exporting, repackaging, recalibrating sites, invalidating caches, or changing public contracts.
- Rank findings by user harm and maintainer time. Prefer a small set of findings that pay for their fixes.

## 0.3 Trust and validation model

The source policy that inspired these rules assumed a private trusted team. This project is public and processes camera streams, avatar files, model packages, plugins, recordings, and local API traffic. It therefore uses a narrower trust model.

### Trusted after construction or verification

The following may be trusted after their owning build or startup gate succeeds:

- generated first-party OpenAPI, Protobuf, and JSON-schema bindings;
- compiled first-party graph profiles and fixed coordinate conventions;
- first-party static configuration packaged with the same release;
- in-process calls that pass canonical domain types;
- signed first-party model packages after schema, hash, and test-vector validation.

A mismatch in one of these fixed contracts fails startup or graph construction. The system does not add recovery branches for hypothetical internal misuse.

### Untrusted at entry

The following are validated at the boundary that first accepts them:

- camera driver frames, timestamps, formats, and device metadata;
- imported VRM, glTF, FBX-derived, image, audio, and recording files;
- downloaded or community model packages;
- plugin manifests, executable processes, and IPC messages;
- local or remote API requests;
- persisted state written by an older or crashed version;
- tensor outputs from provider plugins and external models.

Validation protects memory safety, indexing, arithmetic, resource limits, coordinate semantics, and stable state. It does not attempt to make every malformed input recoverable. Reject, quarantine, or fail the affected operation with a structured error. Do not repeat the same validation at every downstream layer.

---

# 1. Architecture at a glance

```text
                         REPLACEABLE CLIENTS

        Desktop UI          CLI          Tests        Third parties
             |               |             |                 |
             +---------------+-------------+-----------------+
                                     |
                         Versioned public API
                  HTTP/JSON control + WS/Protobuf streams
                                     |
                            API transport adapters
                                     |
                          Application service layer
           resources, commands, operations, authorization, events
                                     |
                           Session/orchestration layer
             builds graph, owns lifecycle, selects quality profile
                                     |
                +--------------------+--------------------+
                |                                         |
          REALTIME ENGINE                          PERSISTENT SERVICES
     staged graph, scheduler, clocks           sites, avatars, models,
     typed packets, bounded queues             settings, recordings
                |
     +----------+----------+----------+----------+----------+
     |          |          |          |          |          |
 capture   calibration  perception  fusion   temporal   retarget/output
     |                     |                     |
 platform adapters    task modules       deterministic safety guards
                           |
                    inference abstraction
                           |
               Rust inference adapters / ONNX Runtime providers

             OFFLINE ML AND DATA TOOLCHAIN

 public data + synthetic generation -> teachers -> students -> export
                         -> model package -> runtime registry
```

The application is not a collection of microservices deployed over a network. It is primarily a local, multi-process desktop system. Process separation is used only where it provides a clean product boundary, crash isolation, security isolation, or language independence. The latency-critical engine remains compact.

### RA-006 — Use a modular monolith for the authoritative backend

The daemon shall be one primary Rust process composed from well-defined crates and modules. It shall not be split into networked microservices merely to obtain modularity. Remote calls have latency, partial-failure, and concurrency semantics that differ from local calls; those costs should be accepted only where isolation justifies them. [A05]

### RA-007 — Keep the realtime output path inside the backend

VMC, VRChat OSC tracker output, VRM preview state, recording taps, and other sinks shall subscribe directly to the final backend pose stream. The frontend shall not receive a pose and send it back to the backend or to VRChat. End-to-end correctness and timing belong at the endpoints that possess the required information. [A04]

### RA-008 — Make the desktop UI optional

The daemon shall expose enough API surface for a CLI to enumerate cameras, import an avatar, calibrate a site, start tracking, select a quality profile, configure an output, record, replay, and collect diagnostics. This makes automated tests and community integrations first-class rather than reverse-engineered. [A01] [A08]

---

# 2. Process topology

## 2.1 Shipping processes

| Process | Required | Language | Responsibility | Realtime-critical |
|---|---:|---|---|---:|
| `avatartrackd` | Yes | Rust 2024 | Authoritative backend, capture, inference orchestration, fusion, temporal state, avatar solve, outputs, API | Yes |
| `avatartrack-ui` | No | TypeScript web UI in a desktop shell | Reference setup, calibration, preview, diagnostics, settings | No |
| `avatartrack-cli` | No | Rust | Headless automation and diagnostics using the same public API | No |
| `avatartrack-plugin-host` | Later | Rust | Starts and isolates third-party extension processes | Sometimes |
| `avatartrack-model-worker` | Optional | Rust | Isolates experimental inference providers or untrusted custom models | Potentially |
| `avatartrack-data` tools | Development | Python | Dataset ingestion, synthetic generation, training, export, evaluation | No |

The desktop shell may launch the daemon and manage its lifetime, but it does not become the daemon. A user may start `avatartrackd` manually, run it as a background service, or use a different UI.

The reference desktop shell may use Tauri, which itself uses Rust, but it remains a client process. Its Rust shell crate is prohibited from depending on daemon, engine, domain, adapter, or model-runner crates. It may depend only on frontend-shell utilities and generated public-client types.

### RA-009 — Use one composition root

`avatartrackd` shall contain the single production composition root that selects concrete camera adapters, storage adapters, inference providers, task modules, graph profiles, and output adapters. Libraries shall not discover global implementations through hidden singletons. Central composition makes dependencies reviewable and permits deterministic test composition. [A01]

### RA-010 — Isolate third-party executable code by default

Community plugins shall run out of process unless they are built, reviewed, and distributed as first-party components. A plugin crash or memory corruption must not terminate the tracking daemon. Cross-process semantics must remain explicit, including timeout, restart, and stale-data behavior. [A05]

### RA-011 — Keep trusted latency-critical modules in process

Built-in capture, geometry, temporal state, retargeting, and production inference adapters may execute in the daemon to avoid unnecessary copies and scheduling delays. Process isolation is not free and shall not be inserted into a deadline path without measured justification. [A03] [A05]

## 2.2 Thread and execution domains inside `avatartrackd`

```text
API I/O threads
    resource commands, event subscriptions, uploads

Camera I/O domain
    one capture loop per active camera or driver-defined capture group

CPU realtime domain
    timestamping, light preprocessing, geometry, state update, output tick

GPU submission domain
    backend-specific command submission and completion polling

Background domain
    avatar analysis, model compilation, recording compression, diagnostics

Output domain
    VMC/OSC/native adapters with bounded queues
```

A node does not own an arbitrary thread. It declares an execution class and deadline, and the scheduler assigns work to the appropriate domain.

### RA-012 — Prefer explicit stages and bounded queues over an unbounded task pool

Capture, preprocessing, inference, fusion, temporal estimation, retargeting, output, and recording shall be explicit stages connected by bounded channels. This permits load shedding and stage-specific measurement. SEDA was designed around event-driven stages and admission control; the same principle applies to a realtime local pipeline under variable GPU load. [A02]

### RA-013 — Optimize p95 and p99 pipeline behavior, not only mean throughput

The scheduler and startup benchmark shall measure tail latency, queue age, dropped work, and deadline misses. A high average FPS does not compensate for periodic stalls that create visible avatar glitches. [A03]

---

# 3. Repository structure

A monorepo is recommended because API schemas, Rust runtime crates, TypeScript clients, Python training code, model manifests, replay fixtures, and cross-platform packaging must change coherently. The repository should remain internally modular rather than becoming a single build target.

```text
avatartrack/
├── README.md
├── LICENSE
├── NOTICE
├── CONTRIBUTING.md
├── SECURITY.md
├── CODEOWNERS
├── .gitignore
├── .editorconfig
├── rust-toolchain.toml              # pinned stable toolchain and components
├── rustfmt.toml                     # one Rust formatting policy
├── Cargo.toml                       # virtual workspace, shared deps/lints/profiles
├── Cargo.lock                       # committed for application reproducibility
├── deny.toml                        # dependency license/advisory/source policy
├── eslint.config.js
├── pnpm-workspace.yaml
├── pnpm-lock.yaml
├── pyproject.toml
├── uv.lock
├── buf.yaml
├── buf.gen.yaml
├── justfile                         # sole cross-project task runner; `just --list`
├── .devcontainer/                   # canonical Linux development shell
├── containers/
│   ├── dev-linux/
│   └── ci-model-tools/
│
├── docs/
│   ├── product/
│   │   ├── person-agnostic-design.md
│   │   └── technical-architecture.md
│   ├── architecture/
│   │   ├── repository-api-modularity.md
│   │   ├── dependency-rules.md
│   │   ├── rust-safety-boundaries.md
│   │   ├── threat-model.md
│   │   └── adr/
│   ├── api/
│   ├── model-packages/
│   ├── plugins/
│   └── operations/
│
├── contracts/
│   ├── public-api/
│   │   ├── openapi/avatartrack-v1.yaml
│   │   ├── proto/avatartrack/v1/
│   │   └── generated/
│   ├── streams/proto/avatartrack/stream/v1/
│   ├── recording/proto/avatartrack/recording/v1/
│   ├── model-package/
│   │   ├── model-package.schema.json
│   │   └── task-contracts/
│   └── compatibility/
│       ├── api-baselines/
│       └── reserved-fields/
│
├── crates/                          # first-party shipping/runtime Rust only
│   ├── foundation/                  # time, math, IDs, errors, tracing primitives
│   ├── domain/                      # semantic camera, pose, avatar, contact types
│   ├── engine/                      # packet graph, scheduler, clocks, replay
│   ├── application/                 # use cases, resources, operations, lifecycle
│   ├── ports/                       # current external capability traits
│   ├── modules/
│   │   ├── capture/
│   │   ├── calibration/
│   │   ├── perception-fast/
│   │   ├── perception-spatial/
│   │   ├── face/
│   │   ├── hands/
│   │   ├── props/
│   │   ├── fusion/
│   │   ├── temporal/
│   │   ├── world-contact/
│   │   ├── avatar-import/
│   │   ├── retarget/
│   │   ├── outputs/
│   │   └── recording/
│   ├── adapters/
│   │   ├── camera-media-foundation/
│   │   ├── camera-v4l2/
│   │   ├── camera-gstreamer/        # optional external C dependency, safe wrapper
│   │   ├── inference-ort/           # Rust wrapper over ONNX Runtime C API
│   │   ├── gpu-wgpu/                # portable custom image/compute kernels
│   │   ├── storage-sqlite/
│   │   ├── output-vmc/
│   │   ├── output-vrchat-osc/
│   │   ├── transport-http/
│   │   └── transport-websocket/
│   ├── composition/
│   │   ├── production/
│   │   ├── profiles/
│   │   └── test/
│   └── bins/
│       ├── avatartrackd/
│       ├── avatartrack-cli/
│       ├── avatartrack-plugin-host/
│       ├── avatartrack-model-worker/
│       └── avatartrack-dev-viewer/
│
├── frontend/
│   ├── apps/
│   │   ├── desktop/                 # reference client; no backend crate deps
│   │   └── browser-dev/
│   └── packages/
│       ├── api-client-generated/
│       ├── backend-state/
│       ├── calibration-ui/
│       ├── avatar-preview/
│       ├── diagnostics-ui/
│       └── ui-kit/
│
├── plugins/
│   ├── sdk/                         # schemas and generated clients, not Rust ABI
│   ├── builtins/
│   ├── examples/
│   └── manifests/
│
├── ml/
│   ├── README.md
│   ├── avatartrack_ml/
│   │   ├── contracts/
│   │   ├── datasets/
│   │   ├── synthetic/
│   │   ├── teachers/
│   │   ├── models/
│   │   ├── training/
│   │   ├── distillation/
│   │   ├── losses/
│   │   ├── evaluation/
│   │   ├── export/
│   │   └── packaging/
│   ├── configs/
│   ├── experiments-scratch/        # ignored; never imported or packaged
│   ├── notebooks/                  # exploratory only inside scratch workflows
│   └── tests/
│
├── data/
│   ├── catalogs/
│   ├── datasheets/
│   ├── licenses/
│   ├── splits/
│   └── schemas/
│
├── models/
│   ├── registry/
│   ├── manifests/
│   ├── model-cards/
│   ├── quality-profiles/
│   └── public-keys/
│
├── tools/
│   ├── recorder/
│   ├── replay/
│   ├── benchmark/
│   ├── calibration-lab/
│   ├── dataset-inspector/
│   ├── model-packager/
│   ├── api-compat/
│   └── dependency-check/            # reads `cargo metadata`
│
├── tests/                           # cross-component tests only
│   ├── contract/
│   ├── replay/
│   ├── provider-parity/
│   ├── UI-e2e/
│   └── release-gates/
│
├── testdata/
│   ├── synthetic-mini/
│   ├── replay-fixtures/
│   ├── avatars/
│   ├── model-test-vectors/
│   └── release-fixtures/
│
├── packaging/
│   ├── windows/
│   ├── linux/
│   ├── appimage/
│   ├── flatpak/
│   └── model-updates/
│
└── .github/workflows/
```

The tree is a target ownership map, not a command to create empty directories. Add a directory when current maintained code, a contract, a release artifact, or an operator workflow needs it. Do not scaffold the full tree at project start.

## 3.1 Repository boundary and artifact policy

### First-party ownership

- Keep first-party behavior in the package that owns its semantic responsibility.
- Do not duplicate calibration, coordinate conversion, preprocessing, confidence calibration, or retarget rules between runtime and training without an explicit shared contract and parity gate.
- Before changing a feature definition used by both offline training and runtime inference, find both consumers and update the contract, generator, export path, runtime adapter, and test vectors together.

### External code

- Put vendored code and maintained hard forks under `external/`.
- Follow upstream style inside external code. Do not reformat unrelated upstream files.
- Keep patches focused. Preserve upstream license headers.
- Wrap external code at an adapter boundary. Do not spread upstream types through domain, application, or public API layers.

### Generated and large artifacts

Do not commit:

- build products;
- compiler, package-manager, or inference caches;
- raw datasets;
- downloaded source videos;
- generated synthetic shards;
- training checkpoints;
- credentials or API tokens;
- local benchmark reports;
- generated experiment output.

Track compact source-owned material instead:

- schemas and generators;
- lockfiles;
- dataset catalogs, splits, licenses, and provenance manifests;
- model manifests, model cards, graph/export recipes, hashes, and tiny test vectors;
- compact experiment notes and measured conclusions;
- release manifests.

Official model binaries are signed release artifacts or separately versioned packages. A bootstrap model may be committed only when the repository cannot build or run its minimum supported path without it, and the exception must state size, license, update policy, and current reader.

### Configuration ownership

- Keep one root Rust toolchain, `rustfmt`, workspace lint, Cargo workspace, lockfile, and dependency-policy configuration.
- Keep one root Python tool configuration in `pyproject.toml`. Subpackages do not add private Ruff, typing, or test policy without a tool limitation that prevents inheritance.
- Keep one root TypeScript lint and formatting policy. Packages add only package-specific compiler or bundler settings.
- Run a check in the environment declared by its owning tool. Use the canonical development environment when no tool-specific environment exists.

### RA-014 — Use a monorepo with independently buildable components

The source repository shall keep runtime, API, frontend, training, model packaging, release fixtures, and packaging definitions together, while each current top-level component remains independently buildable and testable. The listed tree is created incrementally. Empty layers and speculative packages are prohibited. This reduces interface drift without turning the system into a single compilation unit. [A01] [A18]

### RA-015 — Make contracts their own top-level product

Public API definitions, stream schemas, recording schemas, model-package schemas, compatibility baselines, and generated clients shall live under `contracts/`, not inside the UI or daemon source tree. This makes the interface visible and reviewable as a separate artifact. [A01] [A07] [A08]

### RA-016 — Keep training code and runtime code separate but schema-compatible

Python training code shall not be imported into the production daemon, and Rust runtime implementation details shall not become dataset contracts. They share generated schemas, task definitions, coordinate conventions, and test vectors. This limits ML glue-code and boundary erosion while preserving reproducibility. [A18] [A19]

### RA-017 — Keep notebooks non-authoritative

Notebooks may investigate models or data, but any result required for generation, training, evaluation, export, or release shall be moved into tested Python modules and declarative configs. Production ML readiness requires repeatable pipelines and tests beyond one-off experiments. [A19]

---

# 4. Dependency rules and layer ownership

## 4.1 Dependency direction

```text
foundation
    ^
    |
domain  <--- engine primitives
    ^              ^
    |              |
ports          reusable modules
    ^              ^
    +------- application -------+
                 ^              |
                 |              |
             adapters       transport
                 ^              ^
                 +------ composition/apps
```

The arrows indicate allowed compile-time dependency direction. The composition root may see all concrete implementations. Lower layers never import higher layers.

## 4.2 Layer responsibilities

| Layer | Owns | Must not own |
|---|---|---|
| `foundation` | time, math, result/error, memory, queues, tracing primitives | pose semantics, cameras, API resources |
| `domain` | canonical pose, camera/site/avatar/contact concepts and pure rules | drivers, ONNX Runtime, HTTP, database code |
| `engine` | packet graph, clocks, scheduler, buffers, replay mechanics | product workflows, UI, specific models |
| `ports` | abstract external capabilities | concrete OS/provider implementations |
| `modules` | tracking algorithms expressed through domain contracts | HTTP, frontend, database schemas |
| `application` | use cases, state machines, operations, resource lifecycle | camera driver details, tensor kernels |
| `adapters` | platform/device/provider/protocol implementations | product decision logic |
| `transport` | API serialization, authentication, request mapping | tracking logic or state mutation outside services |
| `composition` | dependency injection and selected defaults | reusable algorithms |

### RA-018 — Enforce dependency direction mechanically

CI shall reject forbidden crate/import edges and cycles. Architectural boundaries that exist only in documentation decay under schedule pressure. Module decomposition is useful only when change remains localized. [A01] [A18]

### RA-019 — Keep domain types independent of wire formats

Public OpenAPI/protobuf DTOs, internal realtime packet structs, persisted recording messages, and ML tensors shall be separate representations connected by explicit mappers. One representation should not be stretched across every boundary. This prevents a transport decision from controlling internal memory layout or forcing runtime changes on API clients. [A01] [A05]

### RA-020 — Keep transport adapters free of business logic

HTTP handlers, WebSocket handlers, and optional native RPC handlers shall authenticate, validate syntax, map DTOs, invoke application services, and map results. They shall not select cameras, compile models, alter calibration math, or mutate engine state directly. [A01] [A08]

### RA-021 — Use explicit application ports rather than service locators

Application services receive repositories, engine control, model registry, event publisher, and other dependencies through constructors or an explicit composition object. Hidden lookup and global state are prohibited because they make test composition and lifecycle reasoning unreliable. [A01]

## 4.3 Internal visibility

Every crate exposes only the public modules and items required by current cross-crate callers. Implementation modules remain private. A module may expose task-level interfaces and canonical outputs. Leaf helpers remain concrete and private. Cross-module friendship and direct access to mutable internals are prohibited.

A replaceable, operated, or externally consumed module normally contains:

```text
README.md              current purpose, use, contracts, timing, failure behavior
src/lib.rs             public crate root with a deliberately small export surface
src/                   private modules and implementation
tests/                 only tests justified by Section 23
benchmarks/            only measured hot paths or published claims
fixtures/              small deterministic inputs with current readers
MODULE.toml             only when tooling consumes the metadata
experiments-scratch/    ignored and never packaged
```

An internal leaf library does not need every directory or a README that repeats its code. Create only the files that have current readers.

### RA-022 — Require operational documentation at owned boundaries

An operated, replaceable, cross-process, model-package, or public module is not complete until its input/output semantics, state lifetime, timing expectations, reset behavior, failure modes, and owning workflow are documented beside the code. Internal leaf implementation details do not require boilerplate documentation. Model Cards and ML production-readiness work support explicit operating characteristics and limitations rather than undocumented artifacts. [A19] [A20]

## 4.4 Documentation and README policy

Maintained prose uses one name per thing and one meaning per name. It leads with the action or conclusion. It uses short, common words, active voice, complete grammar, and one instruction per sentence. It removes filler, hype, vague claims, unexplained emphasis, and synonyms used only for variety. Replace `fast`, `small`, `reliable`, and `accurate` with a measured value, bound, gate, or named limitation when the evidence exists.

Separate facts, engineering targets, measured results, and recommendations. Define each acronym once. Keep units and absolute dates. Do not alter code, literal protocol strings, field names, commands, or model identifiers for prose style.

A README is written for the person who must use or operate its component. It follows this order:

1. purpose;
2. prerequisites;
3. setup;
4. common commands and when to run them;
5. inputs, outputs, frames, units, rates, deadlines, and state lifetime;
6. configuration;
7. failure and troubleshooting behavior;
8. measured performance or tuning results, with date and hardware;
9. architecture details only when needed to maintain the component.

Link to the document that owns a workflow instead of repeating it. Do not restate code-owned defaults, thresholds, model lists, or provider matrices in several READMEs. Point to the schema, command, manifest, or generated capability output that owns the value.

## 4.5 Public-surface creation rule

A public API field, Protobuf message, CLI option, plugin permission, configuration key, model-manifest field, or persisted column must have a current producer and consumer. Do not add reserved placeholders for imagined features. Protobuf field reservation protects removed released fields; it is not a reason to publish unused fields.

---

# 5. Public API architecture

## 5.1 Planes

The public backend surface is divided into four planes.

| Plane | Transport | Payload | Use |
|---|---|---|---|
| Control | Loopback HTTP/JSON | OpenAPI-defined resources | configuration, lifecycle, import, calibration, outputs |
| Events | WebSocket | Protobuf or JSON debug form | progress, status, warnings, health, state changes |
| Tracking | WebSocket | Binary Protobuf | optional final pose stream for preview/integrations |
| Bulk/media | HTTP streaming; optional WebRTC later | model uploads, logs, recordings, compressed previews | large or media payloads |

The daemon binds only to loopback by default. Remote access is a separate opt-in mode with stronger authentication and TLS requirements.

### RA-023 — Use a language-neutral HTTP control API

The control plane shall be described by OpenAPI and expose resource-oriented HTTP/JSON operations. OpenAPI is a language-agnostic interface description suitable for generated clients and independent frontends; resource-oriented design avoids mirroring backend classes or storage tables. [A08] [A11]

### RA-024 — Use WebSocket for browser-compatible live event and pose streams

Low-latency server-pushed events and optional pose frames shall use a persistent WebSocket connection with defined subprotocols. WebSocket standardizes bidirectional message framing for browser and native clients. [A09]

### RA-025 — Use Protobuf for high-rate and persisted structured streams

Pose, event, and recording envelopes shall use Protobuf where payload frequency or size makes JSON inefficient. Field numbers are never reused, removed fields are reserved, and changes remain additive within a major schema. Protobuf explicitly supports compatible message evolution when these rules are followed. [A07]

### RA-026 — Keep native RPC optional and secondary

A native gRPC endpoint may be added for plugin hosts, automation, or high-throughput local clients, but the reference frontend shall not require it. gRPC provides typed service definitions and streaming, but browser and desktop UI portability remain the primary public-interface requirement. [A14]

## 5.2 API bootstrap

The desktop shell starts the daemon with a one-time bootstrap channel. The daemon chooses a loopback port and random session token, writes only the bootstrap information to a permission-restricted pipe or file, and then serves the normal API. The token is never embedded in the frontend bundle.

Example bootstrap record:

```json
{
  "pid": 41280,
  "base_url": "http://127.0.0.1:47831",
  "websocket_url": "ws://127.0.0.1:47831/v1/stream",
  "bearer_token": "<random launch token>",
  "api_major": 1,
  "expires_at": "2026-08-16T22:14:00Z"
}
```

### RA-027 — Authenticate even loopback clients

The local API shall require a per-launch bearer token and validate browser Origin headers. Binding to `127.0.0.1` alone does not authorize arbitrary local applications or web content. The WebSocket security model includes origin-based controls, and local process boundaries must be treated as real trust boundaries. [A05] [A09]

## 5.3 API compatibility model

- URL major version: `/v1/...`.
- Additive fields and endpoints are allowed within v1.
- Existing field semantics may not change silently.
- Removed protobuf fields are reserved forever.
- Every resource includes `schema_version` where persisted/exported.
- Every mutable resource includes a monotonic `revision`.
- Clients call `GET /v1/system/capabilities` at startup.
- The server returns explicit minimum and maximum supported client API versions.
- Model-package, plugin, recording, and public API versions evolve independently.

### RA-028 — Negotiate capabilities rather than infer them from version strings

Clients shall discover supported avatar formats, camera adapters, inference providers, output protocols, plugin types, quality modes, and optional debug streams from a capability resource. Version numbers establish compatibility ranges; capabilities establish actual availability on the current machine. [A05] [A16]

### RA-029 — Use optimistic concurrency for mutable configuration

Updates to sites, avatars, outputs, and profiles shall include the expected resource revision or HTTP `If-Match`. Conflicting edits return a revision-mismatch error instead of silently overwriting another client or background update. [A11]

### RA-030 — Make mutating retries safe

Create/start/import/calibrate requests shall accept an idempotency key. If a client retries after losing a response, the backend returns the original operation/resource instead of executing the mutation twice. Request IDs are a standard mechanism for idempotency guarantees. [A13]

## 5.4 Error contract

Every API error has a stable machine code, human summary, retryability, affected resource, and structured remediation hints.

```json
{
  "error": {
    "code": "CALIBRATION_INSUFFICIENT_BASELINE",
    "message": "Camera 2 is too similar to Camera 1 for reliable depth.",
    "retryable": true,
    "resource": "sites/demo-room/calibrationRuns/7f62",
    "details": {
      "camera_ids": ["cam-1", "cam-2"],
      "triangulation_score": 0.18
    },
    "remediation": [
      {
        "action": "MOVE_CAMERA",
        "camera_id": "cam-2",
        "direction": "farther_right",
        "severity": "recommended"
      }
    ]
  }
}
```

The frontend localizes machine codes and parameters. Backend English text is a fallback, not the UI's primary copy source.

### RA-031 — Return actionable structured errors

Errors shall describe what failed, whether retry is valid, and what corrective actions are available. The UI must not parse log text to determine application behavior. A language-neutral interface is useful only when consumers can understand capabilities and failures without inspecting implementation details. [A08]

## 5.5 API surface discipline

- Add an endpoint or field only for a current reference client, CLI workflow, automation path, plugin, or operator contract.
- Keep experimental controls on private development composition or scratch tools. Do not expose them through the maintained API.
- Do not use a generic `settings` map to bypass typed resource contracts.
- Do not mirror every backend class or model parameter. Expose the user operation or product resource.
- Before public v1 stability, remove obsolete provisional endpoints rather than retain adapters. After stability, follow the declared major-version policy.
- A field that only echoes a config value is not automatically useful. Return it only when a client needs it to render, reconcile, or diagnose state.
- A compatibility layer needs a named supported caller and removal condition.

---

# 6. Public resources and service functions

The API is organized around durable resources, ephemeral sessions, and long-running operations. The listed endpoints are the proposed v1 surface, not an instruction to expose internal database tables.

## 6.1 System and capabilities

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/v1/system` | version, build, OS, runtime status, daemon uptime |
| `GET` | `/v1/system/capabilities` | cameras, providers, models, formats, outputs, modes |
| `GET` | `/v1/system/health` | high-level readiness and degraded components |
| `POST` | `/v1/system:benchmark` | start startup/provider benchmark operation |
| `POST` | `/v1/system:shutdown` | clean daemon shutdown for authorized local client |

`GET /v1/system/capabilities` is the first substantive frontend request. The UI is capability-driven rather than assuming a fixed provider or feature list.

## 6.2 Camera devices

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/v1/cameras` | enumerate physical/logical cameras and modes |
| `GET` | `/v1/cameras/{camera_id}` | current mode, health, timestamps, permissions |
| `PATCH` | `/v1/cameras/{camera_id}` | select resolution, FPS, exposure policy, orientation |
| `POST` | `/v1/cameras/{camera_id}:probe` | short health and timing probe |
| `GET` | `/v1/cameras/{camera_id}/preview` | compressed low-rate preview stream or descriptor |

The camera resource represents a device and selected mode. It does not expose driver handles or raw pointers.

## 6.3 Sites and calibration runs

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/sites` | create a persistent capture site |
| `GET` | `/v1/sites` | list sites |
| `GET` | `/v1/sites/{site_id}` | site state and calibration revision |
| `PATCH` | `/v1/sites/{site_id}` | rename or update non-geometric settings |
| `DELETE` | `/v1/sites/{site_id}` | delete site after explicit confirmation |
| `POST` | `/v1/sites/{site_id}/calibration-runs` | start guided human-pose calibration |
| `GET` | `/v1/sites/{site_id}/calibration-runs/{run_id}` | progress and diagnostics |
| `POST` | `/v1/sites/{site_id}/calibration-runs/{run_id}:cancel` | cancel safely |
| `POST` | `/v1/sites/{site_id}:validate` | validate with held-out motion |
| `GET` | `/v1/sites/{site_id}/health` | camera movement, timing, baseline, floor health |

Calibration is represented as a long-running operation plus a calibration-run resource because it has progress, prompts, intermediate diagnostics, cancelability, and a durable result.

### RA-032 — Represent long work as operations

Avatar import, site calibration, provider benchmarking, model compilation, recording export, and diagnostic bundle creation shall return a long-running Operation resource rather than holding a request open. The long-running-operation pattern gives clients a token to track progress and retrieve a result. [A12]

## 6.4 Avatars and Avatar Profiles

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/uploads` | initiate content upload |
| `PUT` | `/v1/uploads/{upload_id}/content` | stream bytes to backend |
| `POST` | `/v1/avatars` | import from completed upload |
| `GET` | `/v1/avatars` | list imported avatars |
| `GET` | `/v1/avatars/{avatar_id}` | metadata, format, readiness |
| `GET` | `/v1/avatars/{avatar_id}/profile` | canonical skeleton mapping and capabilities |
| `POST` | `/v1/avatars/{avatar_id}:reanalyze` | rebuild profile after importer/model update |
| `POST` | `/v1/avatars/{avatar_id}:validate` | run canonical motion probes |
| `DELETE` | `/v1/avatars/{avatar_id}` | delete asset/profile |

The backend imports by content, not by assuming a local path. A trusted desktop shell may use a file picker, but it uploads the selected content through the same API used by other clients.

### RA-033 — Do not expose backend filesystem paths as the primary API

Public clients shall upload content or refer to backend resource IDs. A local path is meaningful only in one process and one security context, and it couples clients to deployment layout. Distributed-call semantics should not be disguised as shared-memory or shared-filesystem semantics. [A05]

## 6.5 Quality profiles

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/v1/quality-profiles` | list built-in and installed profiles |
| `GET` | `/v1/quality-profiles/{profile_id}` | rates, module variants, requirements |
| `POST` | `/v1/quality-profiles:recommend` | benchmark-aware recommendation |
| `POST` | `/v1/quality-profiles:validate` | validate requested profile against machine/site |

A quality profile is a named recipe. It is not a frontend-only collection of toggles.

## 6.6 Tracking sessions

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/sessions` | create session from site, avatar, profile, cameras |
| `GET` | `/v1/sessions/{session_id}` | authoritative session state |
| `PATCH` | `/v1/sessions/{session_id}` | update allowed live settings with revision |
| `POST` | `/v1/sessions/{session_id}:start` | build graph, warm modules, acquire performer |
| `POST` | `/v1/sessions/{session_id}:pause` | stop outputs but retain recoverable runtime state |
| `POST` | `/v1/sessions/{session_id}:resume` | resume after validation |
| `POST` | `/v1/sessions/{session_id}:stop` | stop and destroy performer/temporal state |
| `POST` | `/v1/sessions/{session_id}:resetPerformer` | explicit guest handoff |
| `POST` | `/v1/sessions/{session_id}:reacquire` | localized or full reacquisition |
| `GET` | `/v1/sessions/{session_id}/quality` | confidence, latency, drop, contact, view scores |
| `GET` | `/v1/sessions/{session_id}/graph` | sanitized active graph and module versions |

The backend owns the session state machine. The UI never assumes that a sequence of button clicks is valid; it asks the session resource which actions are currently allowed.

### RA-034 — Make session lifecycle explicit

Session commands shall be validated against an explicit backend state machine such as `CREATED`, `WARMING`, `ACQUIRING`, `TRACKING`, `DEGRADED`, `PAUSED`, `STOPPING`, and `STOPPED`. This prevents concurrent clients and asynchronous failures from producing impossible implicit state. [A02] [A05]

## 6.7 Outputs

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/outputs` | create VMC, VRChat OSC, recording, or plugin output |
| `GET` | `/v1/outputs` | list outputs and health |
| `GET` | `/v1/outputs/{output_id}` | configuration/status |
| `PATCH` | `/v1/outputs/{output_id}` | update destination or mapping |
| `POST` | `/v1/outputs/{output_id}:test` | send safe test packet or connection probe |
| `POST` | `/v1/outputs/{output_id}:enable` | enable for session |
| `POST` | `/v1/outputs/{output_id}:disable` | stop without deleting |
| `DELETE` | `/v1/outputs/{output_id}` | remove configuration |

VMC, VRChat OSC trackers, and other protocols are output adapters fed from the final avatar solve. VRM provides a standardized humanoid avatar representation, while VMC and VRChat OSC provide documented motion/tracker integration surfaces. [A17] [A24] [A25]

## 6.8 Models and inference providers

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/v1/models` | installed model packages and status |
| `POST` | `/v1/models:install` | validate and install package from upload |
| `POST` | `/v1/models/{model_id}:benchmark` | benchmark provider/profile combinations |
| `POST` | `/v1/models/{model_id}:validate` | run package test vectors |
| `DELETE` | `/v1/models/{model_id}` | uninstall if not required |
| `GET` | `/v1/inference-providers` | provider capabilities and compiled-cache health |

## 6.9 Recordings, replay, and diagnostics

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/v1/recordings` | begin a recording attached to a session |
| `POST` | `/v1/recordings/{id}:stop` | finalize recording |
| `GET` | `/v1/recordings/{id}` | metadata and privacy tier |
| `POST` | `/v1/replays` | create replay session from recording |
| `POST` | `/v1/replays/{id}:start` | execute graph or selected downstream stages |
| `POST` | `/v1/diagnostic-reports` | create redacted support bundle |
| `GET` | `/v1/operations/{operation_id}` | generic operation state |
| `POST` | `/v1/operations/{operation_id}:cancel` | cancel if supported |

---

# 7. Event, telemetry, preview, and pose streams

## 7.1 Event WebSocket

Endpoint:

```text
GET ws://127.0.0.1:<port>/v1/events
Sec-WebSocket-Protocol: avatartrack.events.v1.protobuf
Authorization: Bearer <token>
```

The client sends one subscription message containing resource filters and event categories. The server sends a snapshot marker and then ordered events.

```protobuf
message EventEnvelope {
  uint32 schema_version = 1;
  string event_id = 2;
  uint64 sequence = 3;
  int64 emitted_monotonic_ns = 4;
  string resource_name = 5;
  string trace_id = 6;

  oneof payload {
    OperationProgress operation_progress = 20;
    SessionStateChanged session_state_changed = 21;
    CameraHealthChanged camera_health_changed = 22;
    CalibrationPrompt calibration_prompt = 23;
    CalibrationQuality calibration_quality = 24;
    TrackingQuality tracking_quality = 25;
    FailureGuardTriggered failure_guard = 26;
    OutputHealthChanged output_health_changed = 27;
    ModelStatusChanged model_status_changed = 28;
  }
}
```

Low-rate events are resumable with the last received sequence number. High-rate pose frames are not replayed through this stream.

### RA-035 — Use snapshot plus ordered event updates

A client shall obtain an authoritative resource snapshot and then subscribe to ordered changes. Reconnection starts from a known revision or requests a fresh snapshot. The UI must not reconstruct durable state from best-effort log messages. [A05] [A10]

## 7.2 Pose WebSocket

Endpoint:

```text
GET ws://127.0.0.1:<port>/v1/sessions/<id>/pose-stream
Sec-WebSocket-Protocol: avatartrack.pose.v1.protobuf
```

The public pose stream is a mirror of the final output state, intended for preview and integrations. It is never a required link in the engine-to-output path.

```protobuf
message PoseFrameHeader {
  uint32 schema_version = 1;
  string session_id = 2;
  uint64 sequence = 3;
  int64 source_time_ns = 4;
  int64 target_output_time_ns = 5;
  string avatar_profile_revision = 6;
  string site_revision = 7;
  float overall_confidence = 8;
}

message BoneTransform {
  uint32 semantic_bone = 1;
  Vec3 translation_m = 2;
  Quaternion rotation = 3;
  float confidence = 4;
  bool predicted = 5;
}

message AvatarPoseFrame {
  PoseFrameHeader header = 1;
  repeated BoneTransform bones = 2;
  repeated ExpressionValue expressions = 3;
  repeated ContactState contacts = 4;
  TrackingQualitySummary quality = 5;
}
```

Queue policy is **latest useful frame**. A slow preview client does not accumulate seconds of stale animation.

### RA-036 — Never apply reliable-delivery semantics to obsolete pose frames

Pose streams shall discard superseded frames under backpressure. Commands and resource mutations require reliable responses; a pose intended for a past display tick does not. Stage-specific queue semantics are necessary to keep an event-driven system well-conditioned. [A02] [A03]

## 7.3 Preview media

Initial implementation:

- camera thumbnails at 5–15 FPS;
- annotated JPEG/WebP frames over HTTP multipart or binary WebSocket;
- final avatar preview rendered in the frontend from `AvatarPoseFrame`;
- no raw full-resolution video sent unless the user opens a diagnostic view.

A future WebRTC preview path is optional. It must not become a prerequisite for tracking.

### RA-037 — Treat preview as best effort

Preview encoding, transfer, and frontend rendering shall run at lower priority than face, body, hands, output, and contact state. The product may temporarily reduce preview FPS rather than increase tracking latency. Tail-latency and staged-load-control principles support sacrificing noncritical work first. [A02] [A03]

## 7.4 Debug streams

Debug streams are capability- and permission-gated. Possible topics:

- per-camera 2D observations;
- reprojection overlays;
- camera covariance and timing offsets;
- fused joint uncertainty;
- contact probabilities;
- scheduler deadlines and queue ages;
- provider tensor timing;
- failure-guard decisions.

They are disabled by default and may expose sensitive camera-derived information.

---

# 8. Frontend architecture

The reference frontend is a TypeScript web application packaged by a replaceable desktop shell. Tauri is a reasonable reference shell because it supports web frontends and Windows/Linux desktop targets, but no backend contract depends on Tauri. [A15]

```text
frontend/apps/desktop
    desktop shell only: launch daemon, window/tray, file picker

frontend/packages/api-client-generated
    generated HTTP client, event and pose decoders

frontend/packages/backend-state
    query cache, event reconciliation, capability model

frontend/packages/calibration-ui
    renders backend-defined prompts and quality feedback

frontend/packages/avatar-preview
    loads standard avatar asset and applies AvatarPoseFrame

frontend/packages/diagnostics-ui
    charts and overlays from permitted debug streams

frontend/packages/ui-kit
    presentation components and themes
```

## 8.1 Frontend rules

The frontend may:

- request resources and operations;
- display backend-defined state and allowed actions;
- render camera previews and final avatar poses;
- locally manage navigation, panel layout, drafts, accessibility, and localization;
- use a desktop file picker, then upload the selected bytes through the API.

The frontend may not:

- open camera devices;
- run tracking models;
- calculate camera extrinsics;
- own the session state machine;
- edit backend storage directly;
- decide which observations are safe enough to move the avatar;
- publish final VMC/OSC output;
- bypass API validation through desktop-shell commands.

### RA-038 — Keep product logic in backend application services

Calibration sequencing, quality recommendation, session validity, camera selection, model selection, output readiness, and failure recovery shall be backend decisions. The frontend presents them. This permits another UI or CLI to obtain identical behavior. [A01] [A08]

### RA-039 — Make calibration UI prompt-driven

The calibration wizard shall display `CalibrationPrompt` resources/events supplied by the backend, including pose, duration, detected quality, and remediation. It shall not hard-code the number or order of calibration steps. Calibration research is evolving, and the UI must not prevent the backend from changing the sequence. [P15] [P16] [P17]

### RA-040 — Use capability-driven rendering

The UI shall show or hide controls based on the capability resource and current resource state, not GPU-vendor detection or hard-coded product editions. This keeps the frontend independent of inference-provider and module composition. [A05] [A16]

## 8.2 Frontend/backend contract examples

### Import avatar

```text
UI file picker
    -> POST /v1/uploads
    -> PUT bytes
    -> POST /v1/avatars
    -> Operation resource
    -> event stream reports phases
    -> Avatar resource becomes READY or FAILED
```

The UI does not import VRM itself to discover whether the backend can use it. It may render a preview copy after the backend has produced an Avatar Profile.

### Calibration

```text
UI creates calibration run
    -> backend emits prompt: stand visible
    -> UI displays prompt
    -> backend emits per-camera visibility and placement advice
    -> backend emits prompt: A-pose
    -> backend solves and validates
    -> run returns Site State revision
```

### Tracking

```text
UI creates session using resource IDs
    -> starts session
    -> receives state and quality events
    -> subscribes to optional pose stream
    -> backend independently drives VRChat/VMC output
```

## 8.3 Mock backend

The frontend repository includes a generated mock server that implements the public API and emits scripted events. UI work and UI end-to-end tests must not require cameras or a GPU.

### RA-041 — Test the frontend against the contract, not the daemon implementation

Frontend CI shall use a contract-generated mock and a smaller set of full daemon tests. This catches accidental reliance on undocumented behavior and lets UI development proceed independently. [A01] [A08]


# 9. Backend application layer

The application layer owns product use cases. It converts public commands into validated domain operations and coordinates the realtime engine without containing the tracking algorithms themselves.

## 9.1 Application services

```text
SystemService
    capabilities, health, benchmark, shutdown

DeviceService
    enumerate/configure/probe cameras and audio devices

SiteService
    create site, start calibration, validate, monitor health

AvatarService
    upload/import/analyze/validate/delete avatar assets

SessionService
    create/start/pause/resume/stop/reset guest/reacquire

QualityService
    recommend and validate hardware/site quality profiles

OutputService
    configure and supervise VMC, OSC, preview, recording sinks

ModelService
    install, validate, benchmark, activate model packages

PluginService
    discover, grant permissions, start, stop, inspect extensions

RecordingService
    create recordings, start replays, export redacted traces

DiagnosticsService
    health snapshots, support bundles, benchmark reports
```

Each service is an interface. The production implementation is composed in `apps/daemon`; unit tests use in-memory repositories and fake engine ports.

### RA-042 — Organize application APIs around use cases, not subsystem objects

The public and application service surface shall expose operations meaningful to users and integrations: calibrate a site, import an avatar, start a session, configure an output. It shall not expose internal objects such as `TemporalTransformer`, `BundleAdjuster`, or `OrtSession`. Resource-oriented APIs should not mirror implementation or database structure. [A11]

## 9.2 Commands, queries, and operations

The application layer distinguishes:

- **Queries:** read snapshots; no tracking-state mutation.
- **Commands:** request a mutation; validated and idempotent where applicable.
- **Operations:** asynchronous work with progress, cancellation, and result.
- **Events:** facts emitted after authoritative state changes.

This is not full event sourcing. Resource repositories remain the source of durable truth. Events support clients, observability, and automation.

### RA-043 — Do not make the event stream the only durable source of truth

The event stream may be sampled, disconnected, or versioned independently. Durable Site State, Avatar Profiles, model installation state, and output configuration shall be queryable directly. Clients recover by reading snapshots and resubscribing. [A05] [A10]

## 9.3 Session orchestrator

`SessionOrchestrator` is the product-level coordinator for a tracking session. It:

1. loads Site State, Avatar Profile, quality profile, model packages, and output configs;
2. validates camera health and calibration revisions;
3. asks `GraphFactory` to construct a pipeline graph;
4. warms inference modules and compiled-provider caches;
5. allocates memory arenas and bounded queues;
6. starts capture and output domains in the correct order;
7. runs guest acquisition;
8. transitions to tracking only after minimum readiness gates;
9. handles camera loss, provider failure, plugin failure, and quality degradation;
10. stops outputs before destroying session state;
11. explicitly clears Performer and Temporal State.

It does not implement pose estimation, calibration math, IK, or network transport.

### RA-044 — Centralize lifecycle, decentralize algorithms

One orchestrator shall own session lifecycle and graph replacement, while individual modules own their algorithms and local state. This avoids a distributed collection of modules independently deciding whether the product is tracking, paused, or stopping. [A02] [A05]

## 9.4 Long-running operation manager

`OperationManager` stores operation metadata and dispatches cancellable background jobs. Operations have:

```text
name
kind
resource target
state: PENDING | RUNNING | SUCCEEDED | FAILED | CANCELING | CANCELED
phase code
progress 0..1, optional
start/end timestamps
can_cancel
result resource name, optional
structured error, optional
trace ID
```

Cancellation is cooperative. A model compilation step that cannot safely stop immediately reports `CANCELING` until it reaches a cancellation point.

## 9.5 Event publisher

Application events are published only after state mutation commits. Events contain a resource name, new revision, and trace ID. The realtime engine emits high-rate quality summaries to a bounded telemetry bridge; the application layer turns selected transitions into client-facing events.

### RA-045 — Keep high-rate telemetry out of the resource transaction path

Per-frame metrics and pose quality shall not require database transactions or synchronous API-event publication. They flow through a bounded telemetry path and are aggregated before client exposure. [A02] [A03]

---

# 10. Realtime engine and pipeline graph

## 10.1 Engine responsibilities

The engine provides reusable mechanics:

- typed nodes and ports;
- immutable packet envelopes;
- bounded queue implementations;
- execution-domain scheduling;
- deadlines and cancellation;
- session clocks and timestamp mapping;
- memory-pool and buffer-handle lifetime;
- graph startup, warmup, drain, reset, and shutdown;
- stage taps for recording and debugging;
- deterministic replay;
- latency tracing.

It does not know what a knee, smile, guitar, VRM, or camera-calibration pose means. Those are domain/module concepts.

### RA-046 — Model the runtime as a streaming directed graph

The tracking path shall be a directed graph of stages that exchange immutable timestamped packets. Nodes may operate at different rates and on asynchronous inputs. This follows streaming online-HMR requirements and staged event-driven architecture, and supports recorded replay and stale-result rejection. [P01] [A02]

## 10.2 Node contract

Conceptual interface:

```rust
pub trait Node: Send {
    fn descriptor(&self) -> NodeDescriptor;
    fn configure(
        &mut self,
        config: &NodeConfig,
        services: &mut NodeServices,
    ) -> Result<(), NodeError>;
    fn warm_up(&mut self, context: &WarmupContext) -> Result<(), NodeError>;
    fn start(&mut self, context: &StartContext) -> Result<(), NodeError>;
    fn request_stop(&self);
    fn drain(&mut self, deadline: Deadline);
    fn reset(&mut self, scope: ResetScope);
}
```

Data processing is implemented through typed input handlers registered in the node descriptor rather than one untyped `void*` callback.

`NodeDescriptor` declares:

- stable node type ID;
- input and output port types;
- execution class;
- statefulness and reset scopes;
- maximum in-flight submissions;
- deadline class;
- whether packets can be dropped/coalesced;
- memory domains accepted/produced;
- capabilities and optional ports;
- instrumentation name.

### RA-047 — Make node scheduling requirements declarative

Nodes shall declare their execution and queue requirements rather than creating unmanaged threads. The scheduler needs a global view to protect output deadlines, shed load, and attribute stalls. SEDA and realtime inference architecture support explicit stages and controlled resource allocation. [A02] [P58]

## 10.3 Typed ports

Examples:

```text
CameraFrame
CameraHealth
FastWholeBody2DObservation
SpatialBodyObservation
FaceObservation
HandObservation
PropObservation
CalibrationTrackWindow
FusedBodyObservation
TemporalPerformanceState
ContactConstraintSet
CanonicalPerformanceFrame
AvatarPoseFrame
TrackingQualityFrame
```

A port type is identified by a stable semantic contract ID and major version. In-process Rust type identity alone is insufficient for replay and plugin boundaries.

### RA-048 — Version semantic port contracts independently

A hand module and temporal module may evolve at different rates. Each cross-module semantic contract shall have an explicit version and converter policy. This prevents a model upgrade from silently changing coordinate, confidence, or visibility semantics. [A07] [A18]

## 10.4 Queue policies

Each edge declares one policy.

| Policy | Behavior | Typical use |
|---|---|---|
| `LATEST_ONLY` | retain newest useful packet; drop superseded | camera frames, pose previews |
| `KEYED_LATEST` | retain newest per key/camera/hand | asynchronous per-camera observations |
| `BOUNDED_FIFO` | preserve order until fixed capacity | commands crossing into engine |
| `WINDOW` | maintain timestamped history range | temporal model context |
| `LOSSLESS_RECORDING` | block or spill only in explicit offline mode | offline export, never live default |
| `COALESCE` | merge updates by resource/field | quality and health telemetry |

Every queue records produced, consumed, dropped, coalesced, maximum age, and maximum depth metrics.

### RA-049 — Make backpressure policy part of the interface

A producer and consumer must agree whether old work is droppable, coalescible, or lossless. Hidden unbounded queues turn temporary GPU or disk stalls into seconds of latency. Explicit queue admission and shedding are central to well-conditioned staged systems. [A02] [A03]

## 10.5 Scheduling classes

| Class | Examples | Scheduling rule |
|---|---|---|
| `OUTPUT_CRITICAL` | prediction tick, final retarget, VMC/OSC output | fixed cadence; cannot wait for heavy inference |
| `INTERACTIVE_HIGH` | face, gaze, fast hands, 2D observations | earliest deadline first within budget |
| `BODY_OBSERVATION` | spatial body, multiview fusion | information-value/deadline scheduling |
| `CALIBRATION` | graph solve, validation | background during setup; bounded CPU/GPU use |
| `BEST_EFFORT` | preview, debug overlays | shed first |
| `BACKGROUND` | cache compilation, recording compression | pause under realtime pressure |

### RA-050 — Schedule by deadline and information value

The scheduler shall not run a global FIFO. It considers output deadline, packet age, current uncertainty, view information score, and module cost. The companion technical design's motion-aware and view-aware scheduling depends on exposing these quantities to the scheduler. [A03] [P23] [P56]

## 10.6 Graph construction

`GraphFactory` receives:

```text
SiteState
AvatarProfile
QualityProfile
available cameras
installed task modules
provider benchmark results
output configurations
optional plugin capabilities
```

It produces a validated `GraphPlan` before allocating runtime resources. Validation includes:

- all required semantic ports are connected;
- coordinate-space converters exist;
- model-package contract ranges match;
- provider memory requirements fit;
- worst-case in-flight buffers fit budget;
- output cadence is satisfiable;
- session reset reaches every stateful node;
- camera count and profile requirements are met;
- optional modules have defined fallbacks.

### RA-051 — Fail graph construction before starting capture

An incompatible hand model, missing coordinate converter, unsupported provider, or insufficient memory shall produce a configuration error before the session begins. Runtime should not discover basic contract incompatibility during a performance. ML production-readiness work emphasizes pre-deployment tests and explicit dependency validation. [A19]

## 10.7 Hot reconfiguration

Permitted live changes:

- preview rate;
- debug subscriptions;
- noncritical model rate within the current profile envelope;
- camera heavy-view scheduling;
- output enable/disable;
- adaptive quality parameters explicitly marked live-safe.

Graph-rebuild changes:

- camera add/remove if topology cannot absorb it dynamically;
- Site State revision;
- Avatar Profile change;
- task-module major version;
- coordinate-contract major version;
- inference provider requiring new memory arenas.

Graph rebuild occurs through a shadow graph:

1. construct and warm new graph;
2. validate outputs on current state or replay window;
3. atomically switch final output ownership;
4. drain old graph;
5. destroy old state.

### RA-052 — Use shadow replacement for material graph changes

The live graph shall not be mutated node-by-node when a change can temporarily leave invalid connections or mixed model versions. Build and validate a replacement, then switch at a defined output boundary. [A04] [A05]

## 10.8 Memory behavior

- Packet payloads are reference-counted immutable buffers.
- Realtime paths use preallocated pools.
- Image frames use `FrameBufferHandle` rather than copying pixel arrays between nodes.
- Tensor buffers use `DeviceBufferHandle` with an explicit memory domain.
- GPU-to-CPU transfers are explicit nodes or inference-adapter operations.
- No heap allocation is allowed on the final output tick after warmup, except bounded emergency logging buffers.

### RA-053 — Preallocate realtime queues and packet storage

The output-critical path shall use bounded preallocated storage. Preallocated ring-buffer designs reduce allocation and contention variability, which matters more for visible tail stalls than peak throughput. [A06] [A03]

---

# 11. Runtime packets and semantic contracts

## 11.1 Universal packet header

```rust
#[derive(Clone, Debug)]
pub struct PacketHeader {
    pub contract: ContractId,
    pub contract_major: u32,
    pub contract_minor: u32,

    pub source: SourceId,
    pub sequence: u64,

    pub source_time: MonotonicTime,
    pub capture_or_observation_time: MonotonicTime,
    pub produced_time: MonotonicTime,
    pub deadline: MonotonicTime,

    pub coordinate_frame: CoordinateFrameId,
    pub site_revision: CalibrationRevision,
    pub avatar_revision: AvatarProfileRevision,
    pub session: SessionId,

    pub trace_id: TraceId,
    pub confidence: f32,
    pub age_at_production: Duration,
    pub flags: PacketFlags,
}
```

Not every packet uses every field, but timestamps, source, contract, revision, and trace context are mandatory.

### RA-054 — Carry source time and age through every stage

A result is not useful merely because it arrived. Fusion and temporal modules must know when the evidence was observed and how stale it is. Asynchronous multi-camera fusion and online HMR require explicit timing rather than processing-order assumptions. [P01] [P16] [P23]

## 11.2 Confidence and uncertainty

Canonical observations distinguish:

- detection confidence;
- visibility probability;
- occlusion probability;
- covariance or anisotropic positional uncertainty;
- age;
- predicted versus observed status;
- source provenance;
- calibration uncertainty contribution.

A single scalar confidence may accompany a public preview, but internal fusion uses structured uncertainty.

### RA-055 — Preserve uncertainty rather than threshold it away early

Per-camera perception shall not convert uncertain evidence into a hard joint location and discard its uncertainty. Uncertainty-aware and probabilistic multiview methods use confidence and covariance to weight evidence and handle missing views. [P22] [P23] [P24]

## 11.3 Coordinate-frame registry

Every transform is expressed between named frames:

```text
camera/<id>/optical
site/world
site/floor
performer/root
performer/canonical
body_model/<family>
avatar/<avatar_id>/root
avatar/<avatar_id>/bone/<semantic>
vrchat/world
vmc/world
prop/<prop_id>
```

`TransformRegistry` stores time-varying or static transforms with revisions and uncertainty. A module may not assume that a position called `xyz` is in camera or world space.

### RA-056 — Make coordinate spaces and units explicit in contracts

Meters, radians, handedness, axis directions, rest pose, and coordinate frame shall be defined for every spatial contract. Rotation-representation and retargeting research show that representation and skeleton conventions materially affect continuity and correctness. [P36] [P41]

## 11.4 Canonical semantic skeleton

The realtime canonical skeleton is stable across body models and avatars. It includes:

- root/pelvis;
- spine segments and chest;
- neck and head;
- clavicles;
- upper/lower arms and wrists;
- upper/lower legs;
- ankles, heels, balls, and toes;
- optional detailed hand joints;
- optional face/gaze channels through separate contracts.

Model adapters map MHR/SMPL-X/MANO/other outputs into this contract. Avatar importers map target rigs out of it.

### RA-057 — Use one canonical performance representation between perception and retargeting

Perception modules shall not output target-avatar bones, and avatar importers shall not alter human-perception modules. A canonical performance space isolates changes in body model from changes in avatar topology and supports skeleton-aware retargeting. [P36] [P39] [P40]

## 11.5 Provenance

Each fused or final element can report contributing sources:

```text
right wrist:
    camera 1 fast2d observation
    camera 2 spatial-body observation
    temporal prediction
    guitar contact constraint
    final IK correction
```

Detailed provenance may be compressed or sampled in normal operation but must be available in replay/debug builds.

### RA-058 — Preserve enough provenance to explain corrections

When a hand jumps or a root correction is rejected, developers need to know which camera, model, contact, or prediction caused the decision. Observability and ML-system testing require tracing behavior across non-learning and learned components. [A10] [A18] [A19]

## 11.6 Public versus internal contract stability

| Contract | Stability | Audience |
|---|---|---|
| Public resources and final `AvatarPoseFrame` | High | UI, plugins, integrations |
| Recording schema | Medium-high with converters | tools and regression corpus |
| Canonical observations | Medium; major-versioned | internal modules and trusted plugins |
| Opaque latent feature blobs | Low; exact family hash required | co-designed model packages only |
| Provider tensor layouts | Private | one task module + backend adapter |

### RA-059 — Do not promise arbitrary latent-feature compatibility

Intermediate neural features may be shared only among explicitly co-versioned model packages with a feature-schema hash. Semantic contracts remain the default module boundary. Hidden ML entanglement and undeclared consumers create severe maintenance debt. [A18]

---

# 12. State ownership and persistence

## 12.1 State domains

| State | Owner | Lifetime | Persistence | Contains |
|---|---|---:|---:|---|
| Site State | `SiteRepository` | until cameras move/change | Yes | intrinsics, extrinsics, timing, floor, capture volume, uncertainty |
| Avatar Profile | `AvatarRepository` | until asset/profile changes | Yes | semantic skeleton mapping, proportions, limits, expressions, retarget config |
| Model Registry | `ModelRepository` | installation lifetime | Yes | packages, variants, hashes, provider caches, benchmark results |
| App Settings | `SettingsRepository` | user installation | Yes | UI-independent defaults and privacy choices |
| Performer State | session | current guest | No by default | ephemeral anthropometry, face neutral state, active track |
| Temporal State | engine nodes | seconds | No | pose history, contacts, velocities, hidden-limb state |
| Runtime State | orchestrator/engine | process/session | No | queues, compiled sessions, device handles, graph instances |
| Recording | `RecordingRepository` | user-controlled | Optional | selected packet taps and optional video |

### RA-060 — Give each state domain one authoritative owner

No module may maintain an independent writable copy of Site State, Avatar Profile, or session lifecycle. Read-only snapshots may be distributed through packets. Explicit ownership avoids inconsistent hidden state during handoff, camera movement, and graph rebuild. [A01] [A05]

### RA-061 — Destroy performer and temporal state on guest handoff

`resetPerformer` and session stop shall clear active identity association, body proportions, face normalization, pose history, contacts, motion mode, and prediction caches while preserving Site State and Avatar Profile. The product must remain person-agnostic after site calibration. [P15] [P16]

## 12.2 Persistent store

Recommended implementation:

- SQLite for resource metadata, revisions, migrations, and operation records;
- content-addressed blob storage for uploaded avatars, model packages, recordings, and caches;
- SHA-256 content IDs and explicit provenance metadata;
- atomic write-then-rename for large file finalization;
- no storage schema exposed through public APIs.

Directory concept:

```text
user-data/
├── state.db
├── blobs/sha256/<prefix>/<hash>
├── model-cache/<provider>/<device>/<package-hash>/
├── logs/
├── recordings/
├── diagnostics/
└── temp/
```

The storage adapter can later be replaced without changing application services.

### RA-062 — Expose repositories, not database connections

Application services depend on `SiteRepository`, `AvatarRepository`, `ModelRepository`, and related ports. They do not execute SQL directly. The storage layout is an implementation decision likely to change and should be hidden. [A01]

## 12.3 Resource revisions and derived artifacts

Derived artifacts include:

- Avatar Profile generated from avatar bytes and importer version;
- compiled inference engine generated from model package, provider, device, driver, and profile;
- site-quality report generated from Site State and camera health;
- calibration result generated from input tracks and solver version.

Every derived artifact records the exact inputs and software versions. If any dependency changes materially, the artifact is invalidated or marked stale.

### RA-063 — Key caches by full compatibility identity

A compiled model cache shall include model hash, backend/provider version, device identity, precision profile, input shape profile, and relevant driver/runtime identity. Reusing a superficially matching but incompatible engine can create silent numerical or performance failures. Model-conversion research documents substantial cross-framework and conversion challenges. [P60]

## 12.4 Privacy tiers

Default persistent state does not include raw performer images. Recording modes:

1. **Pose only:** canonical/final pose, confidence, timing, system metadata.
2. **Derived debug:** 2D observations, masks, calibration tracks, no raw frames.
3. **Redacted media:** selected blurred/cropped/compressed frames.
4. **Full research capture:** raw or high-quality frames; explicit opt-in and warning.

### RA-064 — Make raw-camera persistence opt-in

The backend shall not save raw webcam streams merely because recording or diagnostics is enabled. Dataset and model documentation practices require explicit provenance and intended use; privacy risk must be visible at collection time. [A20] [A21]

---

# 13. Modular ML architecture

The ML design has three separate abstraction layers:

```text
TASK MODULE
    understands images, joints, faces, hands, body, fusion, temporal state
    performs preprocessing and postprocessing
    emits canonical semantic observations

MODEL RUNNER
    compiles and executes a model package with named tensors
    has no knowledge of knees, smiles, cameras, or VRM

INFERENCE BACKEND
    owns provider/device/session/buffer mechanics
    implements ONNX Runtime or another backend behind the same interface
```

This separation prevents a task module from depending directly on CUDA/TensorRT/DirectML and prevents the inference backend from understanding product semantics. It does not require an interface for every neural network class or preprocessing helper. The task seam, model-runner seam, and provider seam are the maintained boundaries. Concrete code inside each seam remains direct until a second current implementation or a measured complexity reduction justifies another layer.

### RA-065 — Separate task semantics from tensor execution

Every learned stage shall be implemented as a task module that maps canonical inputs to canonical outputs through an abstract model runner. Provider-specific tensor execution remains below it. This localizes model and backend changes and counters ML boundary erosion. [A18] [P56] [P58]

## 13.1 Task module families

| Task contract | Typical implementation choices | Required fallback |
|---|---|---|
| `PersonRegionTracker` | detector + segmentation + temporal association | full-frame single-person crop |
| `FastWholeBody2D` | RTMPose/RTMO-derived student | smaller keypoint student |
| `SpatialBodyEstimator` | distilled SAM-3D-body/HMR student | monocular canonical skeleton estimator |
| `FaceEstimator` | face mesh + expression + head pose | head pose + basic expressions |
| `GazeEstimator` | eye crop model | head-directed gaze |
| `AudioVisualLipEstimator` | causal audio/visual model | visual mouth or audio visemes |
| `HandEstimator` | hand localization + 3D hand mesh | wrist + coarse finger curl |
| `PropEstimator` | guitar/prop detector and pose | disabled prop mode |
| `ViewFusionResidual` | set transformer residual | geometry-only fusion |
| `TemporalStateEstimator` | causal cached transformer/state model | deterministic filter/state machine |
| `ContactEstimator` | learned contact head | kinematic heuristic |
| `RetargetCorrection` | avatar-conditioned residual | analytical IK only |

### RA-066 — Require a degraded but defined fallback for optional learned modules

A learned residual or specialist model shall not be the sole path to any safety-critical state. Geometry-only fusion, analytical IK, confidence gating, and simpler filters allow the product to degrade rather than explode when a package fails or hardware is weak. [P24] [P37] [A19]

## 13.2 Task interface

Conceptual Rust interface:

```rust
pub trait TaskModule: Send {
    fn descriptor(&self) -> TaskDescriptor;

    fn prepare(
        &mut self,
        packages: &ModelPackageSet,
        runners: &dyn ModelRunnerFactory,
        context: &TaskPrepareContext,
    ) -> Result<(), TaskError>;

    fn submit(
        &mut self,
        input: TaskInput<'_>,
        deadline: Deadline,
        sink: &dyn TaskOutputSink,
    ) -> Result<SubmissionToken, TaskError>;

    fn cancel(&mut self, token: SubmissionToken);
    fn reset(&mut self, scope: ResetScope);
}
```

`TaskDescriptor` includes:

- task contract and output contract ranges;
- required and optional inputs;
- stateful cache specification;
- supported camera counts;
- supported precision profiles;
- provider requirements;
- expected latency/cost model;
- whether inputs can be batched;
- session reset semantics;
- model package dependencies;
- fallback implementation ID.

## 13.3 Module adapters

A model package does not emit domain packets directly. The task module adapter owns:

1. image crop and normalization;
2. tensor layout and shape selection;
3. inference submission;
4. output decoding;
5. coordinate conversion;
6. confidence calibration;
7. semantic validation;
8. canonical packet construction.

This is intentional. It contains model-specific glue in one place rather than spreading it across fusion, temporal, and UI code.

### RA-067 — Localize model-specific preprocessing and postprocessing

Normalization constants, crop conventions, tensor names, heatmap decoders, body-model mappings, and confidence calibration shall remain inside the task module associated with a model family. Downstream modules see only canonical observations. [A18]

## 13.4 Deterministic and learned modules

The system deliberately keeps deterministic operators as peers of learned modules:

```text
robust triangulator
camera ray builder
coordinate transform
contact constraint solver
joint-limit validator
root innovation gate
analytical IK
pose resampler
output protocol adapter
```

A learned module may propose a residual or prior. The deterministic layer enforces hard invariants and records rejected corrections.

### RA-068 — Keep final invariants outside learned models

Bone-length consistency, coordinate validity, impossible joint limits, root-translation jump limits, stale-result rejection, and output protocol constraints shall be checked after learned stages. Production ML testing requires system-level validation because model outputs alone cannot guarantee application invariants. [A19] [P37]

## 13.5 Model-family composition

A quality profile resolves task contracts to module implementations rather than naming a single giant model.

Example:

```yaml
profile_id: quality-2cam-v1
modules:
  person_region: region_tracker_v2
  fast_whole_body_2d: wholebody2d_medium_v4
  spatial_body: body_student_quality_v3
  face: face_expression_quality_v2
  gaze: gaze_small_v1
  audiovisual_lips: av_lips_causal_v1
  hands: hands_quality_v3
  view_fusion: fusion_geometry_plus_residual_v2
  temporal: temporal_quality_v4
  contact: contact_quality_v2
  retarget: analytical_ik_plus_avatar_residual_v2
```

The same graph can swap `body_student_quality_v3` for `body_student_eco_v3`, or remove the learned fusion residual, without changing the UI or output adapters.

### RA-069 — Resolve by task capability, not concrete class name

Graph profiles shall request semantic capabilities and constraints. The model registry selects a compatible installed implementation. This supports model updates, alternate licenses, and vendor-specific packages without rebuilding the entire product. [A01] [A18]

## 13.6 Opaque feature sharing

Some optimized systems may share an image encoder or intermediate features between body, face, hands, or views. This is permitted only through a co-designed `FeatureFamily` contract:

```text
producer package hash
feature family ID and major version
spatial stride
channel count
normalization
coordinate mapping
precision
memory domain
consumer allow-list
```

If an exact compatible consumer is absent, the system uses semantic observations instead.

### RA-070 — Treat shared latent features as an optimization, not the architecture

The system must remain correct when modules do not share a backbone. Shared features may reduce compute but create tight model entanglement; they require exact compatibility and cannot become the only path to composability. [A18]

## 13.7 Camera-count-agnostic ML

Fusion models consume a set of camera observation tokens plus masks. They do not use fixed positional slots named `camera1` and `camera2`. Training uses camera-count randomization and view dropout.

### RA-071 — Make fusion permutation-invariant and camera-count-agnostic

The learned fusion interface shall accept an unordered set of view observations with explicit camera geometry and timing. Set-based attention and uncalibrated multiview research support arbitrary camera counts and per-view weighting. [P19] [P20] [P21]

## 13.8 Stateful temporal modules

Temporal models expose cache tensors explicitly through the model package and task module. Reset scopes include:

- `SESSION_RESET`;
- `PERFORMER_RESET`;
- `LOCAL_LIMB_REACQUIRE`;
- `CAMERA_SET_CHANGED`;
- `SITE_REVISION_CHANGED`.

A temporal model cannot hide persistent state in framework globals.

### RA-072 — Make recurrent/cache state explicit and resettable

Causal temporal models depend on history, but hidden state must be inspectable, serializable for replay where appropriate, and destroyed on guest handoff. Online HMR and autoregressive motion reconstruction rely on cached state while the product requires strict person-state lifetime. [P01] [P09]

## 13.9 ML failure semantics

Every task invocation returns one of:

```text
SUCCESS
PARTIAL_SUCCESS
NO_OBSERVATION
STALE_INPUT
DEADLINE_MISSED
CANCELED
PROVIDER_FAILURE
INVALID_OUTPUT
PACKAGE_INCOMPATIBLE
```

A model error is not encoded as zero confidence plus a plausible-looking zero pose.

### RA-073 — Make absence and failure explicit

Missing evidence, low confidence, provider failure, stale input, and an anatomically invalid output are different states and require different recovery. Conflating them encourages downstream hallucination and silent failure. [A18] [A19]


# 14. Model packages and model registry

A model is distributed as a self-describing package, not as a loose `.onnx` file and a README containing hidden preprocessing assumptions.

## 14.1 Package layout

```text
body_student_quality_v3.atmodel/
├── manifest.json
├── model-card.md
├── LICENSES/
├── graphs/
│   ├── fp32/model.onnx
│   ├── fp16/model.onnx
│   └── int8/model.onnx
├── profiles/
│   ├── 384x256.json
│   ├── 512x320.json
│   └── 640x384.json
├── task/
│   ├── input-contract.json
│   ├── output-contract.json
│   ├── preprocessing.json
│   └── postprocessing.json
├── calibration/
│   ├── confidence-calibration.bin
│   └── quantization-calibration.json
├── tests/
│   ├── vectors/
│   └── tolerances.json
└── signature/
    ├── sha256sums.txt
    └── signature.ed25519
```

## 14.2 Manifest

Illustrative manifest:

```json
{
  "package_schema": "avatartrack.model-package/v1",
  "model_id": "org.avatartrack.body.student.quality",
  "model_version": "3.1.0",
  "task": "spatial-body-estimation/v2",
  "producer": "AvatarTrack project",
  "license": "Apache-2.0",
  "provenance": {
    "training_run": "run-2026-08-14-1732",
    "dataset_manifest_hash": "sha256:...",
    "teacher_versions": ["sam3dbody-teacher-adapter/1.2.0"]
  },
  "contracts": {
    "input": ["camera-frame/v2", "person-crop/v1", "fast-2d-body/v3"],
    "output": ["spatial-body-observation/v2"]
  },
  "variants": [
    {
      "id": "fp16-512x320",
      "graph": "graphs/fp16/model.onnx",
      "input_profile": "profiles/512x320.json",
      "precision": "fp16",
      "estimated_vram_mb": 860,
      "provider_allow": ["ort-cuda", "ort-tensorrt", "ort-directml", "ort-migraphx", "ort-openvino"],
      "provider_deny": [],
      "state_tensors": []
    }
  ],
  "validation": {
    "test_vector_set": "tests/vectors",
    "tolerances": "tests/tolerances.json"
  },
  "feature_families": [],
  "minimum_runtime": "0.4.0"
}
```

### RA-074 — Require self-describing model packages

Every production model shall declare task contracts, tensor profiles, preprocessing, state tensors, precision variants, provider compatibility, memory estimates, provenance, licenses, test vectors, and limitations. Model Cards and ML production-readiness research support shipping operating characteristics and validation material with models. [A19] [A20]

## 14.3 Model registry responsibilities

`ModelRegistry`:

- validates archive structure, hashes, signature, and licenses;
- checks task-contract compatibility;
- registers variants independently;
- executes package test vectors on candidate providers;
- records benchmark distributions, not just one timing;
- selects package by task, profile, device, and policy;
- stores compiled provider caches separately from immutable package content;
- exposes model status through the public API;
- supports rollback to the previous package;
- refuses activation when output parity or latency gates fail.

### RA-075 — Separate installation from activation

A package may be installed and inspected without becoming active. Activation requires contract validation, test-vector parity, and profile-specific benchmarks. This prevents an update from replacing a working model before it proves compatible on the user's hardware. [A19] [P60]

## 14.4 Provider parity

Each model package includes reference inputs and outputs. The runtime executes them through every claimed provider/precision combination and checks:

- tensor shape and finite values;
- semantic decoder output;
- maximum and percentile numerical differences;
- pose-space angular differences;
- confidence calibration;
- avatar-space movement differences after downstream solve;
- latency and memory.

The final criterion matters because a small tensor difference can produce a visible IK or contact-state change.

### RA-076 — Validate providers in avatar space

Provider and quantization parity shall be measured not only at raw tensor outputs but after semantic decoding and representative avatar retargeting. The companion technical architecture explicitly optimizes perceived avatar behavior, while model-conversion and production-quantization research show that conversion and reduced precision can introduce practical output differences. [P60] [P53]

## 14.5 Model updates

Model metadata and binaries are versioned independently of the application. Update channel rules:

- signed index;
- explicit package licenses;
- delta or full download;
- install to a new content-addressed location;
- validate before activation;
- retain previous active package for rollback;
- never download or activate during a live performance unless explicitly requested;
- update UI shows model-card limitations and required runtime version.

### RA-077 — Make model rollback atomic

Activating a model version shall update one registry pointer only after validation. Rollback restores the previous pointer without mutating package files. ML models create system-level dependencies and need the same release discipline as code. [A18] [A19]

## 14.6 Package licensing and provenance

Because the project relies on public data, public models, synthetic generation, and teacher distillation, every package must identify:

- source datasets and their licenses;
- whether source clips were used as pixels, derived motion, or evaluation only;
- teacher checkpoints and licenses;
- synthetic assets and motion sources;
- code commit and config hash;
- prohibited uses or redistribution constraints;
- demographic and capture-condition evaluation coverage.

### RA-078 — Ship dataset and model documentation together

Each released package shall include a Model Card and link to machine-readable dataset manifests/datasheets. Dataset and model documentation are necessary to communicate composition, recommended use, performance conditions, and limitations. [A20] [A21]

---

# 15. Multi-camera composition

Multi-camera support is a graph-composition property, not a separate product codebase.

## 15.1 Per-camera subgraph

For each active camera:

```text
CameraSource
    -> TimestampMapper
    -> FrameQuality
    -> PersonRegionTracker
    -> FastWholeBody2D
    -> CropPlanner
    -> optional scheduled SpatialBody
    -> optional Face/Hand/Prop crops
    -> CameraObservationBundle
```

`CameraObservationBundle` is keyed by camera ID and source timestamp. A `CameraSetAggregator` maintains the latest usable bundle per camera and produces time-aligned fusion windows.

### RA-079 — Instantiate camera pipelines from one template

The graph shall create identical per-camera subgraphs from a camera-node template, with capabilities selected from the device and quality profile. No source code path named `if camera_count == 2` may define the core architecture. Set-based multiview methods support arbitrary view counts. [P19] [P20] [P21]

## 15.2 One-camera composition

```text
per-camera observation
    -> monocular spatial hypothesis
    -> temporal estimator
    -> root/contact safeguards
    -> retarget
```

Geometry nodes are present but operate in monocular mode. Scale is locked per performer session. Confidence reflects unresolved depth ambiguity.

## 15.3 Two-camera composition

```text
camera A bundle ----\
                      -> time alignment -> ray construction -> robust triangulation
camera B bundle ----/                                  |
                                                       +-> learned set residual
spatial hypotheses ------------------------------------|          |
                                                                  v
                                                     fused body observation
```

Heavy spatial body inference can run:

- on both views at a reduced staggered rate;
- primarily on the highest-information view while both views run fast 2D;
- dynamically switched according to visibility and uncertainty.

## 15.4 N-camera composition

Additional cameras add tokens and geometric constraints. Compute scaling is controlled by:

- fast 2D on every useful frame;
- cap on simultaneous heavy-body views;
- per-region view selection for face/hands/feet;
- staggered heavy submissions;
- view-quality and novelty score;
- camera sleep or low-rate mode when redundant;
- fused uncertainty determining whether another view is worth processing.

### RA-080 — Scale expensive work sublinearly with camera count

The system shall not run every expensive model on every camera at every output tick. Extra cameras primarily supply complementary 2D evidence, crops, and occasional spatial hypotheses. View scheduling follows expected information gain and the companion architecture's sublinear camera strategy. [P23] [P24]

## 15.5 Dynamic camera arrival and loss

The graph treats cameras as keyed producers. When a camera is unplugged:

1. capture adapter emits `CAMERA_LOST`;
2. aggregator removes its tokens after an age threshold;
3. fusion uncertainty increases;
4. quality controller may increase spatial inference on remaining views;
5. session enters `DEGRADED` only if minimum profile requirements are no longer met;
6. output continues using remaining evidence and temporal state;
7. UI receives remediation.

When a camera returns, it must match the calibrated device identity and Site State health check before rejoining fusion. A replacement camera requires calibration update.

### RA-081 — Make camera loss local before global

Loss or corruption of one view shall remove that view from fusion rather than reset the full body or session. Uncertainty-aware multiview fusion supports per-view weighting and graceful missing-view behavior. [P19] [P23]

## 15.6 Cross-camera feature independence

Per-camera task modules may differ by device quality. A low-resolution side camera may run only fast body/hand observations while a front camera runs face and spatial body. All outputs normalize to the same semantic contracts.

### RA-082 — Permit heterogeneous per-camera capability

A camera is not required to execute the same model set as every other camera. The graph selects useful tasks by resolution, angle, current visibility, and compute budget, while fusion consumes normalized observations. This is necessary for mixed cheap webcams and sublinear scaling. [P23] [P24]

## 15.7 Calibration revision boundary

Every camera observation carries the Site State revision used to interpret it. Fusion rejects observations from mixed calibration revisions. A shadow calibration may be computed in the background, but it is activated only at a controlled session boundary unless an explicit seamless transition has been validated.

### RA-083 — Never fuse mixed site geometry silently

Observations transformed by different camera calibration revisions shall not enter one geometric solve. Calibration is part of the data's meaning, not merely mutable configuration. [P15] [P16]

---

# 16. Avatar import, profiling, and retargeting composition

## 16.1 Importer ports

```rust
pub trait AvatarImporter: Send + Sync {
    fn format(&self) -> AvatarFormatDescriptor;

    fn import(
        &self,
        source: &mut dyn BlobReader,
        options: &ImportOptions,
    ) -> Result<ImportedAvatar, AvatarImportError>;
}
```

Built-in first target: VRM 1.0. Additional importers may support VRM 0.x, glTF humanoids, and selected Unity/FBX conversion workflows where licensing and implementation are practical.

Imported content is normalized into:

```text
AvatarAsset
    renderable asset and metadata

AvatarProfile
    canonical bone mapping
    rest transforms and axes
    semantic proportions
    joint limits
    feet and support geometry
    facial expression mapping
    gaze mapping
    optional finger mapping
    output tracker roles
    retarget policy
    validation report
```

VRM defines standardized humanoid and expression concepts, which makes it a suitable first canonical avatar import format. [A17]

### RA-084 — Keep asset parsing separate from motion retargeting

An importer shall produce a normalized Avatar Profile; the realtime retargeter consumes that profile. The retargeter shall not parse VRM/glTF files during tracking. This isolates file-format changes from realtime animation logic. [A01] [A17]

## 16.2 Avatar analysis pipeline

```text
uploaded asset
    -> format validation and resource limits
    -> skeleton extraction
    -> semantic-bone mapping
    -> rest-pose and axis normalization
    -> proportion analysis
    -> feet/support geometry
    -> facial expression and gaze mapping
    -> canonical motion probes
    -> auto-repair suggestions or hard failure
    -> Avatar Profile
```

Canonical probes include:

- neutral stand;
- arm raise and cross-body reach;
- squat and kneel;
- hip rotation;
- toe/heel articulation;
- head/gaze extremes;
- basic finger curl;
- facial expression channels.

The backend generates a report that the UI can visualize.

### RA-085 — Validate an avatar before a live session

An Avatar Profile is `READY` only after canonical probes verify bone mapping, bend directions, limits, foot placement, expression channels, and nonfinite transforms. Retargeting research demonstrates that skeleton topology and geometry require explicit handling rather than raw bone copying. [P36] [P39]

## 16.3 Retarget layer decomposition

```text
CanonicalPerformanceFrame
    -> semantic normalization
    -> avatar proportion adaptation
    -> contact-preserving target construction
    -> neural or heuristic initialization
    -> analytical constrained IK
    -> balance/root adjustment
    -> expression/gaze mapping
    -> final invariant validation
    -> AvatarPoseFrame
```

The optional learned retarget correction proposes target-space residuals conditioned on Avatar Profile features. Analytical IK and contact constraints remain authoritative.

### RA-086 — Make retargeting an explicit replaceable module

The human-tracking pipeline shall end at a canonical performance frame. Avatar-specific scale, bone topology, stylization, and IK belong in a separate retarget module. Skeleton-aware motion-retargeting research supports explicit topology- and geometry-aware adaptation. [P36] [P39]

## 16.4 Arbitrary avatar sizes

Avatar normalization uses ratios and semantic landmarks rather than assuming human proportions. The retarget profile defines:

- performer-normalized body coordinates;
- target limb reach envelopes;
- head/torso/limb proportion ratios;
- pelvis and foot scale policy;
- contact priority;
- bend-plane preferences;
- motion exaggeration limits;
- model-specific collision or garment constraints when available.

The output is judged by whether the avatar visibly performs the same action, not whether every target bone matches human meters.

### RA-087 — Optimize action semantics and contacts across proportions

Retargeting shall preserve hand targets, foot contacts, gaze, balance, and recognizable action before centimeter correspondence. The product's target is stylized-avatar fidelity; retargeting research supports semantic and geometry-aware residual correction. [P39]

## 16.5 Preview independence

The frontend preview receives:

- the standard avatar asset or a backend-served preview asset;
- Avatar Profile mapping metadata safe for rendering;
- final `AvatarPoseFrame`.

It never duplicates the authoritative retarget solve. If the web renderer's spring-bone or material simulation differs from VRChat, that difference is a preview issue, not tracking state.

### RA-088 — Stream final avatar transforms, not raw human observations, to the default UI

The standard preview should show what output adapters receive. Raw canonical poses remain a debug stream. This prevents the UI from presenting a falsely good or bad retargeting result. [A04] [P39]

---

# 17. OS- and GPU-neutral inference layer

The companion technical architecture defines ONNX Runtime as the default cross-platform execution host and prohibits CUDA as an application dependency. This document maps that decision into repository and API boundaries.

## 17.1 Inference interfaces

```rust
pub trait InferenceBackend: Send + Sync {
    fn descriptor(&self) -> BackendDescriptor;
    fn enumerate_devices(&self) -> Result<DeviceInventory, InferenceError>;
    fn compile(
        &self,
        variant: &ModelPackageVariant,
        profile: &CompileProfile,
    ) -> Result<CompiledModelHandle, InferenceError>;
    fn submit(
        &self,
        model: &CompiledModelHandle,
        bindings: &[TensorBinding<'_>],
        deadline: Deadline,
    ) -> Result<SubmissionHandle, InferenceError>;
    fn poll(&self, submission: &mut SubmissionHandle) -> PollResult;
    fn cancel(&self, submission: SubmissionHandle);
}

pub trait ModelRunner: Send {
    fn descriptor(&self) -> ModelRunnerDescriptor;
    fn bind(
        &mut self,
        model: CompiledModelHandle,
        schema: &TensorSchema,
    ) -> Result<(), InferenceError>;
    fn run(
        &mut self,
        tensors: NamedTensorSet<'_>,
        deadline: Deadline,
    ) -> Result<SubmissionToken, InferenceError>;
    fn collect(
        &mut self,
        token: SubmissionToken,
    ) -> Result<OwnedNamedTensorSet, InferenceError>;
}
```

The task module uses `ModelRunner`; only model-registry and inference-scheduler code use `InferenceBackend` directly. The `ModelRunner` is intentionally mutable and owned by one inference worker. The current Rust ONNX Runtime binding exposes mutable session execution because some execution-provider allocators and statistics paths are not safe for concurrent use through one session; the architecture therefore uses one exclusive runner per session or an explicit pool rather than a global lock around all inference. [A31]

### RA-089 — Keep provider APIs below `InferenceBackend`

CUDA streams, DirectML objects, TensorRT engines, MIGraphX programs, OpenVINO compiled models, and provider-specific allocators shall not appear in task-module or domain crate APIs. ONNX Runtime's execution-provider architecture exists specifically to map graphs to hardware-specific accelerators behind a common runtime. [A16]

## 17.2 Default implementations

Recommended provider adapters:

```text
inference_ort_core
    common ONNX Runtime session and graph handling

provider_windows_ml or provider_directml
    Windows broad hardware path

provider_cuda
    NVIDIA general ONNX Runtime path

provider_tensorrt / TensorRT RTX plugin
    NVIDIA optimized path where package/provider compatibility is proven

provider_migraphx
    AMD Linux/ROCm path

provider_openvino
    Intel CPU/GPU/NPU path

provider_cpu
    compact fallback and test reference

optional provider_vulkan
    future broad-GPU compact-model path
```

Current provider availability is discovered at runtime. No UI screen assumes that one provider exists because the GPU brand string matches.

### RA-090 — Select providers per task model

Face, hand, spatial body, temporal, and fusion models may use different providers when that produces a better deadline and memory result. Provider selection shall not be one global application setting. ONNX Runtime partitions and executes models through provider-specific accelerators, while model behavior and support vary by graph. [A16]

## 17.3 Device and memory abstractions

```text
DeviceId
    backend/provider-scoped device identity

MemoryDomain
    CPU | CPU_PINNED | PROVIDER_LOCAL | SHARED_HOST

DeviceBufferHandle
    opaque buffer, size, format, owner, synchronization token

TensorView
    name, dtype, shape, strides, memory domain, buffer slice
```

The initial implementation may use CPU/pinned-host transitions between providers. Zero-copy cross-provider interop is an optimization added only after correctness and measured benefit.

### RA-091 — Prefer explicit copies over fake portability

The abstraction shall expose memory domain and copy cost rather than pretend every provider buffer is interchangeable. Distributed and heterogeneous boundaries have real latency and ownership semantics. [A05] [P56]

## 17.4 Backend and extension boundaries

First-party inference support is implemented as ordinary Rust workspace crates selected by the production composition root. The initial product does not define a dynamic in-process Rust plugin ABI. Rust does not promise a stable trait-object or compiler ABI across independently built dynamic libraries, so passing `dyn Trait`, Rust enums, `String`, `Vec`, or allocator-owned buffers across that boundary is prohibited.

The default ONNX Runtime adapter uses a safe Rust wrapper over ONNX Runtime's stable C API. All `unsafe` code, raw pointers, provider handles, and lifetime reconstruction stay inside the adapter crate. The rest of the system sees safe Rust handles and typed errors. ONNX Runtime itself and GPU vendor runtimes may contain native C or C++ internally; that does not introduce first-party C++ source or a C++ ABI into this repository. [A16] [A31]

Third-party model providers and plugins run out of process through the versioned Protobuf contract by default. If a measured future requirement makes an in-process extension necessary, it may expose a minimal `#[repr(C)]` function table implemented in Rust, with fixed-width values and opaque handles. That exception requires an Architecture Decision Record, fuzzed boundary tests, and a clear performance result that justifies losing process isolation.

### RA-092 — Do not publish a dynamic Rust ABI

First-party provider implementations shall be Rust crates composed into the daemon. Third-party providers shall use the model-worker process. A stable C-compatible table written in Rust is permitted only after measured need; a C++ ABI and cross-library Rust trait objects are never supported. This keeps provider integration separate from task semantics without creating an unstable binary contract. [A01] [A05]

## 17.5 Startup benchmark

For each candidate model variant/provider/device combination:

1. compile and warm;
2. run package test vectors;
3. measure median, p95, and p99 latency;
4. measure host/device memory;
5. measure copy and synchronization cost;
6. verify no hidden large CPU fallback;
7. execute a short representative pipeline replay;
8. store result keyed by compatibility identity.

The quality recommender uses these measurements, camera count, requested output rate, and safety margin.

### RA-093 — Benchmark integrated task paths

A provider is accepted based on task-level and pipeline-level latency, not isolated graph throughput. Tail latency and data-transfer overhead can dominate visible responsiveness. [A03] [P58] [P59]

## 17.6 OS capture and packaging adapters

Windows and Linux share all domain, engine, application, ML task, calibration, fusion, temporal, retarget, output, and API code. OS-specific code is limited to adapters and packaging.

| Capability | Windows adapter | Linux adapter |
|---|---|---|
| Camera capture | Media Foundation; optional GStreamer/FFmpeg fallback | V4L2; GStreamer/FFmpeg fallback |
| Audio capture | WASAPI | PipeWire/PulseAudio/ALSA adapter |
| Desktop shell | Tauri/WebView2 reference | Tauri/WebKitGTK reference |
| GPU providers | Windows ML/DirectML, CUDA/TensorRT, OpenVINO | CUDA/TensorRT, MIGraphX/ROCm, OpenVINO, optional Vulkan |
| Local bootstrap | named pipe + permission-restricted file fallback | Unix domain socket + permission-restricted file fallback |
| Packaging | signed installer/portable bundle | AppImage/Flatpak/native packages as feasible |

### RA-094 — Keep OS conditionals in adapters and composition

Core crates shall not contain scattered `#[cfg(target_os = "windows")]` branches for product logic. Platform-specific capture, audio, bootstrap, filesystem, and provider loading live behind ports. This is information hiding around likely change. [A01]

---

# 18. Plugin architecture

Plugins extend integrations and specialist observations without allowing third-party code to redefine core state semantics. The project implements a feature as a built-in module first unless an existing external integration already requires isolation. A plugin point becomes stable only after at least one real external consumer or two current implementations prove the contract. Do not publish a general plugin API to preserve hypothetical flexibility.

## 18.1 Plugin classes

| Plugin class | Examples | Default isolation | Realtime data |
|---|---|---|---|
| Output adapter | OSC variant, game integration, recording exporter | Out of process | final pose only |
| Prop/instrument observer | guitar, microphone, controller, drumsticks | Out of process; shared memory optional | selected frames/crops + pose |
| Avatar importer | non-VRM humanoid format | Out of process | asset bytes, no live frames |
| Camera adapter | unusual industrial/phone stream | Trusted or out of process | frames |
| Diagnostic exporter | custom metrics/dashboard | Out of process | aggregated telemetry |
| Model package | alternate ONNX weights and manifests | No executable plugin required | through task modules |
| Inference provider | new accelerator backend | First-party Rust crate or model worker | tensors |
| UI extension | deferred | sandboxed web component if ever added | public API only |

### RA-095 — Prefer data plugins over code plugins for models

A new model should normally be a signed Model Package consumed by an existing task module, not an executable plugin. Executable extension is reserved for genuinely new preprocessing, task semantics, devices, or protocols. This reduces supply-chain and compatibility risk. [A18] [A20]

## 18.2 Plugin manifest

```toml
schema = "avatartrack.plugin/v1"
id = "org.example.guitar-observer"
name = "Example Guitar Observer"
version = "0.3.0"
plugin_type = "prop_observer"
executable = "bin/guitar-observer"
api_min = "1.2.0"
api_max_exclusive = "2.0.0"

[permissions]
pose_canonical = "read"
camera_crops = ["hands", "upper_body", "prop_roi"]
audio_features = "read"
raw_camera_frames = "deny"
network = "deny"
filesystem = "plugin_private"

[capabilities]
observations = ["prop-observation/guitar/v1", "contact-suggestion/v1"]
max_input_fps = 30
supports_shared_memory = true
```

## 18.3 Plugin handshake

1. Plugin host validates manifest and user-granted permissions.
2. Host starts process with an inherited local IPC endpoint and one-time token.
3. Plugin sends `Hello` with ID, version, API range, capabilities, and package hash.
4. Host replies with granted permissions, session-scoped resource IDs, stream descriptors, and deadlines.
5. Plugin must emit health heartbeats.
6. Host may suspend, restart, or quarantine it.

No plugin discovers or connects to the daemon's full local API without explicit authorization.

### RA-096 — Make plugin permissions explicit and least-privilege

Plugins shall request only the observations and capabilities they need. Access to raw frames, audio, network, or filesystem is denied by default. Plugin isolation is a security and privacy boundary, not merely an extension mechanism. [A05]

## 18.4 Plugin data path

Low-rate control and observations use protobuf RPC/messages. High-rate images or crops use a shared-memory ring:

```text
Daemon creates shared-memory region and slots
    -> sends descriptor over authenticated IPC
    -> producer writes slot and commits sequence
    -> consumer reads immutable slot before expiry
    -> control channel carries metadata and backpressure
```

Slots are preallocated. The host can revoke the mapping. The plugin never receives a daemon pointer or provider GPU handle in v1.

### RA-097 — Use shared memory only for measured high-bandwidth needs

Control and semantic observations should remain ordinary IPC messages. Shared memory is introduced for selected frame/crop paths where copies are proven material, with explicit sequence and lifetime rules. Preallocated ring-buffer approaches reduce allocation and contention, but they increase correctness complexity. [A06]

## 18.5 Plugin contribution rules

A prop/instrument plugin may emit:

- object pose observation with uncertainty;
- keypoints or object-relative landmarks;
- probable contact suggestions;
- audio event probabilities;
- region-of-interest requests;
- quality/visibility scores.

It may not directly set avatar bones or root translation. Core fusion, temporal state, contact, and retarget modules decide how to use the observation and apply final safety guards.

### RA-098 — Plugins propose observations; core owns final state

Third-party observers shall contribute evidence, not bypass the authoritative state estimator or final retarget validation. This preserves consistent confidence, contact, root, and safety semantics across extensions. [A04] [A18]

## 18.6 Plugin failure behavior

On timeout, crash, invalid packet, excessive CPU/GPU use, or permission violation:

1. stop accepting observations;
2. age out existing plugin evidence;
3. continue using core pipeline;
4. emit a structured health event;
5. optionally restart with backoff;
6. quarantine after repeated failure;
7. never reset the whole performer unless the plugin was explicitly required by the active profile.

### RA-099 — Make extension failure non-catastrophic

Optional plugins shall fail locally. Their evidence expires through the same uncertainty/age mechanism as a lost camera. A distributed component can fail independently and must not be treated like an in-process infallible object. [A05]


# 19. Recording, deterministic replay, and debugging

Recording and replay are core architecture, not optional developer conveniences. Without them, every model and pipeline change requires reproducing a physical performance with the same cameras, timing, avatar, and failure conditions.

## 19.1 Recording container

Working extension: `.atrace`.

```text
session.atrace/
├── manifest.json
├── site-state.pb
├── avatar-profile.pb
├── graph-plan.pb
├── model-lock.json
├── device-lock.json
├── streams/
│   ├── camera-1.frames/            # optional by privacy tier
│   ├── camera-1.fast2d.pbstream
│   ├── spatial-body.pbstream
│   ├── fused-body.pbstream
│   ├── canonical-performance.pbstream
│   ├── avatar-pose.pbstream
│   ├── tracking-quality.pbstream
│   └── events.pbstream
├── indexes/
│   └── time-index.bin
└── checksums.txt
```

A real implementation may use one chunked archive rather than a directory. The logical contents and schemas remain the same.

### RA-100 — Make every major stage recordable through taps

The engine shall permit recording at selected semantic boundaries without changing the producing or consuming module. Stage taps receive immutable packets and run on noncritical queues. Deterministic replay is necessary to test modular substitutions and system-level ML behavior. [A19] [A18]

## 19.2 Recording manifest

The manifest records:

- application commit/build;
- API and recording schema versions;
- Site State and Avatar Profile revisions;
- active graph plan;
- exact model package hashes and variants;
- provider/device/runtime versions;
- quality profile and adaptive changes;
- packet taps enabled;
- privacy tier and consent timestamp;
- start/end monotonic and wall-clock times;
- dropped packet counts;
- whether raw frames are complete, sampled, redacted, or absent.

## 19.3 Replay modes

### Exact downstream replay

Use recorded canonical packets to test retargeting, outputs, UI preview, or protocol adapters without perception.

### Module replacement replay

Use recorded upstream packets and replace one module:

```text
recorded FastWholeBody2D
    -> candidate fusion
    -> original downstream graph
```

### Full inference replay

Use recorded camera frames with original timestamps to run the full graph. The scheduler can run:

- realtime pacing;
- faster-than-realtime offline;
- deterministic single-threaded test mode;
- injected stalls and dropped frames.

### Comparative replay

Run baseline and candidate graphs from the same trace and compare:

- joint/contact/root trajectories;
- failure events;
- latency simulation;
- final avatar pose;
- perceptual render outputs.

### RA-101 — Make replay time authoritative

Modules shall read time through an injected `Clock` trait, not directly from operating-system time. Replay provides a virtual monotonic clock so timing, age, cache, and timeout behavior can be reproduced. [A02] [A05]

## 19.4 Determinism levels

| Level | Guarantee | Use |
|---|---|---|
| `STRUCTURAL` | same graph, packet ordering, and resource revisions | broad integration tests |
| `NUMERICAL_TOLERANCE` | outputs within provider-specific tolerances | GPU/provider parity |
| `BITWISE_CPU_REFERENCE` | exact outputs for deterministic CPU operators | geometry and state-machine unit tests |
| `PERCEPTUAL_EQUIVALENCE` | final avatar metrics/render ratings within gate | release evaluation |

GPU inference is not assumed bitwise deterministic across providers. Tests use semantic tolerances and failure-rate gates.

### RA-102 — Define determinism at the appropriate layer

The project shall not claim bitwise determinism where providers cannot guarantee it. Geometry, state machines, resource transitions, and CPU reference operators should be exact; model outputs use validated semantic tolerances. [P60] [A19]

## 19.5 Fault injection

Replay can inject:

- camera frame drops and bursts;
- timestamp jitter and drift;
- temporary camera blackout;
- corrupted keypoint confidence;
- delayed spatial inference;
- GPU out-of-memory;
- provider reset;
- plugin crash;
- output socket loss;
- site-calibration revision mismatch;
- avatar profile replacement attempt;
- long shared occlusion;
- repeated guest handoff.

### RA-103 — Test degraded operation deliberately

Release tests shall include partial failures and asynchronous stalls, not only clean recordings. A modular realtime system must prove that one camera, model, plugin, or client failure remains contained. [A03] [A05] [A19]

## 19.6 Developer inspection tools

`tools/replay-ui` or `apps/dev-viewer` can display synchronized timelines:

```text
camera frames and crops
2D observations and covariance
spatial body hypotheses
triangulated joints and residuals
fusion weights
temporal prediction/innovation
contact state
root corrections
canonical pose
avatar pose
scheduler queues and deadlines
failure guards
```

Selecting a joint shows its provenance over time.

---

# 20. Training, synthetic data, and evaluation repository

The runtime and ML repository are connected by contracts and packaging, not by shared implementation. The ML area can evolve rapidly while the production daemon remains stable.

## 20.1 Python package structure

```text
ml/avatartrack_ml/
├── contracts/
│   ├── generated/                  # protobuf/JSON-schema bindings
│   ├── canonical_pose.py
│   ├── camera.py
│   └── tensor_contracts.py
├── datasets/
│   ├── catalog.py
│   ├── adapters/
│   ├── licensing.py
│   ├── sequence_record.py
│   ├── transforms/
│   ├── sampling/
│   └── validation/
├── synthetic/
│   ├── blender_addon/
│   ├── scene_generator/
│   ├── camera_randomization/
│   ├── performer_randomization/
│   ├── avatar_randomization/
│   ├── clothing_and_props/
│   ├── webcam_corruption/
│   ├── guitar/
│   └── render_farm/
├── teachers/
│   ├── body_hmr/
│   ├── multiview_optimizer/
│   ├── face/
│   ├── hands/
│   ├── contacts/
│   └── motion_extraction/
├── models/
│   ├── fast2d/
│   ├── spatial_body/
│   ├── view_fusion/
│   ├── temporal/
│   ├── face/
│   ├── gaze/
│   ├── av_lips/
│   ├── hands/
│   ├── contacts/
│   └── retarget/
├── training/
│   ├── loops/
│   ├── distributed/
│   ├── checkpointing/
│   └── experiment_registry/
├── distillation/
├── losses/
├── evaluation/
│   ├── metrics/
│   ├── scenarios/
│   ├── render_compare/
│   ├── blind_test_exports/
│   └── hardware_profiles/
├── export/
│   ├── onnx/
│   ├── quantization/
│   ├── parity/
│   └── simplify/
└── packaging/
    ├── manifest_builder.py
    ├── model_card.py
    ├── sign.py
    └── publish.py
```

## 20.2 Canonical `SequenceRecord`

All dataset adapters yield a common record or a subset with explicit missing fields.

```text
sequence_id
license/provenance
performer split identity, pseudonymous
motion category
camera array:
    intrinsics/extrinsics/distortion
    timestamps and frame references
    image-quality metadata
world/floor
canonical body pose and shape, optional
face state, optional
hand state, optional
prop pose and contacts, optional
visibility and occlusion
2D labels
audio features/reference, optional
avatar target examples, optional
quality flags
```

### RA-104 — Normalize datasets at the semantic boundary

Dataset-specific skeletons, camera conventions, and annotation formats shall be converted by adapters into `SequenceRecord`. Training models should not each contain private parsers and coordinate conversions for every source. Hidden data dependencies and pipeline jungles are major ML technical-debt risks. [A18]

## 20.3 Dataset catalog and licensing

Every source has a machine-readable catalog entry:

```yaml
id: amass
version: local-release-id
source_url: ...
license_id: ...
allowed_uses:
  training: true
  redistribution_raw: false
  redistribution_derivatives: review
contains_pixels: false
contains_biometrics: true
required_attribution: ...
adapter: avatartrack_ml.datasets.adapters.amass
splits: data/splits/amass-v1.yaml
notes: ...
```

A training run resolves exact dataset versions and writes a dataset manifest hash into checkpoints and model packages.

### RA-105 — Treat license and provenance as executable metadata

The data loader shall refuse a dataset for an incompatible release/training policy rather than relying on developer memory. Datasheets for Datasets motivate documenting source, composition, collection, and recommended uses. [A21]

## 20.4 Public VTuber clips

Online VTuber footage is not assumed to reveal the original performer's physical ground truth. It can support:

- motion-distribution extraction;
- gesture and timing priors;
- face/expression timing where resolution permits;
- style and perceptual-reference mining;
- evaluation scenarios;
- rendered-avatar pose extraction followed by synthetic re-rendering.

Each clip must be processed according to platform terms, copyright, creator permission where required, and dataset release policy. The preferred derived artifact is a motion sequence with provenance, not a redistributed video corpus.

### RA-106 — Separate motion-distribution use from physical-ground-truth use

Final VTuber output clips shall not be labeled as direct ground truth for the hidden human performer. They may provide avatar motion priors or source motions that become exact labels only after retargeting into a known synthetic scene. [P36] [P39]

## 20.5 Synthetic generation architecture

Synthetic generation is a deterministic job graph:

```text
motion source
    -> canonical motion cleanup
    -> performer body sample
    -> face/hand augmentation
    -> clothing and hair sample
    -> room/desk/chair/prop sample
    -> avatar target sample
    -> 1..N camera rig sample
    -> temporal offset/drop/exposure sample
    -> render passes
    -> cheap-webcam corruption
    -> exact labels and visibility
    -> SequenceRecord shard
```

Render passes may include:

- RGB clean and corrupted;
- segmentation and instance IDs;
- optical flow;
- depth and normals;
- 2D/3D joints;
- body/hand/face parameters;
- camera matrices and distortion;
- contact labels;
- prop geometry;
- floor/support labels;
- occlusion maps;
- motion blur vectors.

BEDLAM and synthetic multiview datasets establish that synthetic humans and exact camera labels can support human reconstruction, while domain randomization supports transfer to real imagery. [P45] [P47] [P52]

### RA-107 — Generate camera, timing, occlusion, and contact labels synthetically

Synthetic data shall be the primary source of exact supervision for arbitrary camera count, uncalibrated placement, unsynchronized timing, contact, prop interaction, and cheap-webcam corruption. These labels are expensive or unavailable in public real footage. [P45] [P47] [P52]

## 20.6 Blender generator modules

```text
blender_addon/core
    scene lifecycle, deterministic seeds, render orchestration

blender_addon/humans
    bodies, skin, hair, clothing, rig retargeting

blender_addon/motions
    AMASS/Motion-X/derived VTuber motion import and cleanup

blender_addon/cameras
    N-camera placement, lens, distortion, rolling shutter approximation

blender_addon/environments
    rooms, desks, chairs, beds, clutter, lighting

blender_addon/props
    guitar, microphone, controller, instrument geometry and contacts

blender_addon/avatars
    target skeleton sampling and retarget validation

blender_addon/labels
    exact skeleton, face, hands, contacts, visibility, camera state

blender_addon/corruption
    noise, compression, sharpening, exposure, blur, dropped frames
```

Each rendered sequence stores the generator commit, seed, asset hashes, and full parameters so failures can be reproduced.

### RA-108 — Make synthetic sequences reproducible by seed and asset hash

Every synthetic sample shall be regenerable from a manifest containing code version, random seed, assets, motion, camera configuration, and corruption parameters. Dataset debugging and model regression are impossible when generation is opaque. [A21] [A19]

## 20.7 Teacher/student organization

Teacher pipelines are offline and may combine:

- SAM-class body reconstruction;
- multiview optimization;
- robust triangulation;
- temporal smoothing with future context;
- body-model fitting;
- hand/face specialist models;
- physics/contact constraints;
- synthetic ground truth;
- consensus across public checkpoints.

Student packages target realtime task contracts. Distillation losses may cover outputs, confidence, intermediate semantics, contacts, and temporal state. Knowledge distillation supports training smaller models from larger teachers. [P51]

### RA-109 — Keep teacher outputs in canonical contracts

Teacher pipelines may be complex, but their training targets shall be stored in canonical semantic form with provenance and uncertainty. Student code must not depend on one teacher's private tensor layout. [P51] [A18]

## 20.8 Training configuration

Training runs are declared in versioned configs:

```yaml
experiment: temporal-quality-v4
seed: 1732
code_commit: auto
model:
  family: causal_temporal
  width: 384
  layers: 8
  cache_frames: 120
inputs:
  contracts:
    - fused-body-observation/v2
    - face-observation/v2
    - hand-observation/v2
    - contact-suggestion/v1
data:
  manifest: data/catalogs/train-2026-08.json
  camera_count_distribution: [0.15, 0.45, 0.25, 0.10, 0.05]
  view_dropout: 0.30
  timestamp_jitter_ms: 18
losses:
  pose: 1.0
  velocity: 0.2
  jerk: 0.05
  contact: 0.4
  foot_slide: 0.6
  root_innovation: 0.3
export:
  task_contract: temporal-performance/v3
```

Config schemas reject unknown or missing required fields. A run copies the resolved config into the artifact directory.

### RA-110 — Version and validate experiment configuration

Training configuration is code-like input and shall be schema-validated, resolved, and archived. Configuration debt is a documented source of hidden ML-system debt. [A18]

## 20.9 Evaluation packages

Every candidate model is evaluated in layers:

1. task metrics;
2. temporal and failure-event metrics;
3. full pipeline replay;
4. final avatar-space metrics across avatar proportions;
5. low-quality webcam corruption suites;
6. unseen-person and person-handoff tests;
7. one/two/N-camera suites;
8. dance and guitar scenarios;
9. provider parity;
10. blind perceptual comparison exports.

### RA-111 — Gate models on full-pipeline avatar behavior

A lower per-joint error does not automatically justify release if the model increases foot skating, root jumps, finger hallucination, or latency. Candidate packages shall pass task and end-to-end avatar-space gates. [P12] [P13] [A19]

## 20.10 Export boundary

The only supported path from training to runtime is:

```text
checkpoint
    -> scripted export
    -> ONNX graph validation
    -> task-contract adapter validation
    -> quantization/precision variants
    -> reference and provider parity
    -> model package construction
    -> signature
    -> registry installation test
```

Developers do not manually copy a checkpoint into the application resources folder.

### RA-112 — Automate model export and packaging

Model export shall be a reproducible pipeline with conversion tests. Empirical work on model conversion finds frequent challenges, so manual one-off export is not acceptable for production packages. [P60] [A19]

## 20.11 Experiment isolation and promotion

Each ML or data subsystem may own an ignored `experiments-scratch/` directory. Scratch code may import maintained modules, but maintained packages never import scratch code. Scratch experiments receive no stable API guarantee. They add no root task-runner commands, package entry points, daemon flags, production profiles, UI controls, or release artifacts.

A winning experiment is promoted through these steps:

1. State the user-visible or release-metric gain.
2. Reproduce it with a checked-in config or compact command owned by the maintained tool.
3. Move only the required implementation into the maintained package.
4. Replace the old production path.
5. Delete or archive the scratch implementation and obsolete compatibility code.
6. Update model cards, manifests, release gates, and measured hardware results.

Historical experiments may remain as short notes with commit, data manifest, seed, hardware, result, and conclusion. They do not continue to build against current APIs.

## 20.12 Deployment-facing calibration

Synthetic data may train the model at scale. Every deployment-facing confidence temperature, uncertainty scale, contact threshold, root gate, quality claim, and automatic profile decision must be fitted or verified on a versioned real-camera calibration cache before packaging. The cache may use public data, opt-in community recordings, and locally recorded non-redistributed fixtures according to their licenses.

The repository tracks the cache manifest, split, metric code, fitted values, and packaging command. It does not track the raw cache when redistribution is not permitted. A model package carries the fitted calibration artifact and provenance hash.

---

# 21. Configuration and quality profiles

The system has several configuration scopes. Keeping them separate prevents a single giant settings file from becoming an implicit API.

## 21.1 Configuration scopes

| Scope | Example | Owner | Live mutable |
|---|---|---|---:|
| Product defaults | privacy defaults, log retention | application | some |
| Site | active cameras, calibration revision, floor | Site State | limited |
| Avatar | profile, expression map, retarget preferences | Avatar Profile | limited |
| Quality profile | module variants, rates, resolutions, budgets | Model/quality registry | selected knobs |
| Session | active site/avatar/profile/output IDs | Session resource | state-dependent |
| Adaptive runtime | current heavy-view schedule, reduced preview | Quality controller | yes |
| Developer overrides | debug taps, fake clocks, fault injection | dev/test only | yes |

### RA-113 — Do not use one untyped global settings map

Settings shall belong to typed scopes with validation, revisions, and lifecycle. A global key/value bag creates hidden dependencies and configuration debt. [A18]

## 21.2 Quality profile schema

```yaml
schema: avatartrack.quality-profile/v1
id: quality-2cam
label_key: quality.quality_2cam
requirements:
  min_cameras: 2
  max_cameras: null
  min_total_camera_fps: 50
  required_tasks:
    - fast-whole-body-2d/v3
    - spatial-body-estimation/v2
    - temporal-performance/v3
budgets:
  output_hz_desktop: 60
  output_hz_vr: 90
  target_body_observation_hz: 28
  target_face_hz: 45
  target_hand_hz: 40
  gpu_memory_mb: 6500
  median_body_latency_ms: 75
  p95_body_latency_ms: 100
scheduling:
  max_heavy_views_in_flight: 1
  stagger_heavy_views: true
  protect_face: true
  preview_fps: 12
module_preferences:
  spatial_body: [body_quality_v3, body_balanced_v3]
  fusion: [fusion_residual_v2, fusion_geometry]
  temporal: [temporal_quality_v4, temporal_balanced_v4]
adaptive_limits:
  min_body_observation_hz: 18
  min_face_hz: 30
  allow_hand_detail_reduction: true
  allow_preview_disable: true
```

## 21.3 User-facing quality selection

The public UI presents a small number of modes and camera choices. Advanced per-module controls belong in a developer panel. The backend recommendation returns:

```text
recommended profile
expected observation/output rates
expected provider/device allocation
memory headroom
camera placement warnings
features reduced relative to maximum
reasons and alternatives
```

### RA-114 — Keep the user-facing knob coarse and backend resolution detailed

Users select intent such as Eco, Balanced, Quality, Performance, or Maximum. The backend resolves that intent into model variants, rates, provider choices, and scheduling based on measured hardware. Once-for-All and elastic-model research support specializing model capacity for deployment conditions. [P55]

## 21.4 Adaptive controller

The controller may adjust only within profile limits. It observes:

- output deadline misses;
- GPU/CPU utilization and memory;
- queue ages;
- motion intensity and uncertainty;
- camera quality;
- face/hand visibility;
- thermal or provider health where exposed.

Priority order under overload:

1. preserve output tick;
2. preserve face/head responsiveness;
3. preserve fast 2D observation;
4. reduce preview/debug;
5. reduce hand detail where evidence is weak;
6. reduce spatial resolution/rate;
7. cap heavy views;
8. enter explicit degraded mode rather than silently stalling.

### RA-115 — Keep adaptation bounded by a declared profile

The quality controller shall not make arbitrary model substitutions or semantic changes. Every permitted adaptation and fallback is declared and tested in the profile. Dynamic control is valuable only when behavior remains understandable and well-conditioned. [A02] [A19]

## 21.5 Configuration migrations

Persistent resources carry schema versions. Migrations are:

- explicit and ordered;
- reversible where practical;
- tested on representative old fixtures;
- performed before resource activation;
- never applied to the only copy without backup;
- independent from public API version.

---

# 22. Build system, CI, packaging, and updates

## 22.1 Build-system decision

The first-party runtime uses a Cargo workspace as its authoritative build graph. The repository does **not** use Bazel in the initial architecture.

Cargo workspaces already provide the properties this Rust-heavy product needs first: one dependency resolution, one `Cargo.lock`, shared target output, common workspace commands, shared package metadata, and workspace-level profiles and lints. The root workspace uses resolver version 3 and the Rust 2024 edition. [A22] [A23]

Bazel can build Rust through `rules_rust`, and `crate_universe` can translate Cargo dependencies into Bazel targets. That is a real option for a later large repository. It also creates another build graph and another integration layer around Cargo packages, build scripts, generated schemas, editor support, Python, and TypeScript. For a public project whose first challenge is realtime tracking rather than build-farm scale, that cost is not justified yet. This is an engineering inference from Cargo's native workspace model and the official `rules_rust`/`crate_universe` workflow. [A22] [A30]

### RA-134 — Use Cargo as the authoritative Rust build

The root `Cargo.toml` and committed `Cargo.lock` define all first-party Rust crates and dependencies. No `BUILD`, `BUILD.bazel`, or `MODULE.bazel` files are maintained. The project shall not keep Cargo and Bazel as competing sources of truth. [A22] [A30]

### RA-135 — Reconsider Bazel only after measured build pain

An Architecture Decision Record may propose Bazel only when the repository has measured one or more of these failures: unacceptable clean-build time despite normal Rust caching, a required remote-execution workflow, unmanageable cross-language code-generation drift, or a release reproducibility problem Cargo plus locked tool environments cannot solve. The proposal must include a prototype on Windows and Linux, editor support, dependency update workflow, GPU-provider packaging, and a deletion plan for the replaced build path. Until then, do not add Bazel files. [A30]

## 22.2 Canonical environments and root commands

The project has one declared environment for each owned toolchain. Contributors may use another environment, but a result is authoritative only when reproduced in the owning environment.

### Canonical environments

- Pin the Rust stable toolchain, formatter, Clippy, source component, and compilation targets in `rust-toolchain.toml`.
- Run Linux runtime checks in the pinned development container or an equivalent locked host toolchain.
- Run Windows camera, Windows ML/DirectML, packaging, and installer checks on a pinned native Windows toolchain. Do not assume cross-compilation validates Media Foundation or GPU-provider behavior.
- Use the root `pyproject.toml` and `uv.lock` for Python data, training, export, and model packaging.
- Use the root `pnpm-lock.yaml` and root TypeScript lint policy for frontend packages.
- Pin `buf`/`protoc` generation tools through the development environment or a checked tool manifest.
- Record driver, runtime, execution-provider, model-package, and power-mode versions for provider-specific performance jobs.

The root `justfile` is the sole cross-project task runner. `just --list` shows maintained commands and their arguments. Each recipe delegates to the tool that owns the work. It does not reimplement Cargo, `uv`, `pnpm`, or schema-generation logic.

Recommended stable root commands:

```text
just bootstrap
just build
just check
just replay-smoke
just package-model MODEL=...
just package-app PROFILE=...
just release-gates PROFILE=...
```

Do not add aliases that wrap a command used once. Keep research, data-acquisition, migration, and maintenance commands in the owning tool's README and package entry point.

## 22.3 First-party Rust policy

These rules apply to first-party Rust. Follow upstream style under `external/` and do not copy external native source into first-party crates.

### Language and crate boundaries

- Use the Rust 2024 edition on the pinned stable toolchain.
- Keep all shipping runtime, daemon, CLI, plugin-host, model-worker, platform-adapter, and desktop-shell code in Rust.
- Do not add first-party `.cc`, `.cpp`, `.cxx`, `.hpp`, or CMake files.
- A dependency may contain native code. Isolate it in one adapter crate and expose a safe Rust API.
- Keep `#![forbid(unsafe_code)]` in domain, application, engine policy, tracking modules, public transport mapping, and retargeting crates unless a current required operation makes that impossible.
- Permit `unsafe` only in narrowly owned FFI, SIMD, memory-mapping, platform, or GPU adapter modules. Every unsafe block states the invariant that makes it safe. The crate README states the external ABI, ownership, thread, and shutdown assumptions.
- Do not expose raw pointers, provider handles, OS handles, or foreign allocator ownership outside the adapter crate.

### Ownership and types

- Prefer owned domain values and explicit immutable snapshots at semantic boundaries.
- Use borrowing inside a synchronous call when the callee cannot retain the value.
- Use `Arc<T>` only when multiple concurrent owners are real. Do not use it as a default replacement for ownership design.
- Use newtypes for IDs, timestamps, durations, coordinate frames, revisions, and units whose interchange would be a bug.
- Use fixed-width integers for wire formats, storage, image/tensor dimensions with defined limits, and hardware fields whose width matters.
- Use slices and iterators instead of copying collections on hot paths.
- Use `bytes::Bytes`, `Arc<[u8]>`, or an owned buffer handle for immutable shared frame payloads. Do not pass `Vec<u8>` through every stage when ownership is shared.
- Avoid mutable global state. Process-global third-party runtime state is contained in the adapter and initialized once through an explicit composition path.

### Errors, panics, and shutdown

- Library crates return typed errors. Application binaries may add context at process or operation boundaries.
- Do not use `unwrap` or `expect` for camera input, model input, API input, filesystem state, provider state, or any condition a user or device can trigger.
- `unwrap` or `expect` is acceptable in tests and for static construction invariants that have no runtime input. The message states the invariant.
- A module reports an expected tracking failure as data or a typed error. It does not panic to control a frame path.
- A panic is a defect. Process and worker boundaries record it and fail the affected process or session rather than trying to continue with unknown state.
- Use explicit cancellation tokens and join owned threads/tasks during shutdown. Do not detach realtime workers.

### Async and realtime execution

- Use Tokio for HTTP, WebSocket, file/network control-plane I/O, operation progress, and non-realtime orchestration. Tokio provides async I/O, scheduling, timers, and bounded channels for that domain. [A26]
- Do not run the output-critical body tick, camera capture callback, geometry solve, or deterministic retarget loop as arbitrary Tokio tasks. Use dedicated named OS threads or controlled worker pools with explicit priority, affinity where supported, bounded channels, and measured deadlines.
- Use bounded channels between realtime stages. `crossbeam-channel` supports bounded queues and nonblocking send failure, which lets the engine implement latest-only and drop/coalesce policies rather than accumulating latency. [A27]
- Do not hold a mutex across inference, I/O, callback invocation, or a channel send.
- Use atomics only for small state and counters with documented ordering. Prefer channels or owned state over hand-built lock-free structures.
- Keep the API runtime and realtime engine connected through narrow command/event channels. A blocked API client cannot stall tracking.

### Traits, generics, and dynamic dispatch

- Add a trait only at a current replaceable boundary, external capability, replay seam, or dependency inversion point.
- Use concrete types inside a module until there is a second current implementation or a net complexity reduction.
- Use `dyn Trait + Send + Sync` at runtime-selected boundaries such as camera sources, storage adapters, output adapters, and inference backends.
- Use generics for local zero-cost algorithms where the caller chooses the type at compile time. Do not propagate generic parameters across the entire application graph merely to avoid one measured-insignificant virtual call.
- Do not expose Rust trait objects across dynamic-library or process boundaries.
- Avoid marker-trait forests, type-state APIs, macro-generated service layers, and generic wrappers that obscure the dataflow without a current safety or performance result.

### Names, modules, and documentation

- Use standard Rust naming: `UpperCamelCase` types and traits, `snake_case` functions/modules/variables, and `SCREAMING_SNAKE_CASE` constants.
- Keep crate roots small. Re-export only the current cross-crate surface.
- Order modules for top-down reading. Put the public operation or type first and the deepest private helper last where Rust item-order constraints permit it.
- Document units, coordinate frames, timing, ownership, thread affinity, reset behavior, and safety invariants.
- Do not add boilerplate file banners or comments that restate code.

### Formatting, lint, dependency, and documentation checks

- Format the entire workspace with `cargo fmt --all --check`. Rustfmt supports workspace formatting through Cargo. [A28]
- Run `cargo clippy --workspace --all-targets --all-features -- -D warnings` in CI. Clippy documents this workflow and warning escalation. [A29]
- Define workspace lint levels in the root `Cargo.toml`. Crates may tighten them. A crate may relax one only with a local reason.
- Run dependency source, license, duplicate, and advisory policy through the root dependency-policy tool. Do not scatter allowlists across crates.
- Build API documentation with `cargo doc --workspace --no-deps`; treat broken intra-doc links as errors.

## 22.4 Python, TypeScript, schemas, and generated code

### Python

- Keep training, data, export, and evaluation behavior in importable packages.
- Keep notebooks and one-off scripts outside authoritative paths.
- Use typed dataclasses or schema-generated types at dataset, artifact, and package boundaries.
- Do not create a generic utility module when a function has one caller.
- Keep data transforms explicit. Avoid global registries whose behavior depends on import order.

### TypeScript

- The frontend imports only generated public API clients, public stream schemas, and frontend-owned types.
- Do not duplicate server enums, error codes, capability names, or resource fields by hand.
- Keep backend-derived state in one frontend state package.
- A UI component does not own product workflow state or backend retry policy.
- The desktop shell may start and discover `avatartrackd`, but it calls the same loopback API as any other client and does not link backend Rust crates.

### Generated code

- Protobuf bindings, OpenAPI clients, JSON-schema validators, documentation, and model-package types are reproducible from checked-in contracts.
- Generate Rust Protobuf types through `prost` or another selected generator behind one checked command. `prost` produces idiomatic Rust from proto2/proto3 definitions. [A32]
- Generated code is not hand-edited.
- Check generated source into the repository only when bootstrapping or downstream distribution requires it.
- CI regenerates checked-in outputs and fails on drift.

### RA-116 — Make code generation deterministic and CI-verified

Generated clients, Protobuf bindings, API docs, schema validators, and model-package types shall be reproducible from checked-in contracts. CI regenerates and fails on drift. Language-neutral interfaces are reliable only when generated code matches the authoritative schema. [A07] [A08] [A32]

## 22.5 Cargo workspace and crate model

The root is a virtual workspace with resolver 3, shared package metadata, shared dependency versions, workspace lint policy, and named build profiles. Cargo workspaces share a lockfile and output directory and allow common commands across all members. [A22]

Conceptual root manifest:

```toml
[workspace]
resolver = "3"
members = [
  "crates/foundation",
  "crates/domain",
  "crates/engine",
  "crates/application",
  "crates/ports",
  "crates/modules/*",
  "crates/adapters/*",
  "crates/composition/*",
  "crates/bins/*",
]

[workspace.package]
edition = "2024"
rust-version = "<pinned-msrv>"
license = "<project-license>"

[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
all = "warn"
perf = "warn"

[profile.release]
lto = "thin"
codegen-units = 1
panic = "abort"

[profile.realtime]
inherits = "release"
debug = 1
```

The exact profile values remain code-owned and must be benchmarked before release. Cargo supports named profiles and workspace-wide settings. [A23]

Illustrative crate ownership:

```text
avatartrack-foundation
avatartrack-domain
avatartrack-engine
avatartrack-application
avatartrack-ports
avatartrack-module-calibration
avatartrack-module-fast2d
avatartrack-module-spatial-body
avatartrack-module-fusion
avatartrack-module-temporal
avatartrack-module-contact
avatartrack-module-avatar
avatartrack-module-retarget
avatartrack-adapter-camera-mf
avatartrack-adapter-camera-v4l2
avatartrack-adapter-inference-ort
avatartrack-adapter-gpu-wgpu
avatartrack-adapter-output-vmc
avatartrack-adapter-output-vrchat
avatartrack-transport-http
avatartrack-transport-websocket
avatartrack-composition-production
avatartrackd
avatartrack-cli
```

This list is an ownership map. Create a crate only when current code needs an independent dependency boundary, platform feature, reusable library, isolated unsafe boundary, or shipped artifact. Do not create one crate per class, trait, or directory.

`tools/dependency-check` reads `cargo metadata` and enforces the documented dependency direction. Cargo features select compile-time platform/provider capabilities such as `camera-mf`, `camera-v4l2`, `ort`, or `wgpu`. Features do not select user quality modes or runtime behavior. Cargo features are conditional-compilation and optional-dependency controls, not a product configuration system. [A23]

## 22.6 GPU and native dependency packaging

The repository contains no first-party C++. It may distribute or discover external native libraries required by ONNX Runtime, execution providers, camera/media stacks, or OS APIs.

Rules:

- Keep each external runtime behind one Rust adapter crate.
- Prefer stable C APIs over C++ APIs.
- Prefer official prebuilt binaries or system packages over vendoring native source.
- Verify hashes, signatures where available, license metadata, architecture, provider version, and runtime compatibility before loading.
- Do not allow a crate `build.rs` script to download executable code from the network during a normal build.
- Package provider binaries in explicit platform bundles or install them through a documented model/runtime manager.
- Record the loaded library path and version in diagnostics.
- Fail provider activation before session start when required symbols or versions do not match.
- If a dependency only exposes a C++ API and has no stable C/process protocol, do not integrate it into the daemon. Use an out-of-process adapter maintained outside the core or choose another dependency.

For portable custom preprocessing, image conversion, compositing, and compact compute kernels, `wgpu` is the preferred optional first-party GPU layer. It is a safe Rust API that runs over Vulkan, Direct3D 12, Metal, and OpenGL; Windows and Linux can therefore share shader and resource-management code while the inference runtime remains independent. [A33]

## 22.7 CI matrix

Pull-request jobs are path-aware. A job runs when the change can affect the behavior it owns. The minimum always-on gate remains small.

### Always-on pull-request gates

| Job | Windows | Linux | GPU required |
|---|---:|---:|---:|
| changed Rust crates compile | Yes | Yes | No |
| `cargo fmt`, Clippy, and Rust docs | One host | Yes | No |
| dependency and contract generation check | One host | Yes | No |
| CPU replay smoke for changed runtime graph | Yes | Yes | No |
| model-package/schema validation when relevant | Yes or Linux | Yes | No |

### Change-owned gates

| Changed area | Required gate |
|---|---|
| public contracts | compatibility, generated-client compile, mock-client behavior |
| frontend | TypeScript checks and focused UI contract flow |
| Python/data | Ruff, schema/generation behavior, affected evaluation |
| Rust camera adapter | adapter conformance and owning OS capture fixture |
| inference adapter/provider | provider conformance, parity, deadline/failure behavior |
| unsafe boundary | Miri or sanitizer/fuzz target where applicable, plus boundary tests |
| model package | export, test vectors, provider parity, avatar-space release gate |
| installer/package | application package install/start/upgrade/uninstall check |

Scheduled or release jobs add available NVIDIA, AMD, and Intel hardware, long replay, latency, low-quality-camera, dance, guitar, and person-handoff gates. Missing hardware coverage is reported explicitly.

### RA-117 — Keep CPU reference and synthetic-mini tests mandatory

Every change that affects the runtime graph, contracts, replay, or model-package plumbing shall exercise a compact CPU reference path without a discrete GPU. GPU-specific jobs add performance and parity evidence but cannot be the only correctness path. [A19] [P60]

## 22.8 API compatibility CI

CI compares:

- OpenAPI surface against the last stable release;
- Protobuf field numbers and types;
- removed and reserved fields;
- enum additions and removals;
- plugin manifest schema;
- model-package schema;
- recording schema;
- generated client compilation.

Before stable v1, CI reports provisional breakage but does not force compatibility shims. After stable v1, a breaking public change requires a major version and an Architecture Decision Record.

### RA-118 — Treat contract compatibility as a release gate

Public API, plugin, recording, and model-package compatibility checks shall run before release. Internal Rust crate APIs remain free to change while they have no external consumer; semver promises apply only to published contracts and intentionally published SDK crates. [A07] [A08]

## 22.9 Packaging and updates

### Application packaging

- Windows receives a signed installer and optional portable bundle.
- Linux begins with a tested AppImage or portable archive; Flatpak and native packages may follow when current users require them.
- Packages include the daemon, reference UI, CLI, first-party Rust libraries, generated contracts needed at runtime, licenses, and a minimal CPU model path.
- GPU execution-provider bundles are platform- and architecture-specific. The installer or runtime manager selects compatible packages rather than shipping every provider to every user.

### Model updates

- Model packages are independently versioned and signed.
- Activation is transactional: download, verify, run test vectors, compile/warm, run replay smoke, then switch.
- Keep the last known-good package for rollback.
- Never replace an active package in place.

### Release manifest

Every release records:

- Rust toolchain and target triples;
- Cargo lockfile hash;
- application and contract versions;
- first-party crate versions or source revision;
- external native runtime hashes and versions;
- model package hashes;
- provider support matrix;
- signed package hashes;
- dated measured latency and release-gate hardware.

### RA-119 — Update model packages transactionally

A downloaded model or provider package becomes active only after signature, schema, test-vector, compile, and replay checks. The previous package remains available for rollback. ML artifacts carry behavior and compatibility risk beyond ordinary static assets. [A19] [A20] [P60]


# 23. Testing strategy

The test strategy protects the product with the smallest set of gates that detect user-visible failures, broken contracts, wrong generated artifacts, and incorrect published results. It does not preserve implementation structure.

## 23.1 Test admission rule

Add a test only when all conditions hold:

1. The failure would ship broken tracking, break a supported runtime or artifact contract, corrupt generation or packaging, or publish a wrong metric.
2. The test observes a wanted outcome or non-trivial algorithm.
3. No existing end-to-end, replay, contract, or release gate already catches the failure at equal or lower maintenance cost.
4. The expected result should remain stable under normal refactoring and model tuning.

Delete a test when those conditions stop holding.

Do not test:

- private call order;
- internal class or file structure;
- config echoes;
- incidental defaults;
- developer constants;
- fixture contents that are not themselves a contract;
- exact logs or English text;
- intermediate tensors that are not a declared model contract;
- expected-to-change numeric outputs from learned models;
- a bug merely because it once occurred.

Do not add production getters, hooks, flags, or public methods only to make a test possible.

## 23.2 Test layers

### Release and replay gates

These are the primary product tests. A deterministic or time-controlled trace drives the maintained graph and verifies final avatar behavior, failure-event counts, latency, state resets, and output contracts.

### Contract conformance tests

Use a shared conformance suite when a port has multiple current implementations or accepts an external implementation. Examples include camera adapters, inference providers, output adapters, avatar importers, model packages, and public API clients.

Do not create a fake implementation solely to prove that a speculative interface is replaceable.

### Algorithm tests

Test non-trivial deterministic algorithms whose failure is hard to localize through an end-to-end gate. Examples include coordinate transforms, camera rays, robust triangulation, covariance propagation, contact-state transitions, timestamp alignment, and retarget constraint math.

### Boundary integration tests

Test a cross-process, storage, package, or provider boundary when serialization, lifetime, failure, or version behavior matters. Keep the graph small and assert boundary outcomes rather than internal calls.

### UI contract tests

Run the reference UI against a generated or minimal mock of the public API. Test only maintained user workflows, capability-driven screens, operation progress, actionable errors, and reconnect behavior.

### Perceptual release tests

Render final avatar clips for maintained scenarios and use metric and blind-review gates where automated measures are insufficient. These are release evaluations, not unit snapshots.

### RA-120 — Require shared conformance tests for replaceable implementations

A current replaceable port or externally implementable task shall provide one reusable conformance suite. Every supported implementation passes the same observable contract. A seam with one concrete implementation and no external consumer does not require a fake solely to justify itself. [A01] [A19]

## 23.3 Core contract gates

### Camera contract

- sequence and source time are monotonic or explicitly marked unreliable;
- declared pixel format and dimensions match accessible memory;
- frame lifetime lasts until handle release;
- stop and reconnect meet declared behavior;
- external driver timestamps and dimensions are bounded before arithmetic or allocation;
- generation changes after reconnect;
- malformed frames fail at the adapter boundary.

### Spatial body contract

- output frame and units are correct;
- canonical joints are present or validity is explicit;
- uncertainty is finite and valid;
- stale or failed input does not emit a fresh confident pose;
- performer reset removes person-specific state;
- provider failure and invalid output are explicit;
- downstream invariant validation accepts the result.

### Temporal contract

- output at time `t` uses no observation from after `t`;
- performer reset removes prior-person influence;
- missing observations increase uncertainty and expire prediction;
- correction and root-innovation bounds hold;
- output cadence remains stable under irregular observation times.

### Retarget contract

- maintained avatar fixtures import and bind or return structured failure;
- contacts stay within the declared avatar-space tolerance;
- transforms remain finite;
- bend planes do not flip under maintained motions;
- the same canonical action remains recognizable across extreme avatar ratios.

## 23.4 Person-agnostic release gate

Calibrate the site with Performer A. Run Performers B through N with varied height, limb ratios, clothes, skin tone, mobility, movement style, face shape, hair, entry direction, sitting, standing, and dance behavior.

Measure:

- no camera recalibration required;
- acquisition time;
- quality relative to same-person setup;
- previous-performer leakage;
- anthropometry convergence;
- contact and root failure rate.

### RA-121 — Include cross-person calibration transfer in every release gate

The site-calibration architecture is successful only if unrelated people can enter and track without personalized weights or camera recalibration. Cross-person transfer is a first-class release gate. [P15] [P16]

## 23.5 Low-quality webcam release gate

Vary:

- 360p, 480p, and 720p inputs;
- 15, 24, and 30 FPS;
- inter-camera FPS mismatch;
- compression, noise, low light, sharpening, and exposure pumping;
- motion blur and rolling-shutter approximation;
- dropped frames and USB contention;
- wide and narrow field of view;
- mixed mirrored and non-mirrored sources.

Report graceful degradation and visible failure-event frequency. Do not use average joint error as the only result.

## 23.6 Dance release gate

Maintain scenarios for rapid direction changes, spins, partial back-facing periods, crossed limbs, jumps when the capture volume permits them, squats, floor transitions, rhythmic foot plants, arms crossing the torso or face, loose clothing, and temporary shared occlusion.

Measure:

- root teleports;
- knee and elbow flips;
- foot slide while contact is active;
- contact release latency;
- pose lag and oversmoothing;
- lost-joint reacquisition discontinuity;
- final avatar perceptual rating.

## 23.7 Guitar release gate

Maintain seated and standing sequences with acoustic and electric guitar shapes, varied neck angles, strumming, picking, fretting-hand occlusion, body lean, head movement, instrument-blocked torso views, and fast rhythm.

Measure:

- instrument-to-body registration;
- left and right hand contact plausibility;
- hand-side correctness;
- wrist and elbow continuity;
- gross fret-region and strum timing;
- face and body expression continuity;
- torso and root hallucination caused by the instrument.

Guitar-specific research supports explicit bimanual instrument interaction and audio timing rather than treating the guitar as an ordinary body motion. [P42] [P43]

## 23.8 API, provider, package, and client gates

Keep only maintained combinations:

- minimum-supported client against the current additive server;
- current client against the minimum-supported server;
- operation reconnect and resume;
- duplicate idempotency key;
- stale revision conflict;
- unauthorized origin or token;
- slow pose-stream consumer;
- malformed external Protobuf frame;
- model-package install, validation, activation, and rollback;
- provider output parity in avatar space;
- installer start, upgrade, and uninstall behavior.

## 23.9 Review and audit policy

When a reviewer reports a problem instead of fixing it, the report includes:

1. the concrete behavior, contract, artifact, training result, or published number that breaks;
2. the checked failure mechanism;
3. the current call site, data structure, state lifetime, and relevant run duration;
4. the owning environment and command that reproduces it;
5. the user and maintainer cost;
6. the fix cost, including retraining, regenerating, re-exporting, repackaging, recalibration, migration, and cache invalidation;
7. the smallest repair that restores the wanted behavior.

Do not report a difference without breakage. Do not report a missing check until current callers and boundary validation have been inspected. Do not inflate every finding to critical. Prefer a small set of findings whose fixes pay for themselves.

## 23.10 Release scorecard

The release report contains at least:

```text
API compatibility: pass/fail
Windows CPU reference: pass/fail
Linux CPU reference: pass/fail
provider parity by tested device
one-camera replay gates
2-camera replay gates
N-camera synthetic gates
cross-person transfer gates
avatar proportion fixture gates
dance failure-event rates
guitar scenario results
face and hand quality gates
median, p95, and p99 latency
privacy and security checks
known unsupported cases
```

### RA-122 — Publish failure-event rates, not only average accuracy

A tracker that is usually accurate but randomly flips a knee or teleports the root is unacceptable for this market. Releases shall report catastrophic and visible failure frequencies alongside average metrics. Avatar artifact research supports evaluating signal artifacts and perceived animation fidelity. [P12] [P13]

---

# 24. Observability and latency attribution

## 24.1 OpenTelemetry-compatible instrumentation

The backend uses trace IDs and spans across API, operations, engine stages, model submissions, and output adapters. OpenTelemetry defines traces, metrics, and logs with context propagation across components and processes. [A10]

Example trace:

```text
camera.exposure [driver timestamp]
  capture.dequeue
  preprocess.fast2d
  inference.fast2d
  crop.plan
  inference.spatial_body
  fusion.geometry
  fusion.residual
  temporal.update
  contact.solve
  retarget.avatar
  output.vmc.send
```

The final pose packet carries a sampled trace ID linking it back to evidence.

### RA-123 — Propagate trace context through packets and process boundaries

Every packet and plugin/RPC request shall carry trace context where sampling is enabled. This allows end-to-end latency and causality to be reconstructed across stages and processes. [A10]

## 24.2 Metrics

### Capture

- actual FPS and interarrival distribution;
- exposure-to-dequeue estimate;
- dropped frames;
- timestamp quality;
- blur, exposure, noise, compression scores.

### Engine

- queue depth and age by edge;
- work drops/coalescing;
- stage median/p95/p99;
- deadline misses;
- graph generation and session state.

### Inference

- compile and warmup time;
- submission latency;
- device execution time where available;
- copy/synchronization time;
- memory high-water mark;
- provider fallback/partitioning;
- invalid output count.

### Tracking quality

- joint confidence/uncertainty;
- reprojection residual;
- camera contribution weights;
- contact probabilities;
- root innovation rejections;
- prediction age;
- failure-guard count;
- reacquisition events.

### Outputs

- output cadence;
- socket/send errors;
- consumer backlog;
- final pose age at send.

### RA-124 — Measure age at every boundary

A stage timing is insufficient without knowing the age of the evidence it processed. Metrics shall include source-to-stage and source-to-output age distributions. [A03] [P01]

## 24.3 Structured logs

Logs contain:

```text
timestamp
severity
component
stable event code
resource/session/camera/model/plugin IDs
trace ID
structured fields
human summary
```

Raw images, audio, face embeddings, and full pose streams are never written to ordinary logs.

## 24.4 Local diagnostics database

A bounded rolling diagnostics store retains aggregated metrics and notable events. It supports graphs such as:

- why Quality mode downgraded;
- which camera is adding useful geometry;
- which provider causes p99 stalls;
- why a foot unlock occurred;
- whether a camera moved;
- which joint is prediction-dominated;
- whether UI preview is slow while tracking remains healthy.

## 24.5 User-facing health model

The UI should not show hundreds of raw counters by default. Backend derives health resources:

```text
System: READY | DEGRADED | ERROR
Camera: GOOD | POOR_LIGHT | BLURRY | TIMING_UNSTABLE | LOST
Site: VALID | MOVED_CAMERA | WEAK_BASELINE | FLOOR_UNCERTAIN
Tracking region: OBSERVED | PREDICTED | DEGRADED | LOST
Output: CONNECTED | RETRYING | DISABLED | ERROR
```

Each state includes evidence and remediation.

### RA-125 — Derive health in the backend

Health semantics and thresholds shall be shared by UI, CLI, logs, and automation. The frontend may visualize them but shall not independently invent readiness rules from raw metrics. [A01] [A10]

## 24.6 Diagnostic bundles

A diagnostic report can include:

- application and provider versions;
- capability snapshot;
- resource schemas and sanitized configs;
- Site State quality without raw camera images;
- model/package hashes;
- benchmark summaries;
- event timeline;
- aggregated metrics;
- optional user-approved short redacted trace.

The API shows exactly what will be included before export.

---

# 25. Security, privacy, and trust boundaries

## 25.1 Trust zones

```text
TRUSTED CORE
    daemon, built-in modules, first-party providers

TRUSTED CLIENT WITH TOKEN
    reference UI, CLI

LIMITED PLUGIN
    out-of-process, explicit permissions

UNTRUSTED CONTENT
    imported avatars, model packages before validation,
    recordings, public datasets, network downloads

EXTERNAL OUTPUT DESTINATION
    VRChat/VMC/OSC consumer, optional remote API client
```

### RA-126 — Validate at every trust-boundary entry

API payloads, plugin messages, imported assets, model packages, recording files, and network output configuration shall be schema-validated and resource-limited before use. Cross-process and untrusted-content boundaries cannot rely on in-process assumptions. [A05]

## 25.2 Local API security

- loopback bind by default;
- random bearer token per launch;
- strict Origin checking for browser clients;
- short bootstrap-token lifetime;
- no wildcard CORS;
- request body and upload limits;
- endpoint authorization scopes;
- rate limits for expensive operations;
- remote mode disabled by default;
- remote mode requires TLS and durable credentials.

## 25.3 Asset import security

Avatar and model importers enforce:

- archive expansion limits;
- maximum file and texture sizes;
- maximum node/bone/material counts;
- path traversal rejection;
- no executable scripts;
- no network fetches embedded in assets;
- finite numeric transforms;
- safe image decoding;
- temporary directory isolation;
- content hashes before durable installation.

## 25.4 Model-package trust

A model package contains data graphs, manifests, and test vectors—not arbitrary Python code. Built-in task modules interpret it. Packages from untrusted sources remain disabled until validation. Signed official packages use a project key; community packages are marked separately.

### RA-127 — Do not execute arbitrary package code

The standard model-package format shall not include Python modules, shared libraries, shell scripts, or custom runtime operators by default. New task semantics or custom operators require a separately reviewed plugin/provider path. This keeps model updates closer to data than executable supply-chain updates. [A18]

## 25.5 Plugin security

- explicit permission grant;
- no inherited full environment secrets;
- private working directory;
- network denied by default;
- raw frames denied by default;
- resource quotas and heartbeat;
- protocol schema validation;
- process termination and quarantine;
- clear UI indicator when a plugin sees camera/audio data.

## 25.6 Performer privacy

- no identity recognition requirement;
- ephemeral track IDs;
- no persistent face template by default;
- no runtime personalization weights;
- performer and temporal state destroyed on handoff;
- raw data storage opt-in;
- public dataset and opt-in community data policies documented;
- diagnostic sharing shows a manifest before export.

### RA-128 — Keep identity unnecessary to tracking

The system may maintain an ephemeral active-person association, but it shall not require recognition, account linkage, or persistent biometric identity. Geometry and short-term continuity are sufficient for the one-person capture space. [P15] [P16]

## 25.7 Output safety

Output adapters receive the final validated pose. They cannot request raw model outputs or disable joint/root safety guards. A plugin output may transform protocol coordinates but cannot rewrite the authoritative avatar pose inside the engine.

---

# 26. End-to-end workflows

This section shows how the layers cooperate without violating boundaries.

## 26.1 First launch

```text
1. Desktop shell starts avatartrackd with bootstrap endpoint.
2. Daemon initializes storage migrations and minimal API.
3. Daemon enumerates adapters/providers in background.
4. UI reads bootstrap token and calls GET /v1/system.
5. UI calls GET /v1/system/capabilities.
6. Backend starts or retrieves a benchmark Operation.
7. UI displays capability-driven setup, not vendor-specific assumptions.
8. Daemon installs/validates bundled model packages.
9. QualityService publishes recommended profiles.
```

Realtime engine is not started merely to open the UI.

## 26.2 Import an avatar

```text
UI                              API/Application                   Backend modules

choose file
POST /uploads ----------------> UploadService
PUT bytes --------------------> content-addressed temp blob
POST /avatars ----------------> AvatarService -> Operation
                                                                   importer
                                                                   profile builder
                                                                   motion probes
<----- operation events ------------------------------------------ validator
GET /avatars/{id} <------------ READY + Avatar Profile revision
```

The UI can close and reconnect while the operation continues.

## 26.3 Calibrate a site once

```text
POST /sites
PATCH selected camera modes
POST /sites/{id}/calibration-runs

Backend builds calibration graph:
    capture -> fast2d/masks -> track collector -> time aligner
    -> camera graph solver -> articulated refinement -> floor solver
    -> held-out validation

Backend emits prompts:
    visibility check
    A/T pose
    turn left/right
    step or sway
    crouch/arm motion if needed

Successful result:
    Site State revision written persistently
    temporary calibration performer variables discarded
```

A later guest uses the same Site State.

## 26.4 Start a new guest

```text
POST /sessions {
    site: "sites/demo-room",
    avatar: "avatars/fox-vrm",
    quality_profile: "quality-2cam",
    outputs: ["outputs/vrchat"]
}
POST /sessions/{id}:start
```

Backend:

1. validates current camera identity and site health;
2. resolves model packages/providers;
3. builds and warms graph;
4. starts capture and fast observers;
5. acquires the single active person;
6. estimates ephemeral proportions/neutral face;
7. locks stable performer ratios;
8. starts final output;
9. emits `TRACKING`.

No camera calibration or weight update occurs.

## 26.5 Guest handoff at a demo

```text
POST /sessions/{id}:resetPerformer
```

Backend:

- pauses final output or holds neutral pose;
- clears performer and temporal state;
- retains Site State, Avatar Profile, graph packages, and warm provider sessions;
- reacquires the next person;
- estimates new ephemeral proportions;
- resumes output.

The warm graph keeps handoff fast without leaking prior-person state.

### RA-129 — Separate graph warm state from performer state

Compiled models, memory arenas, Site State, and Avatar Profile may persist across guests, while all person-dependent and temporal caches reset. This supports demo throughput without specialization to one person. [P15] [P16]

## 26.6 Live camera loss

```text
camera 2 capture error
    -> CameraHealth event
    -> camera 2 observations age out
    -> fusion falls back to remaining cameras
    -> temporal uncertainty grows for affected joints
    -> QualityController reallocates heavy body rate
    -> session state becomes DEGRADED if necessary
    -> output continues
    -> UI suggests reconnect/reposition
```

No frontend decision is required to prevent a pose explosion.

## 26.7 Quality-mode change

A live-safe change within the profile envelope:

```text
PATCH /sessions/{id}
    quality_override.preview_fps = 5
```

A material profile change:

```text
POST /sessions/{id}:changeProfile { profile: "performance-3cam" }
    -> Operation
    -> shadow graph build and warm
    -> validation
    -> atomic switch
```

## 26.8 Install a candidate model

```text
upload package
POST /models:install
    -> signature/hash/schema/license validation
    -> test vectors on CPU reference
    -> package registered INACTIVE
POST /models/{id}:benchmark
    -> provider parity and pipeline replay
POST /models/{id}:activate
    -> update task-resolution policy after gates
```

The active session does not change until a controlled graph replacement.

## 26.9 Guitar plugin

```text
Plugin requests:
    canonical upper-body/hand pose
    selected hand and guitar ROI crops
    audio event features

Plugin emits:
    GuitarPoseObservation
    object-relative hand landmarks
    probabilistic contact suggestions

Core:
    validates uncertainty and age
    fuses guitar evidence
    constrains contacts
    performs final avatar solve
```

The plugin cannot directly set the avatar wrist or root.

## 26.10 Headless VRChat use

```text
avatartrackd --config profile.json
avatartrack-cli session create ...
avatartrack-cli session start ...

Daemon captures, tracks, and sends OSC/VMC.
No UI process is needed.
```

## 26.11 UI replacement

A community developer generates a client from OpenAPI and protobuf schemas, implements setup and preview in a different framework, and controls the same daemon. No fork of capture, ML, or output code is needed.

---

# 27. Composability matrix

| Desired change | Stable boundary used | What remains unchanged |
|---|---|---|
| Replace desktop UI | Public HTTP/WS API | daemon, models, outputs, storage |
| Add CLI automation | Public API | UI and engine |
| Add a camera backend | `CameraSource` port | calibration, perception contracts, UI |
| Add a third camera | keyed camera subgraph + set fusion | public API and downstream skeleton |
| Swap body model | `SpatialBodyEstimator` contract + package | fusion, temporal, retarget, UI |
| Use a smaller GPU profile | model variant + quality profile | task semantics and outputs |
| Add AMD/Intel backend | `InferenceBackend` plugin | task modules and model contracts |
| Change fusion model | `ViewFusionResidual` contract | cameras, temporal, retarget |
| Remove learned fusion | geometry fallback | public behavior and graph semantics |
| Replace temporal model | `TemporalStateEstimator` contract | per-view perception and outputs |
| Add guitar support | prop-observer plugin + contact contracts | body model and public session API |
| Add a new avatar format | `AvatarImporter` | canonical performance and retarget core |
| Add an output protocol | `OutputAdapter` or plugin | tracking graph and UI |
| Debug without cameras | recording/replay contracts | modules under test |
| Train with a new dataset | dataset adapter -> `SequenceRecord` | training loops and export contract |
| Replace teacher model | canonical teacher targets | student task and runtime package |
| Quantize one model | package variant + parity tests | task module and downstream contracts |
| Change storage engine | repository ports | API/application/domain |
| Run as kiosk/demo | same daemon API + guest reset | tracking modules and site calibration |
| Add remote controller | secure API transport mode | engine and frontend reference app |

## 27.1 Why this composition works

### Stable semantics, unstable implementations

The system assumes that the meaning of `FusedBodyObservation` changes less often than the network architecture used to produce it. It therefore stabilizes semantic meaning and lets implementations vary.

### One owner per state

Modules can be swapped because they do not secretly own copies of Site State, Avatar Profile, or session lifecycle.

### Dataflow instead of direct object webs

A new stage consumes a packet contract and emits another. It does not need references to every upstream and downstream class.

### Optional learned residuals

Geometry, temporal safety, contacts, and IK provide a baseline. Learned components improve it without becoming impossible-to-debug authorities over every state.

### Contract-generated clients

Frontend and integrations consume generated types and functions rather than copying endpoint strings or backend structs.

### Replay as integration currency

A module author can iterate on fusion, temporal behavior, or retargeting using the same recordings and expected outputs as the rest of the project.

### RA-130 — Judge composability by localized replacement

A seam is composable when a current alternate implementation, external adapter, replay source, or provider can replace it without changing consumers or semantic contracts. Do not create a fake implementation solely to justify an interface. Composability is lower change cost and localized replacement, not interface count. [A01] [A19]

---

# 28. Explicit anti-patterns

The following designs are prohibited or require an ADR documenting a compelling exception.

## 28.1 Frontend anti-patterns

- Tauri command directly calls `Tracker::start()`.
- UI opens webcams itself.
- UI decides calibration success from image overlays.
- UI maintains the authoritative session state.
- UI publishes OSC/VMC each frame.
- UI imports internal Rust domain types.
- UI reads SQLite or model files directly.
- UI assumes NVIDIA/AMD/Intel features from device names.
- calibration steps are hard-coded in React components.

## 28.2 Backend anti-patterns

- a wrapper, factory, service, or interface with one current caller and no net complexity reduction;
- an API field, CLI option, config key, database column, or compatibility shim with no current reader;
- retaining both old and replacement production paths after a decision;
- one giant `Tracker` class owning cameras, models, UI callbacks, and outputs;
- global singleton service locator;
- synchronous chain `capture -> model -> model -> output` on one thread;
- unbounded queues;
- every node starts its own thread;
- background preview blocks output;
- session state inferred from which pointers are non-null;
- direct SQL inside application services;
- API handler mutates engine internals;
- camera-count branches duplicated across the product.

## 28.3 ML anti-patterns

- experiment-only model, flag, endpoint, profile, or package entry on a maintained surface;
- abstract base class for every model family before a second current implementation exists;
- loose ONNX file with undocumented preprocessing;
- model-specific output type passed into temporal/retargeting code;
- one package includes arbitrary Python executed at runtime;
- latent feature tensor becomes a public plugin contract;
- training notebook is the only export pipeline;
- dataset license tracked in a spreadsheet outside the run manifest;
- quality mode is one giant model with no fallback;
- model update activates before provider parity;
- low confidence represented as a zero pose;
- runtime guest calibration fine-tunes weights.

## 28.4 Plugin anti-patterns

- third-party dynamic library loaded into the daemon through an unstable Rust ABI;
- plugin receives unrestricted raw frames by default;
- plugin directly writes avatar bones;
- plugin crash resets the session;
- plugin-specific UI calls private backend methods;
- plugin data lacks timestamps, confidence, and contract version.

## 28.5 Data anti-patterns

- committed raw datasets, downloaded clips, generated shards, caches, or training checkpoints;
- deployment thresholds fitted only on synthetic data;
- scraping clips without provenance or use policy;
- calling final VTuber video hidden-human ground truth;
- mixing train/test performers or motion sources accidentally;
- synthetic data without seed/asset manifest;
- evaluation only on clean renders;
- retaining raw guest demo footage by default.

### RA-131 — Reject glue-code shortcuts that cross stable boundaries

Code review shall treat direct cross-layer access and undocumented model/data dependencies as architecture defects even when they reduce initial code. ML systems accumulate hidden debt specifically through glue code, pipeline jungles, and boundary erosion. [A18]

---

# 29. Implementation sequence

The sequence below is designed for a serious open-source spare-time project. It validates architecture and product risk without requiring every model to exist first.

## Phase 0 — Contracts and skeleton

Create only the directories and targets required by this phase. Do not scaffold later phases.

Deliver:

- repository scaffold;
- dependency-rule checker;
- public API v1 draft;
- event and pose protobuf schemas;
- headless daemon with `/system` and `/capabilities`;
- generated TypeScript client;
- mock backend;
- reference UI shell;
- CLI health command;
- ADR process.

Exit condition: UI and CLI can connect to a fake/headless daemon without linking implementation code.

## Phase 1 — Capture, packets, and replay

Deliver:

- Windows and Linux camera ports/adapters;
- timestamped `CameraFrame` contract;
- graph engine and bounded queues;
- preview tap;
- recording container v1;
- virtual clock and replay;
- camera health metrics.

Exit condition: recorded multi-camera frames replay identically through timing and preview paths.

## Phase 2 — Avatar and output baseline

Deliver:

- VRM 1.0 importer;
- Avatar Profile;
- canonical skeleton;
- analytical retarget/IK baseline;
- frontend preview from final pose stream;
- VMC and VRChat OSC output adapters;
- canonical motion probe validator.

Exit condition: synthetic canonical motion drives varied avatars and outputs without perception.

## Phase 3 — One-camera baseline

Deliver:

- person crop and fast 2D;
- initial spatial-body model adapter;
- deterministic temporal/filter baseline;
- root/contact safety baseline;
- person reset and state separation;
- API session workflow.

Exit condition: one camera produces stable full-body avatar motion through the complete headless path.

## Phase 4 — Site calibration and two-camera geometry

Deliver:

- calibration-run API and prompt events;
- time alignment;
- camera graph solve;
- floor and scale;
- robust triangulation;
- per-view uncertainty;
- camera placement score;
- cross-person calibration tests.

Exit condition: two-camera replay materially reduces monocular depth/root/occlusion errors with no target board in the normal UX.

## Phase 5 — Modular learned fusion and temporal student

Deliver:

- model-package registry;
- set-based fusion residual;
- causal temporal task module;
- explicit cache/reset contract;
- synthetic camera-count randomization;
- provider parity gates;
- quality profiles.

Exit condition: learned components can be replaced by baselines through config and replay comparison.

## Phase 6 — Face, gaze, lips, and hands

Deliver:

- independent face branch;
- expression semantic contract;
- gaze;
- audiovisual lips;
- hand crop/mesh branch;
- hand evidence gating;
- avatar face/finger mapping.

Exit condition: integrated output remains responsive under body load and degrades cleanly with poor hand pixels.

## Phase 7 — Guitar/prop plugin and advanced contacts

Deliver:

- plugin host and permissions;
- shared-memory crop transport;
- guitar observation/contact contracts;
- audio feature stream;
- synthetic guitar generator;
- instrument-mode evaluation.

Exit condition: plugin failure does not disrupt body tracking, and guitar interaction visibly improves when enabled.

## Phase 8 — Distilled production models and broad hardware

Deliver:

- teacher/student pipelines;
- multiple model widths and precisions;
- NVIDIA/AMD/Intel provider coverage as practical;
- startup benchmark and recommendation;
- signed model update/rollback;
- public model cards and release scorecards.

## Phase 9 — Community extension stabilization

Deliver:

- plugin SDK v1;
- contract stability policy;
- examples and templates;
- remote/headless docs;
- dataset contribution tooling;
- automated redaction and diagnostic bundles.

### RA-132 — Build architecture seams before advanced models

The project shall implement public contracts, replay, canonical skeleton, retargeting, and baseline modules before training tightly coupled custom networks. This makes later model gains measurable and prevents research code from defining accidental product architecture. [A18] [A19]

---

# 30. Interface sketches

These sketches are illustrative Rust. They define ownership and separation, not final crate names or syntax. The realtime traits remain synchronous because the engine owns their scheduling. The HTTP/WebSocket adapters may use async functions internally.

## 30.1 Camera port

```rust
#[derive(Clone, Copy, Debug)]
pub struct CameraMode {
    pub width: u32,
    pub height: u32,
    pub fps: Rational,
    pub pixel_format: PixelFormat,
}

#[derive(Clone, Debug)]
pub struct CameraDescriptor {
    pub id: CameraId,
    pub display_name: String,
    pub modes: Vec<CameraMode>,
    pub timestamp_capability: TimestampCapability,
    pub hardware_controls_available: bool,
}

pub trait CameraSource: Send {
    fn descriptor(&self) -> CameraDescriptor;
    fn configure(&mut self, config: &CameraConfig) -> Result<(), CameraError>;
    fn start(&mut self, sink: Box<dyn FrameSink>) -> Result<(), CameraError>;
    fn request_stop(&self);
    fn health(&self) -> CameraHealthSnapshot;
}
```

The Media Foundation and V4L2 crates own OS handles and unsafe calls. `CameraFrame` carries an immutable buffer handle plus timestamps. The domain crate never sees COM, file descriptors, or driver-specific pixel-buffer types.

## 30.2 Inference backend port

```rust
#[derive(Clone, Debug)]
pub struct BackendDescriptor {
    pub id: BackendId,
    pub adapter_version: SemanticVersion,
    pub devices: Vec<DeviceDescriptor>,
    pub supported_types: Vec<DataType>,
    pub memory_domains: Vec<MemoryDomain>,
}

#[derive(Clone, Debug)]
pub struct CompileProfile {
    pub device: DeviceId,
    pub variant_id: String,
    pub shapes: Vec<ShapeProfile>,
    pub precision: PrecisionPolicy,
    pub memory_budget: MemoryBudget,
}

pub trait InferenceBackend: Send + Sync {
    fn compile(
        &self,
        variant: &ModelPackageVariant,
        profile: &CompileProfile,
    ) -> Result<CompiledModelHandle, InferenceError>;

    fn submit(
        &self,
        model: &CompiledModelHandle,
        bindings: &[TensorBinding<'_>],
        deadline: Deadline,
    ) -> Result<SubmissionHandle, InferenceError>;

    fn poll(&self, submission: &mut SubmissionHandle) -> PollResult;
    fn cancel(&self, submission: SubmissionHandle);
}
```

The handle types own or reference adapter-private state through safe Rust wrappers. No provider object escapes as a raw pointer.

## 30.3 Fast 2D task module

```rust
pub trait FastWholeBody2d: Send {
    fn prepare(
        &mut self,
        package: &ModelPackage,
        runners: &dyn ModelRunnerFactory,
        context: &TaskPrepareContext,
    ) -> Result<(), TaskError>;

    fn submit(
        &mut self,
        frame: &CameraFrame,
        region: &PersonRegion,
        deadline: Deadline,
        sink: &dyn Sink<FastWholeBody2dObservation>,
    ) -> Result<SubmissionToken, TaskError>;

    fn reset(&mut self, scope: ResetScope);
}
```

## 30.4 Fusion interface

```rust
pub struct FusionWindow<'a> {
    pub target_time: MonotonicTime,
    pub views: &'a [CameraObservationBundle],
    pub primary_spatial: Option<&'a SpatialBodyObservation>,
    pub auxiliary_spatial: &'a [SpatialBodyObservation],
    pub site: &'a SiteStateSnapshot,
}

pub trait BodyFusion: Send {
    fn fuse(
        &mut self,
        window: FusionWindow<'_>,
        context: &FusionContext,
    ) -> Result<FusedBodyObservation, FusionError>;
}
```

Current implementations:

```text
GeometryOnlyFusion
GeometryPlusSetResidualFusion
OfflineOptimizationFusion
RecordedReferenceFusion
```

## 30.5 Temporal interface

```rust
pub trait TemporalPerformanceEstimator: Send {
    fn prepare(&mut self, profile: &TemporalProfile) -> Result<(), TemporalError>;

    fn update(
        &mut self,
        body: &FusedBodyObservation,
        faces: &[FaceObservation],
        hands: &[HandObservation],
        props: &[PropObservation],
        output_time: MonotonicTime,
    ) -> Result<TemporalPerformanceState, TemporalError>;

    fn predict(
        &self,
        output_time: MonotonicTime,
    ) -> Result<CanonicalPerformanceFrame, TemporalError>;

    fn reset(&mut self, scope: ResetScope);
}
```

## 30.6 Retarget interface

```rust
pub trait AvatarRetargeter: Send {
    fn bind(&mut self, profile: &AvatarProfile) -> Result<(), RetargetError>;

    fn solve(
        &mut self,
        performance: &CanonicalPerformanceFrame,
        contacts: &ContactConstraintSet,
        output_time: MonotonicTime,
    ) -> Result<AvatarPoseFrame, RetargetError>;

    fn reset(&mut self, scope: ResetScope);
}
```

## 30.7 Output adapter

```rust
pub trait OutputAdapter: Send {
    fn descriptor(&self) -> OutputDescriptor;
    fn configure(&mut self, config: &OutputConfig) -> Result<(), OutputError>;
    fn start(&mut self, avatar: &AvatarProfile) -> Result<(), OutputError>;
    fn submit_latest(&mut self, pose: &AvatarPoseFrame);
    fn stop(&mut self);
    fn health(&self) -> OutputHealthSnapshot;
}
```

The output queue is bounded and latest-only unless the protocol explicitly requires another semantic.

## 30.8 Repository port

```rust
pub trait SiteRepository: Send + Sync {
    fn get(&self, id: SiteId) -> Result<Option<SiteState>, RepositoryError>;
    fn list(&self, page: PageRequest) -> Result<Page<SiteSummary>, RepositoryError>;
    fn put(
        &self,
        site: &SiteState,
        expected_revision: Option<Revision>,
    ) -> Result<Revision, RepositoryError>;
    fn delete(&self, id: SiteId, expected_revision: Revision)
        -> Result<(), RepositoryError>;
}
```

Application services depend on this trait. The SQLite adapter owns SQL and transaction details.

## 30.9 Application service

```rust
pub struct StartSessionService {
    sites: Arc<dyn SiteRepository>,
    avatars: Arc<dyn AvatarRepository>,
    models: Arc<dyn ModelRepository>,
    engine: Arc<dyn EngineControl>,
    operations: Arc<dyn OperationRepository>,
}

impl StartSessionService {
    pub fn execute(
        &self,
        command: StartSessionCommand,
    ) -> Result<OperationId, ApplicationError> {
        // Validate resource revisions, create a long-running operation,
        // and ask the engine boundary to build and warm the graph.
        todo!()
    }
}
```

The HTTP handler maps a generated request DTO into `StartSessionCommand` and invokes this service. It does not construct the graph itself.

## 30.10 Engine command boundary

```rust
pub trait EngineControl: Send + Sync {
    fn start_session(
        &self,
        request: EngineStartRequest,
    ) -> Result<EngineOperation, EngineError>;

    fn stop_session(&self, session: SessionId) -> Result<(), EngineError>;
    fn reset_performer(&self, session: SessionId) -> Result<(), EngineError>;
    fn apply_live_update(
        &self,
        session: SessionId,
        update: LiveSessionUpdate,
    ) -> Result<(), EngineError>;
    fn snapshot(&self, session: SessionId) -> Result<EngineSnapshot, EngineError>;
}
```

The concrete implementation owns bounded command channels into the engine thread. A caller never receives a mutable graph or node pointer.

## 30.11 Model package manifest subset

```json
{
  "schema_version": "1.0",
  "package_id": "body.spatial.student-medium",
  "package_version": "0.3.0",
  "task_contract": "avatartrack.spatial-body/v1",
  "graphs": [
    {
      "id": "main-fp16",
      "path": "graphs/main.onnx",
      "sha256": "...",
      "inputs": ["image", "person_prompt", "temporal_cache"],
      "outputs": ["pose", "shape", "camera", "uncertainty", "next_cache"],
      "shape_profiles": ["720p-person"],
      "precision": "fp16"
    }
  ],
  "provider_requirements": {
    "required_ops": [],
    "allowed_cpu_fallback_fraction": 0.02
  },
  "calibration": {
    "confidence_temperature": "calibration/confidence.json"
  },
  "test_vectors": "test-vectors/index.json",
  "model_card": "MODEL_CARD.md"
}
```

## 30.12 Plugin process handshake

```protobuf
message PluginHello {
  string plugin_id = 1;
  string plugin_version = 2;
  repeated string supported_contracts = 3;
  repeated string requested_permissions = 4;
  uint32 protocol_major = 5;
  uint32 protocol_minor = 6;
}

message HostHello {
  string host_version = 1;
  repeated string accepted_contracts = 2;
  repeated string granted_permissions = 3;
  uint64 max_message_bytes = 4;
}
```

The plugin protocol sends semantic observations or output frames. It does not expose Rust trait objects, allocator-owned containers, GPU pointers, or daemon internals.


# 31. Architecture-decision record policy

Significant architecture changes require an ADR under `docs/architecture/adr/`.

## 31.1 ADR triggers

Write an ADR only when the decision changes a durable boundary, ownership rule, compatibility promise, trust boundary, or measured hot-path architecture. Ordinary implementation choices remain in code and review.

- changing public API transport or version policy;
- allowing frontend direct backend bindings;
- changing process topology;
- adding a runtime language;
- changing canonical skeleton or coordinate conventions;
- adding arbitrary executable code to model packages;
- changing plugin isolation;
- changing persistent storage technology or migration policy;
- introducing shared latent features across model families;
- removing a deterministic fallback;
- changing privacy defaults;
- changing output ownership;
- adding a new GPU/backend abstraction;
- adding a compatibility layer with a release lifetime;
- introducing an abstraction that crosses more than one owned package;
- keeping two production paths instead of replacing one.

## 31.2 ADR template

```markdown
# ADR-XXXX: Title

Status: Proposed | Accepted | Superseded | Rejected
Date:
Owners:

## Context
What problem and constraints exist?

## Decision
What exact choice is made?

## Alternatives considered
Include the strongest plausible alternatives.

## Consequences
Positive, negative, migration, latency, security, and maintenance effects.

## Contract impact
Public API, semantic packets, model packages, plugins, recordings.

## Validation plan
Tests, replay corpus, benchmarks, rollback.

## Current reader and removal condition
Name the current producer, consumer, or external boundary. State when the decision can be deleted.

## Cost
State implementation, migration, retraining, regeneration, repackaging, recalibration, and maintenance cost.

## References
Papers and official specifications.
```

### RA-133 — Record decisions that change boundaries

ADRs shall focus on decisions that alter ownership, dependency direction, contracts, process boundaries, state lifetime, trust, or compatibility. They shall name the current reader, simplest rejected alternative, validation path, cost, and removal condition. Do not use an ADR to excuse ordinary complexity. [A01] [A18]

---

# 32. Reference ledger

## 32.1 Software architecture, APIs, and runtime systems

- [A01] David L. Parnas. **On the Criteria To Be Used in Decomposing Systems into Modules.** Communications of the ACM, 1972. [https://dl.acm.org/doi/10.1145/361598.361623](https://dl.acm.org/doi/10.1145/361598.361623)
- [A02] Matt Welsh, David Culler, Eric Brewer. **SEDA: An Architecture for Well-Conditioned, Scalable Internet Services.** SOSP 2001. [https://dl.acm.org/doi/10.1145/502034.502057](https://dl.acm.org/doi/10.1145/502034.502057)
- [A03] Jeffrey Dean, Luiz André Barroso. **The Tail at Scale.** Communications of the ACM, 2013. [https://research.google/pubs/the-tail-at-scale/](https://research.google/pubs/the-tail-at-scale/)
- [A04] J. H. Saltzer, D. P. Reed, D. D. Clark. **End-to-End Arguments in System Design.** ACM TOCS, 1984. [https://www.cs.princeton.edu/courses/archive/spring19/cos463/papers/endtoend.pdf](https://www.cs.princeton.edu/courses/archive/spring19/cos463/papers/endtoend.pdf)
- [A05] Jim Waldo, Geoff Wyant, Ann Wollrath, Sam Kendall. **A Note on Distributed Computing.** Sun Microsystems Laboratories, 1994. [https://waldo.scholars.harvard.edu/publications/note-distributed-computing](https://waldo.scholars.harvard.edu/publications/note-distributed-computing)
- [A06] Martin Thompson et al. **Disruptor: High Performance Alternative to Bounded Queues for Exchanging Data Between Concurrent Threads.** 2011. [https://lmax-exchange.github.io/disruptor/files/Disruptor-1.0.pdf](https://lmax-exchange.github.io/disruptor/files/Disruptor-1.0.pdf)
- [A07] Google. **Protocol Buffers Language Guide and Message Evolution.** [https://protobuf.dev/programming-guides/proto3/](https://protobuf.dev/programming-guides/proto3/)
- [A08] OpenAPI Initiative. **OpenAPI Specification 3.1.1.** [https://spec.openapis.org/oas/v3.1.1.html](https://spec.openapis.org/oas/v3.1.1.html)
- [A09] IETF. **RFC 6455: The WebSocket Protocol.** [https://datatracker.ietf.org/doc/html/rfc6455](https://datatracker.ietf.org/doc/html/rfc6455)
- [A10] OpenTelemetry. **Signals, Traces, and Context Propagation.** [https://opentelemetry.io/docs/concepts/signals/](https://opentelemetry.io/docs/concepts/signals/)
- [A11] Google. **AIP-121: Resource-Oriented Design.** [https://google.aip.dev/121](https://google.aip.dev/121)
- [A12] Google. **AIP-151: Long-Running Operations.** [https://google.aip.dev/151](https://google.aip.dev/151)
- [A13] Google. **AIP-155: Request Identification and Idempotency.** [https://google.aip.dev/155](https://google.aip.dev/155)
- [A14] gRPC. **Core Concepts, Architecture, and Lifecycle.** [https://grpc.io/docs/what-is-grpc/core-concepts/](https://grpc.io/docs/what-is-grpc/core-concepts/)
- [A15] Tauri. **Tauri 2 Documentation and Supported Desktop Platforms.** [https://v2.tauri.app/](https://v2.tauri.app/)
- [A16] ONNX Runtime. **Execution Providers and Runtime Architecture.** [https://onnxruntime.ai/docs/execution-providers/](https://onnxruntime.ai/docs/execution-providers/)
- [A17] VRM Consortium. **VRM 1.0 Humanoid Avatar Specification.** [https://vrm.dev/en/vrm1/](https://vrm.dev/en/vrm1/)
- [A18] D. Sculley et al. **Hidden Technical Debt in Machine Learning Systems.** NeurIPS 2015. [https://papers.nips.cc/paper/5656-hidden-technical-debt-in-machine-learning-systems](https://papers.nips.cc/paper/5656-hidden-technical-debt-in-machine-learning-systems)
- [A19] Eric Breck et al. **The ML Test Score: A Rubric for ML Production Readiness and Technical Debt Reduction.** IEEE Big Data 2017. [https://research.google.com/pubs/pub45742.html](https://research.google.com/pubs/pub45742.html)
- [A20] Margaret Mitchell et al. **Model Cards for Model Reporting.** FAT* 2019. [https://arxiv.org/abs/1810.03993](https://arxiv.org/abs/1810.03993)
- [A21] Timnit Gebru et al. **Datasheets for Datasets.** Communications of the ACM, 2021. [https://arxiv.org/abs/1803.09010](https://arxiv.org/abs/1803.09010)
- [A22] The Rust Project. **Cargo Workspaces.** [https://doc.rust-lang.org/cargo/reference/workspaces.html](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [A23] The Rust Project. **Cargo Features and Build Profiles.** [https://doc.rust-lang.org/cargo/reference/features.html](https://doc.rust-lang.org/cargo/reference/features.html) and [https://doc.rust-lang.org/cargo/reference/profiles.html](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [A26] Tokio Project. **Tokio Runtime, Channels, and Graceful Shutdown.** [https://tokio.rs/tokio/tutorial](https://tokio.rs/tokio/tutorial)
- [A27] Crossbeam Project. **Bounded Multi-Producer Multi-Consumer Channels.** [https://docs.rs/crossbeam-channel/](https://docs.rs/crossbeam-channel/)
- [A28] The Rust Project. **Rustfmt and `cargo fmt`.** [https://github.com/rust-lang/rustfmt](https://github.com/rust-lang/rustfmt)
- [A29] The Rust Project. **Clippy Usage and Continuous Integration.** [https://doc.rust-lang.org/clippy/usage.html](https://doc.rust-lang.org/clippy/usage.html)
- [A30] Bazel `rules_rust`. **Rust Rules and Crate Universe.** [https://bazelbuild.github.io/rules_rust/](https://bazelbuild.github.io/rules_rust/) and [https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html](https://bazelbuild.github.io/rules_rust/crate_universe_bzlmod.html)
- [A31] `ort` Project and ONNX Runtime. **Rust Interface for ONNX Runtime and Session Execution.** [https://docs.rs/ort/](https://docs.rs/ort/) and [https://onnxruntime.ai/](https://onnxruntime.ai/)
- [A32] `prost` Project. **Protocol Buffers for Rust.** [https://docs.rs/prost/](https://docs.rs/prost/)
- [A33] `wgpu` Project. **Cross-Platform Safe Rust GPU API.** [https://wgpu.rs/doc/wgpu/](https://wgpu.rs/doc/wgpu/)
- [A24] Virtual Motion Capture. **VMC Protocol.** [https://protocol.vmc.info/english.html](https://protocol.vmc.info/english.html)
- [A25] VRChat. **OSC Trackers.** [https://docs.vrchat.com/docs/osc-trackers](https://docs.vrchat.com/docs/osc-trackers)

## 32.2 Tracking, fusion, temporal, retargeting, data, and deployment papers

The keys below match the companion technical architecture where possible.

- [P01] **OnlineHMR: Video-based Online World-Grounded Human Mesh Recovery.** arXiv:2603.17355; CVPR 2026. [https://arxiv.org/abs/2603.17355](https://arxiv.org/abs/2603.17355)
- [P09] **DiffusionPoser: Real-time Human Motion Reconstruction From Arbitrary Sparse Sensors Using Autoregressive Diffusion.** CVPR 2024. [https://arxiv.org/abs/2308.16682](https://arxiv.org/abs/2308.16682)
- [P12] **Self-Avatar Animation in Virtual Reality: Impact of Motion Signals Artifacts on the Full-Body Pose Reconstruction.** 2024. [https://arxiv.org/abs/2404.18628](https://arxiv.org/abs/2404.18628)
- [P13] **Animation Fidelity in Self-Avatars: Impact on User Performance and Sense of Agency.** 2023. [https://arxiv.org/abs/2304.05334](https://arxiv.org/abs/2304.05334)
- [P15] **Kineo: Calibration-Free Metric Motion Capture From Sparse RGB Cameras.** 2025 preprint. [https://arxiv.org/abs/2510.24464](https://arxiv.org/abs/2510.24464)
- [P16] **Spatiotemporal Multi-Camera Calibration using Freely Moving People.** 2025. [https://arxiv.org/abs/2502.12546](https://arxiv.org/abs/2502.12546)
- [P17] **Human Pose as Calibration Pattern: 3D Human Pose Estimation with Multiple Unsynchronized and Uncalibrated Cameras.** CVPR Workshops 2018. [https://openaccess.thecvf.com/content_cvpr_2018_workshops/w34/html/Takahashi_Human_Pose_As_CVPR_2018_paper.html](https://openaccess.thecvf.com/content_cvpr_2018_workshops/w34/html/Takahashi_Human_Pose_As_CVPR_2018_paper.html)
- [P19] **MUC: Mixture of Uncalibrated Cameras for Robust 3D Human Body Reconstruction.** 2024. [https://arxiv.org/abs/2403.05055](https://arxiv.org/abs/2403.05055)
- [P20] **Human Mesh Recovery from Arbitrary Multi-view Images.** 2024. [https://arxiv.org/abs/2403.12434](https://arxiv.org/abs/2403.12434)
- [P21] **Set Transformer: A Framework for Attention-based Permutation-Invariant Neural Networks.** ICML 2019. [https://arxiv.org/abs/1810.00825](https://arxiv.org/abs/1810.00825)
- [P22] **Probabilistic Triangulation for Uncalibrated Multi-View 3D Human Pose Estimation.** ICCV 2023. [https://arxiv.org/abs/2309.04756](https://arxiv.org/abs/2309.04756)
- [P23] **UPose3D: Uncertainty-Aware 3D Human Pose Estimation with Cross-View and Temporal Cues.** ECCV 2024. [https://arxiv.org/abs/2404.14634](https://arxiv.org/abs/2404.14634)
- [P24] **Learnable Triangulation of Human Pose.** ICCV 2019. [https://arxiv.org/abs/1905.05754](https://arxiv.org/abs/1905.05754)
- [P36] **Skeleton-Aware Networks for Deep Motion Retargeting.** SIGGRAPH 2020. [https://arxiv.org/abs/2005.05732](https://arxiv.org/abs/2005.05732)
- [P37] **HybrIK: A Hybrid Analytical-Neural Inverse Kinematics Solution for 3D Human Pose and Shape Estimation.** CVPR 2021. [https://arxiv.org/abs/2011.14672](https://arxiv.org/abs/2011.14672)
- [P39] **Skinned Motion Retargeting with Residual Perception of Motion Semantics & Geometry.** 2023. [https://arxiv.org/abs/2303.08658](https://arxiv.org/abs/2303.08658)
- [P40] **Expressive Body Capture: 3D Hands, Face, and Body from a Single Image.** CVPR 2019. [https://arxiv.org/abs/1904.05866](https://arxiv.org/abs/1904.05866)
- [P41] **On the Continuity of Rotation Representations in Neural Networks.** CVPR 2019. [https://arxiv.org/abs/1812.07035](https://arxiv.org/abs/1812.07035)
- [P42] **Synchronize Dual Hands for Physics-Based Dexterous Guitar Playing.** SIGGRAPH Asia 2024. [https://arxiv.org/abs/2409.16629](https://arxiv.org/abs/2409.16629)
- [P43] **Audio Matters Too! Enhancing Markerless Motion Capture with Audio Signals for String Performance Capture.** 2024. [https://arxiv.org/abs/2405.04963](https://arxiv.org/abs/2405.04963)
- [P45] **BEDLAM: A Synthetic Dataset of Bodies Exhibiting Detailed Lifelike Animated Motion.** CVPR 2023. [https://arxiv.org/abs/2306.16940](https://arxiv.org/abs/2306.16940)
- [P47] **Syn4D: A Multiview Synthetic 4D Dataset.** 2026 preprint. [https://arxiv.org/abs/2605.05207](https://arxiv.org/abs/2605.05207)
- [P51] **Distilling the Knowledge in a Neural Network.** 2015. [https://arxiv.org/abs/1503.02531](https://arxiv.org/abs/1503.02531)
- [P52] **Domain Randomization for Transferring Deep Neural Networks from Simulation to the Real World.** IROS 2017. [https://arxiv.org/abs/1703.06907](https://arxiv.org/abs/1703.06907)
- [P53] **Quantization and Training of Neural Networks for Efficient Integer-Arithmetic-Only Inference.** CVPR 2018. [https://arxiv.org/abs/1712.05877](https://arxiv.org/abs/1712.05877)
- [P55] **Once-for-All: Train One Network and Specialize it for Efficient Deployment.** ICLR 2020. [https://arxiv.org/abs/1908.09791](https://arxiv.org/abs/1908.09791)
- [P56] **TVM: An Automated End-to-End Optimizing Compiler for Deep Learning.** OSDI 2018. [https://arxiv.org/abs/1802.04799](https://arxiv.org/abs/1802.04799)
- [P58] **ANIRA: An Architecture for Neural Network Inference in Real-Time Audio Applications.** 2025 preprint. [https://arxiv.org/abs/2506.12665](https://arxiv.org/abs/2506.12665)
- [P59] **Deep Learning Inference Frameworks Benchmark.** 2022. [https://arxiv.org/abs/2210.04323](https://arxiv.org/abs/2210.04323)
- [P60] **An Empirical Study of Challenges in Converting Deep Learning Models.** 2022. [https://arxiv.org/abs/2206.14322](https://arxiv.org/abs/2206.14322)

---

# Final recommended architecture statement

Build the project as a **headless, API-first, modular Rust tracking daemon** with a replaceable web frontend, explicit process boundaries, a staged realtime packet graph, strict state ownership, canonical semantic observation contracts, self-describing model packages, and an offline ML/data toolchain that communicates with runtime only through schemas and exported packages.

The first-party implementation language is Rust. Cargo owns the Rust build graph. Tokio owns control-plane async I/O, while the realtime graph uses dedicated workers and bounded queues. ONNX Runtime and operating-system APIs remain adapter-contained foreign dependencies. No frontend package imports backend crates, and no plugin depends on a Rust or C++ dynamic-library ABI.

The most important practical rule is:

```text
Frontend knows resources, operations, events, previews, and final poses.
Backend application knows use cases and lifecycle.
Realtime engine knows packets, clocks, queues, and deadlines.
Tracking modules know semantic observations.
Task modules know model-specific preprocessing/postprocessing.
Inference backend knows tensors and hardware providers.
No layer reaches through the layer below it to take shortcuts.
```

That split permits the project to begin with simple geometry, filters, and existing models, then replace current pieces with stronger custom models without rebuilding the UI, output integrations, calibration workflow, storage, or entire runtime. It also permits other developers to contribute camera backends, output protocols, model packages, avatar importers, instrument observers, frontends, and training datasets without needing to understand or fork every part of the system. The repository does not pursue modularity by accumulating wrappers, fake implementations, dormant plugin points, compatibility shims, or tests of internal structure. Every maintained seam, field, file, test, and dependency must have a current reader.
