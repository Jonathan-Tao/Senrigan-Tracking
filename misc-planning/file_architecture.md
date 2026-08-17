# Senrigan — Implementation and Delivery Plan

**Product behavior:** [`overall_ideal.md`](overall_ideal.md).
**Engine contracts and algorithms:** [`engine_concept.md`](engine_concept.md).
**Purpose of this document:** define the stack, repository boundaries, implementation order, hardware matrix, tests, and release gates.

The plan assumes one developer working for fun. It prioritizes short measurable loops and a usable live path before optional research.

---

## 1. Initial stack

| Layer | Initial choice | Boundary |
|---|---|---|
| Runtime | Rust 2024, Cargo workspace | Capture, scheduling, geometry, estimator, retargeting, VMC, recording |
| Inference | ONNX Runtime through `ort` | Concrete sessions first. Provider/device selected from model manifest and configuration |
| Model/export/evaluation | Python managed by `uv` | Never required by ordinary end users |
| Setup UI | Tauri + TypeScript | Typed in-process commands. No pose round-trip through JSON |
| Windows capture | Media Foundation | Record actual mode, timestamp behavior, and camera controls |
| Record/replay | MCAP | Timestamped schemas for frames, observations, state, status, residuals, and timing |
| Avatar | VRM 0.x/1.0 import | Engine-owned canonical skeleton and explicit version mapping |
| Live output | VMC/OSC over UDP | Final retargeted pose only |
| Task entry point | `just` | Wrap Cargo, `uv`, `pnpm`, tests, export, and benchmarks |

Windows is first. Linux/V4L2 follows only after the Windows live slice works on both reference GPU classes.

### 1.1 Reference hardware and providers

| Reference | Phase 0 provider candidates | First-release rule |
|---|---|---|
| Intel Arc B580 desktop | ONNX Runtime OpenVINO/Intel-compatible EP and DirectML EP | At least one validated provider/precision configuration |
| NVIDIA RTX 4060 desktop class | ONNX Runtime CUDA EP and DirectML EP, with TensorRT optional later | At least one validated provider/precision configuration |

Senrigan is the primary GPU workload during the release benchmark. Background desktop activity is allowed. A concurrently running game is not part of the gate.

Both reference classes must pass. Provider-specific model files or precision are allowed when the manifest records them and final motion remains within the declared quality tolerance. There is no silent “Arc uses a smaller/worse model” policy.

### 1.2 Dependency policy

- Pin Rust, JavaScript, Python, ONNX Runtime, model, and schema versions used for a release.
- Commit lockfiles and model manifests. Do not commit multi-gigabyte model weights to Git.
- A release bundle may place verified weights beside the application when their terms permit redistribution.
- Every model file is hash-checked before session creation.
- Unsupported provider/operator combinations fail during startup validation, not after tracking begins.
- Do not add a generic plugin system, daemon boundary, or inference trait until a real second implementation or deterministic test double requires it.

## 2. Repository layout

Start with five Rust crates rather than one crate per conceptual box:

```text
senrigan/
├── crates/
│   ├── types/       timestamps, frames, schemas, coordinate-safe math, status
│   ├── engine/      calibration, analytical estimator, contacts, lifecycle
│   ├── perception/  preprocessing, ORT sessions, model adapters, coverage
│   ├── avatar/      VRM import, retargeting, expression mapping, VMC encoding
│   └── app/         capture, scheduler, MCAP, Tauri commands, binaries
├── ml/              export, parity, training, evaluation, data tools
├── ui/              Tauri web frontend
├── models/          manifests, notices, and ignored weight files
├── testdata/        small consented replays and synthetic fixtures
├── benchmarks/      expected configurations and machine-readable reports
├── justfile
└── Cargo.toml
```

Dependency direction is `types` ← (`engine`, `perception`, `avatar`) ← `app`. `avatar` may use shared math/contracts from `types` but does not call capture or UI. The application composes concrete implementations in one process.

Split capture, outputs, geometry, grounding, or temporal logic into new crates only after dependencies or build times demonstrate a useful boundary. Module boundaries inside a crate are sufficient before then.

## 3. Application and process boundary

The Tauri application embeds the engine. Typed commands cover camera selection, model/provider selection, avatar import, calibration, session control, recording, and status. Bounded events or shared snapshots carry UI-rate preview/status data. The 60 Hz pose stream never travels from Rust to TypeScript and back to an output adapter.

The same composition can expose a headless binary for replay, benchmarking, and VMC output. It is not a daemon required by the desktop UI.

The first output adapters are:

