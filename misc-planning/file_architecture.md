# Senrigan — Implementation and Delivery Plan

**Product behavior:** [`overall_ideal.md`](overall_ideal.md).
**Engine contracts:** [`engine_concept.md`](engine_concept.md).
**Prior art:** [`prior_art.md`](prior_art.md).
**Purpose:** define the stack, repository boundaries, owned numbers, implementation order, tests, and release gates.

This file owns every numeric gate until tests exist. Other documents link here. Do not copy the numbers into other planning files.

The plan assumes one developer working for fun. Short measured loops come before optional research.

---

## 1. Initial stack

| Layer | Initial choice | Boundary |
|---|---|---|
| Runtime | Rust 2024, Cargo workspace | Capture, scheduling, geometry, estimator, retargeting, VMC, recording |
| Inference | ONNX Runtime through `ort` | Concrete sessions first. Provider and device come from the model manifest and configuration |
| Model, export, and evaluation | Python managed by `uv` | Never required by ordinary end users |
| Setup UI | Tauri and TypeScript | Typed in-process commands. No pose round-trip through JSON |
| Windows capture | Media Foundation | Record actual mode, timestamp behavior, and camera controls |
| Record and replay | MCAP | Timestamped schemas for frames, observations, state, status, residuals, and timing |
| Avatar | VRM 0.x and 1.0 import | Engine-owned canonical skeleton and explicit version mapping |
| Live output | VMC over OSC and UDP | Final retargeted pose only |
| Task entry point | `just` | Wrap Cargo, `uv`, `pnpm`, tests, export, and benchmarks |

Windows is first. Linux and V4L2 follow only after the Windows live slice works on both reference GPU classes.

### 1.1 Reference hardware and providers

| Reference | Phase 0 provider candidates | First useful release rule |
|---|---|---|
| Intel Arc B580 desktop | ONNX Runtime OpenVINO or Intel-compatible EP, and DirectML EP | At least one validated provider and precision configuration |
| NVIDIA RTX 4060 desktop class | ONNX Runtime CUDA EP and DirectML EP. TensorRT is optional later | At least one validated provider and precision configuration |

One GPU is enough to develop. Phase 0 may close on one card with a written “other class pending” note. Both reference classes must pass before the first useful release.

Senrigan is the primary GPU workload during the release benchmark. Background desktop activity is allowed. A concurrently running game is not part of the gate.

Provider-specific model files or precision are allowed when the manifest records them and final motion stays within the declared quality tolerance. There is no silent “Arc uses a smaller or worse model” policy.

### 1.2 Dependency policy

- Pin Rust, JavaScript, Python, ONNX Runtime, model, and schema versions used for a release.
- Commit lockfiles and model manifests. Do not commit multi-gigabyte model weights to Git.
- A release bundle may place verified weights beside the application when their terms permit redistribution.
- Every model file is hash-checked before session creation.
- If a provider or operator combination is unsupported, fail during startup validation. Do not fail after tracking begins.
- Do not add a generic plugin system, daemon boundary, or inference trait until a real second implementation or a deterministic test double requires it.

## 2. Repository layout

Start with five Rust crates. Do not create one crate per conceptual box.

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

Dependency direction is `types` ← (`engine`, `perception`, `avatar`) ← `app`. `avatar` may use shared math and contracts from `types`. It does not call capture or UI. The application composes concrete implementations in one process.

Split capture, outputs, geometry, grounding, or temporal logic into new crates only after dependencies or build times demonstrate a useful boundary. Module boundaries inside a crate are enough before then.

## 3. Application and process boundary

The Tauri application embeds the engine. Typed commands cover camera selection, model and provider selection, avatar import, calibration, session control, recording, and status. Bounded events or shared snapshots carry UI-rate preview and status data.

Rust owns the output-clock pose stream. The web view receives a coalesced snapshot. The pose stream never travels from Rust to TypeScript and back to an output adapter. A later `wgpu` preview may render in Rust. It must not pass through TypeScript.

