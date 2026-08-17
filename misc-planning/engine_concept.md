# Senrigan — Engine Architecture

**Scope:** technical behavior of the one-camera tracking engine.
**Product requirements:** [`overall_ideal.md`](overall_ideal.md).
**Implementation order, owned numbers, and release gates:** [`file_architecture.md`](file_architecture.md).
**Prior art:** [`prior_art.md`](prior_art.md).

This document defines how timestamped webcam frames become finite, status-labeled, avatar-ready motion. It does not select a final spatial model before the Phase 0 comparison in `file_architecture.md`.

---

## 1. Architectural invariants

1. The output loop is causal. It may predict forward. It does not wait for future frames during live tracking.
2. Observation rate and output rate are independent. The final avatar stream uses the output clock in `file_architecture.md` even when spatial inference is slower.
3. Exactly one estimator owns source root, pose, velocity, contact, and uncertainty state.
4. Site, performer, avatar, inference-cache, estimator, and retarget state have separate types and reset rules.
5. Visibility evidence and estimate origin are separate concepts.
6. No isolated limb observation may rescale the performer or translate an unsupported root.
7. A predicted joint cannot create or promote a hard contact.
8. Every output tick is finite or replaced by an explicit last-valid fallback. Partial solver output is never published.
9. Provider-specific execution may change numerical details. It must not change the canonical motion contract.
10. Product calibration is board-free. Instrumented development validation is permitted and required.
11. Losing the performer is an ordinary lifecycle state. It is not an exception. It is not permission to stop output.
12. The first useful release uses one GPU. Later multi-GPU placement cannot change estimator semantics.

## 2. End-to-end data flow

```text
Camera
  │ FramePacket
  ▼
Capture and preprocessing ───────────────► MCAP recording tap
  │
  ├── cheap ROI and coverage path, if retained
  │
  ▼
Spatial inference on one selected GPU
  │ SpatialObservation (camera-relative, timestamped)
  ▼
Body-model adapter
  │ canonical joint observations only
  ▼
Authoritative causal estimator
  │ predict → observe → ground and contact correction → safety projection
  │ CanonicalMotionFrame + TrackingStatus
  ▼
Stateful avatar retargeter
  │ RetargetResult
  ├────────► preview
  ├────────► VMC sender
  └────────► MCAP and evaluation output
```

The estimator update for each accepted observation is:

1. Predict the current source state to the observation timestamp.
2. Validate the observation, crop, coverage, age, and camera or site compatibility.
3. Form pose, reprojection, floor, contact, limb-length, continuity, and root-support factors.
4. Apply one bounded correction to the authoritative state.
5. Project the result onto finite-output and hard kinematic constraints.
6. Save residuals and updated uncertainty.
7. Predict the corrected state to each requested output timestamp.

Grounding is part of this correction. It is not a second downstream owner of root state.

## 3. Coordinate frames, time, and units

All distances are meters. Every transform name states destination then source.

- `C` — camera optical frame. OpenCV convention. `+X` right, `+Y` down, `+Z` forward from the optical center.
- `W` — calibrated site frame. Right-handed. `+Y` up. Nominal floor near `Y=0`. `+Z` is the site forward used at calibration.
- `S` — canonical source skeleton root and rest frame. `+Y` up after the adapter.
- `A` — imported avatar rest frame. Follows the VRM asset. glTF `+Y` up.

`T_W_C` maps a point expressed in `C` into `W`. Internal quaternions use `xyzw` component order, Hamilton product, and unit length before publication. Handedness and forward-axis conversion occur only at named adapters. VMC and glTF adapters convert from this convention.

Every packet uses a monotonic runtime clock. Wall-clock time may be recorded as metadata. It never drives fusion. A frame carries:

- Capture or source timestamp when the backend exposes a trustworthy value.
- Application receipt timestamp.
- Sequence number.
- Timestamp-quality classification.
- Camera mode and nominal frame interval.

If a backend timestamp is only a delivery timestamp, the packet says so. Latency reports must not rename it “exposure time.” VMC publication latency is measured from the best available source timestamp and always reports the timestamp-quality class.

## 4. Core contracts

The syntax below is illustrative. Exact Rust layout may change without weakening the fields or invariants.

### 4.1 Capture and observation