1. In-process preview.
2. MCAP final-pose recording.
3. VMC sender.

REST, WebSocket, plugins, VRChat-specific virtual trackers, and remote control are deferred until a concrete consumer exists.

## 4. Engineering rules

1. **Record/replay precedes quality work.** A change that cannot run against a stable replay cannot support a quality claim.
2. **One owner per state.** Root/contact state is corrected once in the estimator. Later stages consume it.
3. **Latest useful evidence wins.** Bounded queues drop stale unprocessed camera frames instead of accumulating latency.
4. **Output never waits for inference.** The 60 Hz loop predicts or holds and always publishes status.
5. **Failure is typed.** No observation, stale input, deadline miss, invalid output, and provider failure remain distinguishable.
6. **No silent provider fallback.** CPU or another EP is used only when the user selects a configuration validated for it.
7. **Hard validity before visual objectives.** Invalid contacts or solver state cannot escape as a partial pose.
8. **Profile both reference machines.** A result on the training desktop or only one GPU class is incomplete.
9. **Measure end to end.** Include capture delivery, copies, preprocessing, synchronization, inference, estimator, retarget, and VMC publication.
10. **Build the feature before the abstraction.** Keep Phase 1 concrete and small.

## 5. Model package layout

Each model configuration is a directory that can be copied into a bundle or installed atomically:

```text
models/<package-id>/
├── manifest.json
├── NOTICE.txt
├── LICENSE.txt
├── model.onnx
├── optional external tensor data
└── optional provider-specific graph files
```

The manifest contains the `ModelManifest` fields in `engine_concept.md` plus:

- Expected file sizes and hashes.
- Upstream source and immutable revision.
- Input color/range/normalization and resize/crop rules.
- Output tensor names, shapes, axes, joint topology, and units.
- Allowed dynamic dimensions.
- Provider/device/precision compatibility tested by the project.
- Expected warm-up behavior.
- Canonical adapter identifier.
- Whether the package is `BUNDLE_APPROVED`, `LOCAL_RESEARCH_ONLY`, or `UNREVIEWED`.

Installation writes to a temporary package directory, verifies every hash, then renames it into place. An incomplete or corrupted package is never selected. The initial hobby release may require a full application reinstall for model rollback. A network update service is not required.

## 6. Build and experiment order

### Phase 0 — Feasibility gate

Do not build the full application around a model that cannot run on both reference GPU classes or satisfy its selected release mode.

| Step | Work | Exit condition |
|---|---|---|
| 0.1 | **Candidate inventory.** Record code, checkpoint, body-model, data/provenance notes, input/output representation, and package terms. | No candidate receives a release label without the exact supporting terms. |
| 0.2 | **Minimal export.** Export the smallest credible candidate before optimizing it. | A fixed recorded image produces finite outputs through ONNX Runtime with tensor names/shapes documented. |
| 0.3 | **Numerical parity.** Compare source-framework and ORT output through the canonical adapter. | Tensor and final-joint/rotation differences stay within predeclared tolerances. Failures are localized. |
| 0.4 | **Dual-GPU provider bake-off.** Run the same exported configuration on Arc B580 and RTX 4060 across candidate EPs/precision. | One provider configuration passes per GPU. Reports include p50/p95/p99, fresh rate, VRAM, warm-up, and failures. |
| 0.5 | **Canonical adapter swap probe.** Adapt a second candidate or deterministic fixture. | Downstream canonical replay is unchanged in schema and no source mesh/body-model type leaks across the boundary. |
| 0.6 | **Feasibility decision.** Select the Phase 1 model/package or declare the precise failed gate. | Written selection names quality, runtime, provider, packaging, and restriction tradeoffs on both GPUs. |

Desired active spatial rate is at least 15 Hz. A configuration between 12 and 15 Hz may pass only if capture-to-VMC latency, motion-onset behavior, and qualitative replay remain acceptable. Below 12 Hz triggers a model/configuration re-plan rather than assuming prediction will hide the deficit.

Apply weight-preserving optimizations in this order:

1. Clean the graph and freeze static shapes.
2. Use provider-supported precision and fusion.
3. Move preprocessing to the GPU and remove synchronization points.
4. Apply crop, resolution, or decoder reductions that the model family already validates.

Feature reuse and student training are not Phase 0 shortcuts.

### Phase 1 — Useful live tracker