The same composition can expose a headless binary for replay, benchmarking, and VMC output. It is not a daemon required by the desktop UI.

The first output adapters are:

1. In-process preview.
2. MCAP final-pose recording.
3. VMC sender.

Named VMC reference receivers are VSeeFace and Warudo. VNyan is an optional third receiver when a test machine has it.

REST, WebSocket, plugins, VRChat-specific virtual trackers, and remote control wait until a concrete consumer exists.

## 4. Engineering rules

1. **Record and replay precede quality work.** If a change cannot run against a stable replay, it cannot support a quality claim.
2. **One owner per state.** The estimator corrects root and contact state once. Later stages consume it.
3. **Latest useful evidence wins.** Bounded queues drop stale unprocessed camera frames. They do not accumulate latency.
4. **Output never waits for inference.** The output clock predicts or holds and always publishes status.
5. **Failure is typed.** No observation, stale input, deadline miss, invalid output, and provider failure stay distinguishable.
6. **No silent provider fallback.** Use CPU or another EP only when the user selects a configuration validated for it.
7. **Hard validity before visual objectives.** Invalid contacts or solver state cannot escape as a partial pose.
8. **Profile both reference machines before release.** A result on the training desktop or only one GPU class is incomplete for the first useful release.
9. **Measure end to end.** Include capture delivery, copies, preprocessing, synchronization, inference, estimator, retarget, and VMC publication.
10. **Build the feature before the abstraction.** Keep Phase 1 concrete and small.

## 5. Model package layout

Each model configuration is a directory that can be copied into a bundle or installed atomically.

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
- Input color, range, normalization, and resize or crop rules.
- Output tensor names, shapes, axes, joint topology, and units.
- Allowed dynamic dimensions.
- Provider, device, and precision compatibility tested by the project.
- Expected warm-up behavior.
- Canonical adapter identifier.
- Whether the package is `BUNDLE_APPROVED`, `LOCAL_RESEARCH_ONLY`, or `UNREVIEWED`.

Example `manifest.json` shape. Field names may gain prefixes. Do not drop required meaning.

```json
{
  "model_id": "sam3d-body-fast-example",
  "version": "0.0.0-unreviewed",
  "file_hashes": { "model.onnx": "sha256:0" },
  "source_and_license_reference": "see NOTICE.txt",
  "preprocessing_and_tensor_schema": "see adapter notes",
  "canonical_adapter_version": "source-v0",
  "supported_providers_and_precision": ["CUDA:fp16", "DirectML:fp16"],
  "minimum_memory_observed": "unmeasured",
  "numerical_tolerances": "unmeasured",
  "release_class": "UNREVIEWED"
}
```

Installation writes to a temporary package directory, verifies every hash, then renames it into place. Never select an incomplete or corrupted package. The first useful release may require a full application reinstall for model rollback. A network update service is not required.

## 6. Owned numbers

This section is the single owner for planning numbers. Other documents link here.

### 6.1 Output and inference

| Name | Value |
|---|---|
| Output clock | 60 Hz |
| Desired active spatial rate | 15 Hz |
| Conditional minimum spatial rate | 12 Hz |
| Capture-to-VMC median latency hypothesis | 100 ms |
| Capture-to-VMC p95 latency hypothesis | 150 ms |
| Output continuity | 99.9 percent of scheduled ticks over a 30-minute replay |

A configuration between the desired and conditional spatial rates may pass only if capture-to-VMC latency, motion-onset behavior, and qualitative replay remain acceptable. Below the conditional minimum, re-plan the model or configuration. Do not assume prediction will hide the deficit.

### 6.2 Setup

| Name | Value |
|---|---|
| First setup | 5 minutes for a valid VRM and supported camera |
| Repeat setup | 1 minute when the camera has not moved |
| Provisional tracking | 5 seconds |

### 6.3 Lifecycle