```text
FramePacket
  camera_id
  sequence
  source_time
  receipt_time
  timestamp_quality
  pixel_format, width, height, stride
  nominal_frame_interval
  image_buffer
  camera_controls_snapshot

VisibilityEvidence = OBSERVED | OCCLUDED | TRUNCATED | UNKNOWN
EstimateOrigin     = FRESH | PREDICTED | HELD | FALLBACK

JointObservation
  joint_id
  position_C
  optional local_rotation
  position_covariance
  rotation_uncertainty
  visibility_evidence
  signed_frame_edge_distance

SpatialObservation
  source_frame_sequence
  source_time, completed_time
  model_id, model_version, adapter_version
  crop_transform
  unclamped_performer_roi, clamped_input_roi
  visible_roi_fraction
  torso_root_support
  joints[]
  optional shape/proportion proposal
  stage_outcome
```

`TRUNCATED` describes what the camera or crop could not contain. `PREDICTED` describes where an output came from. A truncated joint can therefore produce a predicted estimate without collapsing two facts into one enum.

The unclamped theoretical performer ROI is retained even when preprocessing clamps pixels to the sensor. This prevents a partial performer from being normalized into an apparently complete smaller performer.

### 4.2 Canonical source motion

```text
CanonicalMotionFrame
  schema_version
  source_time, state_time, output_time
  T_W_S
  local_joint_rotations[]
  global_joint_positions_W[]
  rest_offsets_S[]
  linear_and_angular_velocities[]
  joint_uncertainty[]
  visibility_evidence[]
  estimate_origin[]
  contact_proposals[]
  global_root_support
  performer_scale_status
```

The canonical topology is VRM-humanoid-compatible. It is not identical to a particular imported avatar or body model.

Required joints:

| Joint | Parent |
|---|---|
| `hips` | root |
| `spine` | `hips` |
| `chest` | `spine` |
| `neck` | `chest` or `upperChest` if present |
| `head` | `neck` |
| `leftUpperArm` | `leftShoulder` or `chest` |
| `leftLowerArm` | `leftUpperArm` |
| `leftHand` | `leftLowerArm` |
| `rightUpperArm` | `rightShoulder` or `chest` |
| `rightLowerArm` | `rightUpperArm` |
| `rightHand` | `rightLowerArm` |
| `leftUpperLeg` | `hips` |
| `leftLowerLeg` | `leftUpperLeg` |
| `leftFoot` | `leftLowerLeg` |
| `rightUpperLeg` | `hips` |
| `rightLowerLeg` | `rightUpperLeg` |
| `rightFoot` | `rightLowerLeg` |

Optional joints. Omit them when the spatial model or avatar does not supply them:

- `upperChest`
- `leftShoulder`, `rightShoulder`
- `leftToes`, `rightToes`
- `leftEye`, `rightEye`, `jaw`
- VRM finger bones, using VRM 1 names when both versions exist

Missing-joint policy:

- If a required joint is absent from the spatial model, the adapter synthesizes it from neighbors. `EstimateOrigin` must not be `FRESH`.
- If an optional joint is absent, omit it from the observation. Set visibility to `UNKNOWN` when a slot must exist. Downstream must not invent a `FRESH` optional joint.
- Unsupported fingers stay at rest. They do not drive root or scale.

The body-model adapter owns topology mapping, rest-pose alignment, twist allocation, axes, parametric-model decoding, and uncertainty propagation. It must provide both rotations and global positions. Rotations alone are insufficient to compare adapters, preserve end-effectors, or diagnose convention errors.

No source parametric mesh crosses this boundary. This makes spatial models replaceable. It does not change the license of the model used above it.

### 4.3 Status and stage outcomes

```text
TrackingLifecycle = TRACKING | PARTIAL | PREDICTING | HOLDING | LOST | REACQUIRING

TrackingStatus
  lifecycle
  last_observation_age
  visibility_masks
  estimate_origin_masks
  root_support
  uncertainty_summary
  active_provider_and_device
  fallback_reason
  reacquisition_progress
  calibration_status

StageOutcome<T> =
  SUCCESS(T)
  NO_OBSERVATION(reason)
  STALE_INPUT(age)
  DEADLINE_MISSED(stage, elapsed)
  INVALID_OUTPUT(reason)
  PROVIDER_FAILURE(provider, reason)
```