| Step | Work | Exit condition |
|---|---|---|
| 1.1 | **Workspace and contracts.** Implement coordinate-safe transforms, timestamps, packet schemas, lifecycle/status, model manifests, and serialization versions. | Unit tests cover transforms, time ordering, schema round trips, invalid numeric rejection, and version errors. |
| 1.2 | **Windows capture and MCAP first.** Enumerate camera modes, capture bounded frames, classify timestamps, and record/replay without inference. | A ten-minute session replays frame order/mode/timing. Queue drops and recording failure are visible. |
| 1.3 | **Concrete ORT runtime.** Load the selected manifest, verify files, warm the selected provider/device, preprocess, and infer. | Live and replay inference work on both GPU classes with no silent provider change. |
| 1.4 | **Canonical adapter and basic continuity.** Convert spatial output to the canonical skeleton and add bounded per-joint continuity/hold behavior. | Recorded motion contains finite rotations/positions/status at 60 Hz and survives dropped observations. |
| 1.5 | **VRM import and baseline retarget.** Validate a VRM and apply normalized rotation/root transfer with terminal IK. | Three normal-proportion VRMs animate without axis reversals, exploding scales, or missing required-bone crashes. |
| 1.6 | **Preview and VMC.** Display the final retargeted pose and send the same pose through VMC. | Reference receivers agree with preview for root, required bones, cadence, status, and selected VRM version. |
| 1.7 | **Setup application and package.** Camera/model/avatar selection, diagnostics, session control, recording consent, and error flows. | A clean Windows machine can complete setup and run without Python or developer tools. |

Phase 1 intentionally does not require perfect grounding, a trained temporal model, extreme-avatar optimization, face refinement, or multi-GPU scheduling.

### Phase 1.1 — Grounding and failure containment

| Step | Work | Exit condition |
|---|---|---|
| 1.1.1 | **Instrumented calibration harness.** Capture board/measured references for development and board-free human-pose inputs for the product solve. | Floor/camera/scale error can be measured independently rather than only by self-consistency. |
| 1.1.2 | **Site calibration and performer fit.** Estimate uncertain intrinsics, up/floor, site pose, capture region, and locked proportions. | Cross-performer and repeat-session tests pass. Moved-camera state becomes stale rather than silently adapting. |
| 1.1.3 | **Coverage contract.** Preserve unclamped ROI, edge distances, visibility evidence, root support, and estimate origin. | Every limb/frame-edge fixture prevents unsupported scale/root updates. |
| 1.1.4 | **Analytical estimator.** Add authoritative pose/root/velocity/contact state, bounded observation correction, constraints, and 60 Hz prediction. | It improves declared stability metrics over raw spatial output without increasing invalid/fallback events. |
| 1.1.5 | **Loss lifecycle and reacquisition.** Implement partial, predicting, holding, lost, and isolated candidate states. | Full-loss durations, false candidates, re-entry, and handoff produce deterministic bounded results. |

### Phase 1.2 — Geometry-aware retargeting

| Step | Work | Exit condition |
|---|---|---|
| 1.2.1 | **Retarget profile import.** Build sparse proxy, semantic regions, contact sites, reach envelopes, bend preferences, and labelled limit sources. | Canonical import probes reject or report broken rigs and proxies. |
| 1.2.2 | **Independent benchmark.** Replay clean canonical motion across normal and extreme morphology fixtures. | Rotation/IK baseline errors are recorded separately from webcam errors. |
| 1.2.3 | **Bounded sparse solver.** Add reachable semantic targets, warm-started fixed iterations, feasibility projection, residuals, and fallback. | It improves extreme-avatar contact/penetration gates while staying within the CPU deadline. |
| 1.2.4 | **General contact contract.** Validate floor, self, and known-transform object surfaces. | Synthetic object fixtures prove the contract without requiring instrument detection. |

### Phase 1.3 — Learned temporal experiment

| Step | Work | Exit condition |
|---|---|---|
| 1.3.1 | **Target and provenance inventory.** Identify lawful motion/contact targets, project recordings, reference captures, and upstream error pairs. | Every sample records origin, labels, transformations, terms, and allowed use. |
| 1.3.2 | **Corruption model.** Fit noise, delay, drops, flips, crop loss, occlusion, and uncertainty features to measured spatial failures. | Synthetic validation distributions are compared with real replays rather than chosen by intuition. |
| 1.3.3 | **Causal model.** Train residual pose/root/contact/uncertainty prediction over irregular observations. | Held-out metrics improve over both raw spatial and analytical baselines with stable long rollout. |
| 1.3.4 | **Runtime gate.** Export, verify provider parity, and retain the analytical fallback. | Both GPU classes meet overall latency/memory gates. Failure behavior remains deterministic. |

If the learned model does not win, the analytical estimator ships and Phase 1.3 closes as a useful negative result.