| Name | Value |
|---|---|
| `PREDICTING` horizon | 300 ms |
| `HOLDING` start | 300 ms |
| `LOST` start | 1000 ms |
| Reacquisition confirm | 5 consecutive reliable frames |
| Reacquisition blend | 250 ms |
| Root correction cap during blend | 2 percent of performer height per output tick |
| Joint correction cap during blend | 10 degrees per output tick |
| Session grace window | 30 s |

These lifecycle values are conservative starting points to tune on replay. They are not claims of perceptual optimality.

### 6.4 Phase 1 checkpoint

| Name | Value |
|---|---|
| Capture replay duration | 10 minutes |
| Normal-proportion VRM count | 3 |
| Named VMC reference receivers | VSeeFace and Warudo |

### 6.5 Phase 2 stability

| Name | Value |
|---|---|
| Stillness joint-angle improvement | at least 50 percent over raw spatial output |
| Planted-foot speed and distance improvement | at least 50 percent over raw spatial output on hand-labeled contacts |
| False candidate that must not replace the live track | 5 frames or fewer |

Set absolute jitter and skate thresholds from the first labeled baseline before tuning Phase 2. Commit the raw measurements and the chosen thresholds together.

### 6.6 Phase 3 retarget

| Name | Value |
|---|---|
| Mean reachable contact-region error | below 1 percent of avatar height on the clean benchmark |
| Extreme-morphology contact and penetration improvement | at least 50 percent over the rotation and IK baseline |
| Solver runtime | below 4 ms p95 on the reference CPUs |

### 6.7 Other test lengths

| Name | Value |
|---|---|
| Endurance replay | 30 minutes with recording and VMC enabled |

## 7. Phase map

Use integer phase numbers only. `overall_ideal.md` must not keep a second outline.

| Phase | Name | In first useful release | Depends on |
|---|---|---|---|
| 0 | Spatial feasibility | Yes, as the selected model package. One GPU may prove it. Both GPUs are required to call the release done | Nothing |
| 1 | Live baseline | Yes, as an internal checkpoint and as part of the release | Phase 0 for live inference. Types, capture, and MCAP may start in parallel |
| 2 | Grounding and containment | Yes | Phase 1 |
| 3 | Geometry-aware retarget | No | Phase 1. Benefits from Phase 2 |
| 4 | Learned temporal experiment | No | Phase 2. May close as a negative result |
| 5 | Face and hand refinement | No | Phase 1. Benefits from Phase 2 |
| 6 | Optional multi-GPU placement | No | Phase 5 |
| 7 | One-family instrument experiment | No | Phase 2 and Phase 3 |
| 8 | Optional second camera | No | Phase 2 |

The live baseline is an internal checkpoint. It is not a public release. The first useful release is Phase 0 plus Phase 1 plus Phase 2 on both reference GPU classes.

### 7.1 Calendar hypothesis

This table is a hypothesis. Replace it after Phase 0 measurements. Assume one developer and one primary GPU. The second GPU is a release check.

| Phase | Order-of-magnitude duration | Why |
|---|---|---|
| 0 | 1 to 3 months | Export and provider risk dominate |
| 1 | 2 to 4 months | Capture, replay, adapter, VRM, VMC, setup UI |
| 2 | 2 to 4 months | Calibration harness and analytical estimator |
| 3 | 1 to 3 months after the release | After a clean retarget benchmark exists |
| 4 | 2 to 6 months, or a written drop | Data rights and training dominate |
| 5 and later | After Phase 2 is stable | Do not schedule in detail now |

The first useful release is then about 5 to 11 months. Phase 0 export risk and Phase 2 estimator work dominate. This is not a public ship date.

## 8. Build and experiment order

### Phase 0 — Spatial feasibility

Do not build the full application around a model that cannot run on a reference GPU or satisfy its selected release mode.