The primary spatial model cannot have a deterministic estimator of equivalent capability as a fallback. Its deterministic failure behavior is to reject the observation, preserve the last valid state, increase uncertainty, and follow the loss lifecycle. “Every learned module has a fallback” means bounded system behavior. It does not mean a fictional duplicate model.

### 4.4 Retargeting and model packaging

```text
ContactIntent
  source_region
  target: FLOOR | AVATAR_REGION | OBJECT_SURFACE
  desired_relative_transform
  contact_mode: RIGID | SLIDING | INTERMITTENT
  confidence, priority
  latch_state

RetargetProfile
  avatar_asset_hash
  VRM_version
  humanoid_map, rest_transforms
  conservative_joint_limits
  sparse_proxy, semantic_regions
  reach_envelopes, contact_sites

RetargetResult
  target_root, target_joint_rotations[]
  contact_residuals, penetration_residuals
  limit_residuals, iteration_count
  deadline_status, fallback_status

ModelManifest
  model_id, version, file_hashes
  source_and_license_reference
  preprocessing_and_tensor_schema
  canonical_adapter_version
  supported_providers_and_precision
  minimum_memory_observed
  numerical_tolerances

InferencePlacement
  provider
  device_id
  precision_mode
  model_package_id
```

The first useful release creates one `InferencePlacement`. Later multi-GPU mode may create independent placements for body, face, hands, or other modalities.

## 5. State ownership and reset rules

| State | Lifetime | Owns | Reset trigger |
|---|---|---|---|
| Site | Until camera geometry changes | Intrinsics estimate, floor, `T_W_C`, capture volume, calibration confidence | Camera or mode change, or failed validation |
| Avatar | Until asset changes | VRM mapping, rest transforms, expression map, sparse proxy, semantic regions | New or reimported avatar |
| Performer | Current session | Locked scale and proportions, fit confidence | Confirmed handoff or session end |
| Spatial cache | Compatible inference run | ROI, crop transform, last observation, optional feature state | Model, provider, shape, crop, camera, or performer discontinuity |
| Estimator | Seconds | Authoritative source pose, root, velocities, contacts, uncertainty, lifecycle | Confirmed handoff. Partial reset on calibration change |
| Retarget | Avatar and performer pair | Warm start, target velocities, latched target contacts, residual history | Avatar or performer change |

Site state may be estimated using a performer. It stores no identity and no reusable body signature. That does not make the estimate immune to performer bias. Development tests must calibrate with one performer and validate with another.

## 6. Capture, preprocessing, and spatial perception

### 6.1 Capture

Windows capture begins with Media Foundation. The backend enumerates actual modes and records delivered format, resolution, cadence, timestamp behavior, and controls. Color conversion and resize must remain on the GPU when that lowers measured end-to-end latency. Correctness comes before avoiding one transfer.

Capture uses a bounded newest-frame queue. When inference falls behind, drop and count stale unprocessed frames. Do not build latency. A frame already referenced by a recording tap remains valid until that tap releases it.

### 6.2 Spatial-model comparison

No spatial model is selected by prose. Phase 0 in `file_architecture.md` evaluates:

- SAM 3D Body or Fast SAM 3D Body. They supply body, feet, and hands. They do not supply facial-expression output.
- Multi-HMR, including the Anny checkpoint, as a research baseline.
- NLF as a research baseline.
- Another redistributable or exportable candidate if the above fail.

The release status and restrictions for each candidate live in the model ledger in [`overall_ideal.md`](overall_ideal.md).

The first gate is successful export and runtime execution through `ort`. Surviving candidates are compared for canonical-output fidelity, feet and hands, flips, truncation behavior, runtime, memory, provider coverage, and model-package terms.

The Arc B580 path evaluates ONNX Runtime-compatible Intel or OpenVINO execution and DirectML. The RTX 4060 path evaluates CUDA and DirectML. TensorRT is optional after the ordinary ORT path works. Different provider graphs are allowed only behind the same manifest and adapter contract.

### 6.3 ROI, coverage, and optional fast 2D

A separate fast-2D model is retained only if it measurably improves one or more of:

- Person ROI and crop stability.
- Per-joint visibility or uncertainty.
- Motion-onset detection between spatial observations.
- Keypoint or mask prompting for the chosen spatial model.

Confidence logits or heatmap sharpness are not treated as calibrated covariance. Calibration uses held-out observations with empirical error measurements.

### 6.4 Adaptive inference