### Phase 2 — Face and hand refinement

SAM 3D Body does not supply facial expressions. Phase 2 therefore treats face as a real modality, not an optional polish pass. MediaPipe Face Landmarker is the first baseline. Native-resolution hand crops are added only if the whole-body hands fail the measured quality bar.

Each modality has its own timestamp, provider session, queue, deadline, uncertainty, and fusion rule. Wrist and neck reconciliation are measured explicitly. MediaPipe’s coefficients map through the imported avatar expression profile. Missing custom expressions degrade to supported VRM presets rather than being sent under nonexistent names.

### Phase 2.x — Optional multi-GPU mode

After multiple independent modalities exist:

1. Enumerate compatible devices and providers.
2. Let configuration assign each complete model session to a device ID.
3. Keep per-device bounded queues and timing traces.
4. Align all outputs by source timestamp in the existing estimator.
5. Compare quality, throughput, memory, transfer, and p95 stalls with the one-GPU baseline.

The first mode may place body on one GPU and face/hands on another. It does not split one graph across devices, promise linear scaling, or automatically mix Intel and NVIDIA providers. If transfers or synchronization erase the benefit, the mode is dropped.

### Phase 3 — Instrument experiment

Choose one family—preferably the instrument the developer can record and evaluate most easily. Add a known `InstrumentProfile`, object pose, contact evidence, and optional audio onset while reusing `ContactIntent`. Only after that end-to-end prototype works should the abstraction expand to fretted strings, keyboards, bowed instruments, and struck surfaces.

### Phase 4 — Optional multi-camera

Add timestamped observations from another camera without changing canonical estimator/output contracts. Human-based board-free product calibration remains the intended UX, while instrumented development references remain allowed. Multi-view fusion, synchronization, and calibration are new research work, not assumed consequences of adding another capture thread.

## 7. Queue and failure policy

| Boundary | Capacity policy | On overflow/failure |
|---|---|---|
| Capture → preprocessing | Small newest-frame queue | Drop oldest unprocessed frame. Count and trace |
| Preprocessing → inference | At most one queued request per session plus active work | Replace queued request with newer compatible frame |
| Inference → estimator | Ordered by source time with bounded late window | Reject stale result. Never rewind live state |
| Estimator → output | Latest authoritative state snapshot | Predict to tick. Never wait |
| Output → VMC | Non-blocking bounded datagrams | Count send failures. Tracking continues |
| Any stage → MCAP | Bounded diagnostic queue | Stop recording visibly before affecting output |
| Runtime → UI | Coalesced status/preview snapshots | Drop intermediate UI updates |

Additional required behavior:

- **Camera disconnect:** enter prediction/hold/lost lifecycle. Retain site state but mark camera unavailable.
- **Camera mode change:** invalidate calibration and crop/inference caches tied to the old mode.
- **Provider loss/device reset:** report failure, stop accepting spatial observations, and hold safely. Do not migrate silently.
- **Corrupt model package:** fail verification before session creation and identify the exact file.
- **Warm-up/recompile:** remain in setup state. Do not include warm-up in steady-state FPS while still reporting it separately.
- **Disk full/permission error:** close the recording cleanly when possible and keep tracking.
- **Invalid VRM:** report missing bones/axes/version before starting output.
- **Shutdown:** stop new capture, drain or cancel inference, publish a final bad/unavailable status where possible, finalize MCAP, then release devices.

## 8. Test strategy

### 8.1 Fast automated tests

- Coordinate-frame composition, inverse, handedness, and quaternion normalization.
- Timestamp ordering, age, and late-result rejection.
- Schema and manifest validation/version rejection.
- Visibility-evidence and estimate-origin combinations.
- State reset boundaries and performer handoff.
- Lifecycle transitions with a fake monotonic clock.
- Finite-output and hard-limit projection.
- Model hash and atomic-package validation.
- VMC encoding, bone names, expression aliases, cadence, and malformed receiver configuration.
- Deterministic rotation/IK fallback.

### 8.2 Replay suites

Maintain small named suites rather than one unstructured library:

- **Stillness:** 60-second holds in several poses.
- **Ordinary VTubing:** seated/standing speech, gestures, pointing, reaching, and head turns.
- **Ground/contact:** step, walk in place, weight shift, sit/stand, crouch.
- **Ambiguity:** 90/180-degree turns, foreshortened reaches, self-occlusion, object occlusion.
- **Coverage:** each limb and torso crossing every frame edge, permanent desk crop, lateral exit.
- **Loss:** blank intervals from 100 ms to 30 seconds, false candidates, displaced re-entry, new performer.
- **Camera:** exposure step, resolution/mode change, physical movement, disconnect/reconnect.
- **Quality:** 480p/720p, low light, blur, dropped frames, cluttered backgrounds.
- **Avatar:** normal, chibi, long-arm, short-arm/long-torso, broad, tall/thin, large hands/feet, optional bones.
- **Endurance:** at least 30 minutes with recording and VMC enabled.