| Step | Work | Exit condition |
|---|---|---|
| 0.1 | **Candidate inventory.** Record code, checkpoint, body-model, data and provenance notes, input and output representation, and package terms. Include SAM 3D Body and Fast SAM 3D Body as named candidates. | No candidate receives a release label without the exact supporting terms. |
| 0.2 | **Minimal export.** Export the smallest credible candidate before optimizing it. | A fixed recorded image produces finite outputs through ONNX Runtime with tensor names and shapes documented. |
| 0.3 | **Numerical parity.** Compare source-framework and ORT output through the canonical adapter. | Tensor and final joint or rotation differences stay within predeclared tolerances. Failures are localized. |
| 0.4 | **Provider comparison.** Run the exported configuration on the GPUs you have, across candidate EPs and precision. | One provider configuration passes on at least one reference GPU. A written note lists the other class as pending if it is not measured yet. Reports include p50, p95, p99, fresh rate, VRAM, warm-up, and failures. |
| 0.5 | **Canonical adapter swap probe.** Adapt a second candidate or deterministic fixture. | Downstream canonical replay is unchanged in schema. No source mesh or body-model type leaks across the boundary. |
| 0.6 | **Feasibility decision.** Select the Phase 1 model package or declare the precise failed gate. | Written selection names quality, runtime, provider, packaging, and restriction tradeoffs. Both GPU classes are documented before the first useful release. |

Apply weight-preserving optimizations in this order:

1. Clean the graph and freeze static shapes.
2. Use provider-supported precision and fusion.
3. Move preprocessing to the GPU and remove synchronization points.
4. Apply crop, resolution, or decoder reductions that the model family already validates.

Feature reuse and student training are not Phase 0 shortcuts.

### Phase 1 — Live baseline

Types, capture, and MCAP may start before Phase 0 closes. Live inference needs Phase 0 on at least one GPU.

| Step | Work | Exit condition |
|---|---|---|
| 1.1 | **Workspace and contracts.** Implement coordinate-safe transforms, timestamps, packet schemas, lifecycle and status, model manifests, and serialization versions. | Unit tests cover transforms, time ordering, schema round trips, invalid numeric rejection, and version errors. |
| 1.2 | **Windows capture and MCAP first.** Enumerate camera modes, capture bounded frames, classify timestamps, and record and replay without inference. | A session of the capture-replay duration replays frame order, mode, and timing. Queue drops and recording failure are visible. |
| 1.3 | **Concrete ORT runtime.** Load the selected manifest, verify files, warm the selected provider and device, preprocess, and infer. | Live and replay inference work on the development GPU with no silent provider change. |
| 1.4 | **Canonical adapter and basic continuity.** Convert spatial output to the canonical skeleton and add bounded per-joint continuity and hold behavior. | Recorded motion contains finite rotations, positions, and status at the output clock and survives dropped observations. |
| 1.5 | **VRM import and rotation and IK baseline.** Validate a VRM and apply normalized rotation and root transfer with terminal IK. | The normal-proportion VRM count animates without axis reversals, exploding scales, or missing required-bone crashes. |
| 1.6 | **Preview and VMC.** Display the final retargeted pose and send the same pose through VMC. | Named reference receivers agree with preview for root, required bones, cadence, status, and selected VRM version. |
| 1.7 | **Setup application and package.** Camera, model, and avatar selection, diagnostics, session control, recording consent, and error flows. | A clean Windows machine can complete setup and run without Python or developer tools. |

Phase 1 does not require grounding, a trained temporal model, extreme-avatar optimization, face refinement, or multi-GPU scheduling. It is not a public release.

### Phase 2 — Grounding and containment

Phase 2 is required for the first useful release.