The first live slice may run dense spatial inference at the highest stable rate. Adaptive scheduling is a later optimization with explicit decisions:

- `FRESH` — submit a new inference.
- `REUSE` — advance state without pretending the old observation has a new timestamp.
- `FLUSH` — invalidate reusable state and reacquire.

Maximum observation age forces periodic fresh inference. Motion onset, crop discontinuity, exposure change, camera movement, confidence collapse, and occlusion recovery can force fresh inference or a flush. Any feature-level token reuse remains an experiment and always retains a dense stateless fallback.

## 7. Calibration and performer fit

### 7.1 Product calibration

The shipping workflow may ask for one height and guided human poses. It never asks for a board.

Site calibration estimates:

- Approximate focal length and intrinsics with retained uncertainty.
- Gravity and up direction.
- Floor plane and camera height and tilt.
- A supported capture region.
- A validation score and evidence summary.

Observables and what they constrain:

| Input | Constrains | Does not solve alone |
|---|---|---|
| Upright standing | Gravity, approximate camera tilt, pelvis image height | Focal length, metric scale |
| Entered height, if plausible | Metric scale, coupled camera height | Focal length, floor orientation, limb ratios |
| A few steps | Floor plane from foot travel, stride-based scale check | Intrinsics |
| Slow turn | Yaw observability, shoulder width, hip width | Floor height |

Ill-posed pair: focal length and metric scale, and camera distance, stay coupled without height. With height, focal length and camera distance still couple. Retain that uncertainty. Without height, operate with relative or population scale. Do not report metric depth as equally certain.

Performer fit estimates limb ratios, shoulder width, hip width, and other low-dimensional proportions, then locks them. It never rewrites site state. Provisional tracking can start with population priors while the UI offers a fit for better grounding.

### 7.2 Development validation

Development evaluation may use checkerboards, fiducials, measured camera height, tape-measured floor points, synchronized reference cameras, or motion-capture markers. These measure the error of the board-free product solve. They are not product dependencies.

Acceptance covers:

- Reprojection residual against independent 2D annotations.
- Floor normal, height, and camera-pose error against measured reference.
- Height and scale closure.
- Cross-performer validation.
- Repeat-session stability.
- Moved-camera detection.

Self-consistency is never labeled absolute accuracy. A coupled focal, scale, and floor solution can be consistently wrong.

## 8. Authoritative causal estimator

### 8.1 State

The estimator owns the source root transform and velocity. It also owns local joint rotations and angular velocities, global joint positions, contact probabilities, per-joint uncertainty and age, and the tracking lifecycle. A separate candidate holds reacquisition state until confirmation.

### 8.2 Analytical method

The first useful release ships this deterministic method. It is also the permanent debugging and fallback baseline.

- Predict with constant root linear velocity and constant local angular velocity.
- Bound the unsupported prediction by the `PREDICTING` horizon in `file_architecture.md`.
- Store diagonal or block-diagonal covariance. Do not store a dense full-body covariance in the first useful release.
- Apply one confidence-weighted and age-weighted correction toward the validated observation.
- Keep performer proportions fixed after lock.
- Apply contact hysteresis and residual-based release.
- Solve a weighted floor and root correction inside that same update.
- Cap joint and root innovation.
- Apply the hard feasibility projection in §8.3.
- Follow the explicit lifecycle transitions.

This is not an iterated extended Kalman filter. It is not a sequential quadratic program. It is not a learned smoother. A later Phase 4 model may replace only the residual correction. Deterministic feasibility stays outside the network.

A learned model is accepted only if it improves declared held-out metrics without worsening latency, rollout stability, or failure containment.

### 8.3 Hard constraints and objectives

The post-update feasibility layer enforces, in order:

1. Finite normalized transforms and valid topology.
2. Locked source bone lengths.
3. Conservative hard joint and velocity bounds.
4. Bounded floor penetration and root correction.
5. Last-valid fallback if feasibility cannot be restored.

Within that feasible region, confidence-weighted objectives prefer compatible latched contacts, floor support, reprojection, temporal continuity, and spatial observations. A contact may soften or release. It may not force an invalid pose.

### 8.4 Innovation gating

Correction authority depends on uncertainty, observation age, visibility, crop support, physical plausibility, and which body region is observed. Root translation and performer scale require torso or pelvis support, or another explicit global anchor. Per-tick caps prevent one observation from teleporting the state. Large consistent evidence is integrated over multiple updates or handled as reacquisition.