Raw recordings in the repository must be small, consented, and documented. Larger/private suites can be referenced by stable local dataset identifiers without committing sensitive footage.

### 8.3 Hardware/provider suite

Run the same immutable replay and manifest on both reference machines. Record:

- OS build, driver, GPU, CPU, RAM, camera mode.
- Provider and precision.
- Warm-up duration.
- Stage and end-to-end p50/p95/p99.
- Fresh observation rate and age distribution.
- VRAM peak and allocation failures.
- Output/VMC missed ticks.
- Canonical and final-motion differences from the reference configuration.

Ordinary CI runs unit, schema, deterministic replay, and formatting checks without requiring a GPU. Hardware reports are manual or self-hosted until dedicated runners exist. Their machine-readable results are kept under `benchmarks/`.

## 9. Quantitative gates

Thresholds are provisional engineering gates, not public claims. Change one only through an explicit documented decision made before evaluating the change it will judge.

### Phase 1 live gate — both reference GPUs

- Meet every applicable product performance target in [`overall_ideal.md` §5](overall_ideal.md#5-experience-and-performance-targets) on both reference GPUs.
- Active spatial inference ≥15 Hz desired and ≥12 Hz hard conditional minimum.
- No NaN, infinite transform, invalid quaternion, partial pose, silent provider change, or unreported fallback.
- No unbounded queue growth or increasing latency trend.
- No device runs out of memory, and each report includes peak device memory.
- Preview and two reference VMC receivers agree on root and required humanoid bone motion.
- Clean setup on Windows without Python or development tools.

### Phase 1.1 stability gate

- Fixed source bone lengths after performer lock.
- Stillness joint-angle standard deviation improves at least 50% over raw spatial output without exceeding the latency gate.
- Planted-foot speed and distance improve at least 50% over raw spatial output on hand-labelled contacts.
- Reacquisition obeys the correction caps in [`engine_concept.md` §8.5](engine_concept.md#85-loss-and-reacquisition).
- Unsupported root displacement stops accumulating by the `LOST` transition.
- Predicted joints never create/promote hard contacts.
- False candidates of five frames or fewer do not replace the live track.
- Lifecycle and reacquisition decisions reproduce exactly on deterministic replay.
- Floor/camera/scale validation reports independent reference error, not only self-consistency.

Absolute jitter and skate thresholds are set from the first labelled baseline before tuning Phase 1.1. The raw measurements and chosen thresholds are committed together so the target cannot move after results are known.

### Phase 1.2 retarget gate

- No final hard-limit violation, invalid pose, partial result, or silent fallback.
- Mean reachable contact-region error below 1% of avatar height on the clean benchmark.
- Contact error and penetration duration improve at least 50% over rotation/IK on the extreme-morphology set.
- No material regression on normal-proportion avatars.
- Solver runtime below 4 ms p95 on the reference CPUs, with iteration and fallback rates reported.

### Phase 1.3 learned-model gate

- Improvement over both raw spatial and analytical baselines on held-out performers/motions.
- No regression beyond declared tolerance in latency, p95 stalls, long rollout, loss containment, or uncertainty calibration.
- Successful export and provider parity on both GPU classes.
- Analytical fallback remains available and tested.

### Public comparison gate

Side-by-side clips name the versions, hardware, camera, avatar, motion, and tuning. Failure clips are included. A tracker/multi-camera “parity” claim requires a separately designed blinded comparison. Informal developer preference is labelled as such.

## 10. Definition of first useful release

The first useful release is complete when:

- Phase 0 selects and documents one working configuration for Arc B580 and one for RTX 4060-class hardware.
- A Windows user can import a supported VRM, select a camera, model, and provider, preview final motion, and send it through VMC.
- The application can record and replay capture, inference, canonical output, fallback, and final avatar output.
- Output remains finite and status-labelled through ordinary dropped frames and short subject loss.
- The installation contains verified model files and notices and does not require Python.
- Published performance and failure reports contain measurements rather than promises.

Advanced grounding, extreme-avatar solving, learned temporal state, face refinement, instruments, multi-GPU, Linux, and multi-camera are explicitly not blockers for this milestone.