| Step | Work | Exit condition |
|---|---|---|
| 2.1 | **Instrumented calibration harness.** Capture board and measured references for development. Capture board-free human-pose inputs for the product solve. | Floor, camera, and scale error can be measured independently rather than only by self-consistency. |
| 2.2 | **Site calibration and performer fit.** Estimate uncertain intrinsics, up and floor, site pose, capture region, and locked proportions. | Cross-performer and repeat-session tests pass. Moved-camera state becomes stale rather than silently adapting. |
| 2.3 | **Coverage contract.** Preserve unclamped ROI, edge distances, visibility evidence, root support, and estimate origin. | Every limb and frame-edge fixture prevents unsupported scale or root updates. |
| 2.4 | **Analytical estimator.** Add authoritative pose, root, velocity, and contact state, bounded observation correction, constraints, and output-clock prediction. | It improves declared stability metrics over raw spatial output without increasing invalid or fallback events. |
| 2.5 | **Loss lifecycle and reacquisition.** Implement partial, predicting, holding, lost, and isolated candidate states. | Full-loss durations, false candidates, re-entry, and handoff produce deterministic bounded results. |

### Phase 3 — Geometry-aware retargeting

| Step | Work | Exit condition |
|---|---|---|
| 3.1 | **Retarget profile import.** Build sparse proxy, semantic regions, contact sites, reach envelopes, bend preferences, and labeled limit sources. | Canonical import probes reject or report broken rigs and proxies. |
| 3.2 | **Independent benchmark.** Replay clean canonical motion across normal and extreme morphology fixtures. | Rotation and IK baseline errors are recorded separately from webcam errors. |
| 3.3 | **Bounded sparse solver.** Add reachable semantic targets, warm-started fixed iterations, feasibility projection, residuals, and fallback. | It improves extreme-avatar contact and penetration gates while staying within the CPU deadline. |
| 3.4 | **General contact contract.** Validate floor, self, and known-transform object surfaces. | Synthetic object fixtures prove the contract without requiring instrument detection. |

### Phase 4 — Learned temporal experiment

| Step | Work | Exit condition |
|---|---|---|
| 4.1 | **Target and provenance inventory.** Identify lawful motion and contact targets, project recordings, reference captures, and upstream error pairs. | Every sample records origin, labels, transformations, terms, and allowed use. |
| 4.2 | **Corruption model.** Fit noise, delay, drops, flips, crop loss, occlusion, and uncertainty features to measured spatial failures. | Synthetic validation distributions are compared with real replays rather than chosen by intuition. |
| 4.3 | **Causal model.** Train residual pose, root, contact, and uncertainty prediction over irregular observations. | Held-out metrics improve over both raw spatial and analytical baselines with stable long rollout. |
| 4.4 | **Runtime gate.** Export, verify provider parity, and retain the analytical fallback. | Both GPU classes meet overall latency and memory gates. Failure behavior remains deterministic. |

If the learned model does not win, the analytical estimator ships and Phase 4 closes as a useful negative result.

#### Phase 4 data-rights checklist

Close Phase 4 as a written drop if any of the following hold:

- No lawful motion or contact target set can be named with origin, terms, and allowed use.
- Project recordings lack consent or a documented intended use.
- Upstream spatial outputs cannot be paired with reference targets without violating checkpoint terms.
- Export or provider parity fails on a reference GPU class.

Synthetic corruption supplements paired real failures. It does not replace them. Do not start training before 4.1 passes.

### Phase 5 — Face and hand refinement

SAM 3D Body does not supply facial expressions. Phase 5 treats face as a real modality. MediaPipe Face Landmarker is the first baseline. Native-resolution hand crops are added only if the whole-body hands fail the measured quality bar.

Each modality has its own timestamp, provider session, queue, deadline, uncertainty, and fusion rule. Measure wrist and neck reconciliation explicitly. MediaPipe coefficients map through the imported avatar expression profile. If custom expressions are missing, degrade to supported VRM presets. Do not send nonexistent names.

### Phase 6 — Optional multi-GPU mode

After multiple independent modalities exist:

1. Enumerate compatible devices and providers.
2. Let configuration assign each complete model session to a device ID.
3. Keep per-device bounded queues and timing traces.
4. Align all outputs by source timestamp in the existing estimator.
5. Compare quality, throughput, memory, transfer, and p95 stalls with the one-GPU baseline.