### 8.5 Loss and reacquisition

Lifecycle numbers live in `file_architecture.md`. This table states behavior only.

| State | Behavior |
|---|---|
| `TRACKING` | Normal bounded updates from supported evidence |
| `PARTIAL` | Update supported chains. Predict others. Gate root and scale |
| `PREDICTING` | No reliable global observation. Bounded prediction for at most the `PREDICTING` horizon |
| `HOLDING` | After that horizon, exponentially damp unsupported velocities until `LOST` |
| `LOST` | Hold last valid pose and root. Continue finite output-clock publication |
| `REACQUIRING` | Build an isolated candidate, confirm it, clear incompatible contacts, then blend |

Confirmation, blend duration, and per-tick correction caps are the reacquisition numbers in `file_architecture.md`. During the blend, apply those caps. A session grace window may reconnect a compatible track. A discontinuous candidate becomes a performer handoff and clears performer-dependent state while retaining valid site and avatar state.

### 8.6 Learned upgrade

If the analytical baseline leaves repeatable failures, train a compact causal transformer or state-space model over irregular observations. It predicts residual pose and state correction, velocities, contacts, uncertainty, and a short horizon. Deterministic feasibility remains outside the network.

Training data must include reference motion and contact targets, upstream model outputs, timestamp jitter, empirically matched spatial errors, separately labeled occlusion and truncation, varied observation intervals, full loss, false candidates, re-entry, and performer replacement. Synthetic corruption supplements paired real failures. It does not substitute for them.

Data-rights and drop rules live in Phase 4 of `file_architecture.md`.

## 9. Avatar import and retargeting

### 9.1 Import

VRM import validates version, humanoid map, required and optional bones, rest transforms, axes, scale, and mesh or skin availability. It builds:

- A conservative normalized rotation and IK mapping.
- Per-bone capsules or spheres for approximate collision tests.
- Sparse skinned anchors near soles, palms, fingers, face, chest, hips, and thighs.
- Semantic surface regions and contact sites.
- Reach envelopes and preferred bend directions.
- Conservative default joint limits with provenance (`DEFAULT`, `IMPORTED`, or `USER_OVERRIDE`).

The full render mesh is not evaluated every output tick. Import probes reach, squat, sit, step, twist, self-hug, hand-to-face, and hands-together. Report unsafe automatic repairs. Do not guess them.

### 9.2 Baseline and solver

The rotation and IK baseline is always available. It uses normalized rotation transfer, root mapping, and terminal IK. The geometry-aware solver is a later quality layer:

1. Seed from the valid baseline pose.
2. Extract normalized motion and contact intent.
3. Project unreachable exact points into semantically appropriate reachable regions.
4. Run a warm-started fixed-iteration sparse optimization.
5. Apply the hard feasibility projection.
6. Publish only a complete valid result. Otherwise blend the baseline from the last valid target state.

Hard validity outranks every visual objective. Within the feasible set, objective priority is: established floor or object contacts, reachable self-contact, penetration avoidance, balance and travel, end-effector orientation, source resemblance, then smoothness and rest regularization.

### 9.3 VMC output

The VMC adapter consumes only the final `RetargetResult`. It never consumes raw source joints. It owns:

- Avatar-root and humanoid-bone mapping.
- Coordinate and handedness conversion.
- Quaternion normalization.
- VMC status and relative-time messages.
- Expression-name mapping.
- Send cadence and UDP error counters.

Default bone output follows the VMC recommendation for original, non-normalized avatar bone transforms. A normalized compatibility mode, if implemented, is explicit and disabled by default. Test VRM 0 and VRM 1 expression aliases separately. Advanced 52-channel face output requires corresponding avatar custom expressions. Do not assume every VRM supplies them.

The internal `TrackingStatus` is richer than VMC receivers can consume. The adapter sends the standard status representation where supported and keeps full diagnostics in preview and MCAP.

Named reference receivers live in `file_architecture.md`.

### 9.4 Preview path

Rust owns the output-clock pose. The `iced` view receives a coalesced snapshot at UI rate. The view never sits between the estimator and an output adapter.

A later in-process `wgpu` preview may render through an `iced` shader widget. It must use the same `RetargetResult` as VMC.

## 10. Scheduling, concurrency, and devices