The first mode may place body on one GPU and face or hands on another. It does not split one graph across devices. It does not promise linear scaling. It does not automatically mix Intel and NVIDIA providers. If transfers or synchronization erase the benefit, drop the mode.

### Phase 7 — Instrument experiment

Choose one family. Prefer the instrument the developer can record and evaluate most easily. Add a known `InstrumentProfile`, object pose, contact evidence, and optional audio onset while reusing `ContactIntent`. Expand the abstraction to other families only after that end-to-end prototype works.

### Phase 8 — Optional multi-camera

Add timestamped observations from another camera without changing canonical estimator or output contracts. Human-based board-free product calibration remains the intended UX. Instrumented development references remain allowed. Multi-view fusion, synchronization, and calibration are new research work. They are not assumed consequences of adding another capture thread.

## 9. Queue and failure policy

| Boundary | Capacity policy | On overflow or failure |
|---|---|---|
| Capture to preprocessing | Small newest-frame queue | Drop oldest unprocessed frame. Count and trace |
| Preprocessing to inference | At most one queued request per session plus active work | Replace queued request with a newer compatible frame |
| Inference to estimator | Ordered by source time with a bounded late window | Reject stale result. Never rewind live state |
| Estimator to output | Latest authoritative state snapshot | Predict to tick. Never wait |
| Output to VMC | Non-blocking bounded datagrams | Count send failures. Tracking continues |
| Any stage to MCAP | Bounded diagnostic queue | Stop recording visibly before affecting output |
| Runtime to UI | Coalesced status and preview snapshots | Drop intermediate UI updates |

Additional required behavior:

- **Camera disconnect:** enter the prediction, hold, and lost lifecycle. Retain site state. Mark the camera unavailable.
- **Camera mode change:** invalidate calibration and crop or inference caches tied to the old mode.
- **Provider loss or device reset:** report failure, stop accepting spatial observations, and hold safely. Do not migrate silently.
- **Corrupt model package:** fail verification before session creation and identify the exact file.
- **Warm-up or recompile:** remain in setup state. Do not include warm-up in steady-state FPS. Report warm-up separately.
- **Disk full or permission error:** close the recording cleanly when possible and keep tracking.
- **Invalid VRM:** report missing bones, axes, or version before starting output.
- **Shutdown:** stop new capture, drain or cancel inference, publish a final unavailable status where possible, finalize MCAP, then release devices.

## 10. Test strategy

### 10.1 Fast automated tests

- Coordinate-frame composition, inverse, handedness, and quaternion normalization.
- Timestamp ordering, age, and late-result rejection.
- Schema and manifest validation and version rejection.
- Visibility-evidence and estimate-origin combinations.
- State reset boundaries and performer handoff.
- Lifecycle transitions with a fake monotonic clock.
- Finite-output and hard-limit projection.
- Model hash and atomic-package validation.
- VMC encoding, bone names, expression aliases, cadence, and malformed receiver configuration.
- Deterministic rotation and IK fallback.

### 10.2 Replay suites

Maintain small named suites rather than one unstructured library:

- **Stillness:** holds in several poses.
- **Ordinary VTubing:** seated and standing speech, gestures, pointing, reaching, and head turns.
- **Ground and contact:** step, walk in place, weight shift, sit and stand, crouch.
- **Ambiguity:** 90 and 180 degree turns, foreshortened reaches, self-occlusion, object occlusion.
- **Coverage:** each limb and torso crossing every frame edge, permanent desk crop, lateral exit.
- **Loss:** blank intervals, false candidates, displaced re-entry, new performer.
- **Camera:** exposure step, resolution or mode change, physical movement, disconnect and reconnect.
- **Quality:** 480p and 720p, low light, blur, dropped frames, cluttered backgrounds.
- **Avatar:** normal, chibi, long-arm, short-arm and long-torso, broad, tall and thin, large hands and feet, optional bones.
- **Endurance:** the endurance replay length with recording and VMC enabled.

Raw recordings in the repository must be small, consented, and documented. Larger or private suites can be referenced by stable local dataset identifiers. Do not commit sensitive footage.

### 10.3 Hardware and provider suite

Run the same immutable replay and manifest on both reference machines before the first useful release. Record:

- OS build, driver, GPU, CPU, RAM, camera mode.
- Provider and precision.
- Warm-up duration.
- Stage and end-to-end p50, p95, and p99.
- Fresh observation rate and age distribution.
- VRAM peak and allocation failures.
- Output and VMC missed ticks.
- Canonical and final-motion differences from the reference configuration.

Ordinary CI runs unit, schema, deterministic replay, and formatting checks without requiring a GPU. Hardware reports are manual or self-hosted until dedicated runners exist. Keep machine-readable results under `benchmarks/`.

## 11. Quantitative gates

Thresholds are provisional engineering gates, not public claims. Change one only through an explicit documented decision made before evaluating the change it will judge. Values live in §6.

### Phase 1 live gate — internal checkpoint

- Meet the output-clock, continuity, and latency hypotheses in §6 on the development GPU.
- Meet the spatial-rate rule in §6.1.
- No NaN, infinite transform, invalid quaternion, partial pose, silent provider change, or unreported fallback.
- No unbounded queue growth or increasing latency trend.
- No device runs out of memory. Each report includes peak device memory.
- Preview and the named VMC reference receivers agree on root and required humanoid bone motion.
- Clean setup on Windows without Python or development tools.

### Phase 2 stability gate — first useful release

- Meet every applicable Phase 1 live gate on both reference GPU classes.
- Meet every applicable product performance target linked from [`overall_ideal.md`](overall_ideal.md).
- Fixed source bone lengths after performer lock.
- Stillness and planted-foot improvements meet §6.5 without exceeding the latency hypothesis.
- Reacquisition obeys the correction caps in §6.3.
- Unsupported root displacement stops accumulating by the `LOST` transition.
- Predicted joints never create or promote hard contacts.
- False candidates at or below the §6.5 length do not replace the live track.
- Lifecycle and reacquisition decisions reproduce exactly on deterministic replay.
- Floor, camera, and scale validation reports independent reference error, not only self-consistency.

### Phase 3 retarget gate

- No final hard-limit violation, invalid pose, partial result, or silent fallback.
- Contact-region error and solver runtime meet §6.6.
- Contact error and penetration duration improve by the §6.6 extreme-morphology rule.
- No material regression on normal-proportion avatars.
- Report iteration and fallback rates.

### Phase 4 learned-model gate

- Improvement over both raw spatial and analytical baselines on held-out performers and motions.
- No regression beyond declared tolerance in latency, p95 stalls, long rollout, loss containment, or uncertainty calibration.
- Successful export and provider parity on both GPU classes.
- Analytical fallback remains available and tested.

### Public comparison gate

Side-by-side clips name the versions, hardware, camera, avatar, motion, and tuning. Include failure clips. A tracker or multi-camera “parity” claim requires a separately designed blinded comparison. Label informal developer preference as such.

## 12. Definition of first useful release

The first useful release is complete when:

- Phase 0 selects and documents one working configuration for Arc B580 and one for RTX 4060-class hardware.
- Phase 1 live baseline works on Windows without Python.
- Phase 2 site calibration, locked proportions, coverage contract, analytical estimator, and loss lifecycle are active and labeled.
- Phase 2 stability gates pass on named replays on both reference GPU classes.
- A Windows user can import a supported VRM, select a camera, model, and provider, complete board-free setup, preview final motion, and send it through VMC.
- The application can record and replay capture, inference, canonical output, fallback, and final avatar output.
- Output remains finite and status-labeled through ordinary dropped frames and short performer loss.
- The installation contains verified model files and notices.

Extreme-avatar solving, learned temporal state, face refinement, instruments, multi-GPU, Linux, and multi-camera are not blockers for this milestone.