The runtime has logical domains for capture, preprocessing, inference, estimator updates, output-clock retarget, VMC publication, recording, and UI control. The implementation may combine lightweight domains until profiling justifies dedicated threads.

- Capture and inference queues are bounded and prefer recent frames.
- The output loop never blocks on inference, recording, UI, or network I/O.
- A missed inference deadline rejects that result. It does not stall the output clock.
- A retarget deadline miss publishes a complete deterministic fallback.
- Recording failure disables recording with a visible error. It does not stop live tracking.
- Camera disconnect enters the loss lifecycle and periodically attempts controlled reacquisition.
- Provider failure is reported and does not silently switch to a slower provider.
- Model warm-up completes before latency is labeled steady state.

The first useful release owns one selected provider and device. A later multi-GPU scheduler may place independent sessions on explicit device IDs. Timestamp alignment occurs after those sessions exactly as it does for asynchronous observations on one device. Modality-level placement is preferred. Tensor or model parallelism is out of scope unless profiling later proves it necessary.

## 11. Evaluation hooks

Every stage is replayable from timestamped MCAP channels. Recordings include enough metadata to identify camera mode, model package, provider and device, precision, preprocessing, adapter version, configuration, scheduling decisions, lifecycle transitions, residuals, and fallbacks.

Deterministic CPU and reference fixtures require exact equality. GPU and provider comparisons use declared tensor tolerances and, more importantly, end metrics: joint and rotation difference, contacts, root trajectory, failure events, and final avatar motion.

Engine metrics include:

- Capture delivery and timestamp quality.
- Observation age and spatial invocation rate.
- Per-stage p50, p95, and p99 latency and memory.
- Jitter during labeled stillness.
- Foot velocity and distance during labeled contact.
- Root jump and unsupported displacement.
- Flip and discontinuity count.
- Contact precision, recall, and residuals.
- Lifecycle duration, false reacquisition, and recovery correction.
- Retarget contact-region error, penetration, limits, runtime, and fallback.
- Output-tick and VMC-send misses.
- Visible failure events per hour.

Concrete fixtures, thresholds, and release decisions belong in `file_architecture.md`.

## 12. Research contingencies

The following are optional experiments. They are not prerequisites for a useful tracker:

- A fast-2D uncertainty or ROI model.
- Adaptive dense-inference scheduling.
- Feature or token reuse in a vision transformer.
- Sapiens2 or another pointmap or depth prior.
- A compact distilled spatial student.
- Learned residual retargeting.
- Multi-GPU modality placement.
- Multiple cameras.

Each requires a named baseline, one targeted hypothesis, an export and provider check before substantial training, and a keep or drop result recorded from the same replay suite.

## 13. Known risks

| Risk | Required response |
|---|---|
| No candidate exports and runs well on both reference GPU classes | Keep the simplest exportable baseline. Re-plan a student only after profiling and a viable supervision route |
| Provider paths disagree materially | Find the operator or precision cause, or ship no common configuration. Do not hide model differences |
| Calibration is self-consistent but wrong | Compare with instrumented development references and cross-performer trials |
| Partial crops alter apparent scale | Preserve unclamped ROI and edge evidence. Deny unsupported root and scale authority |
| Contact errors create sticky feet | Use hysteresis and residual-based release. Never let contacts override feasibility |
| Learned temporal rollout drifts | Retain the analytical baseline. Evaluate long autoregressive loss cases |
| Avatar proxy is wrong | Validate import probes, expose defaults and overrides, and fall back to the rotation and IK baseline |
| VMC works in only one receiver | Test the named reference receivers and VRM 0 and 1 configurations |
| Multi-GPU overhead erases benefit | Compare with the one-GPU baseline and drop the mode if transfers or synchronization lose |
| Raw recordings expose sensitive data | Make recording explicit and local. Keep diagnostic contents inspectable |
| Phase 0 export consumes months | Start types, capture, and MCAP in parallel. Keep contracts stable |

## 14. Prior-art position

Senrigan did not invent grounded monocular capture, temporal human mesh recovery, contact reasoning, or geometry-aware retargeting. It focuses on integration, live failure behavior, fixed-session specialization, avatar-space evaluation, and accessible packaging.

Named papers, products, and replacement classes live in [`prior_art.md`](prior_art.md). Recheck external capabilities and terms at implementation time.
