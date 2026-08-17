# Senrigan — Product Definition

**Status:** Design. No implementation yet.
**Authority:** this is the authoritative product-direction document. The companion documents must not contradict it.
**Purpose of this document:** define the vision, experience, supported workflows, quality bar, limitations, privacy posture, and release posture. Algorithm and code-structure decisions belong in the companion documents.

---

## 1. What Senrigan is

Senrigan is a local application that converts one ordinary webcam into live 3D humanoid-avatar motion. A user selects a camera, imports a VRM, completes any setup requested by the current tracking mode, and sends the result to another application through VMC or views it in the built-in preview.

It is a hobby and research-engineering project. The objective is to build something stable, expressive, and useful. Establishing a company or maximizing a commercial market is not a project goal. Engineering comparisons answer “does this look and behave better?”

The north star is single-camera motion that feels good enough for ordinary streaming without making the user wish they had built a tracker or multi-camera setup. This is an evaluation target, not a claim that one camera can remove monocular depth ambiguity or match every tracked motion.

The first supported deployment is Windows on one dedicated desktop GPU from either reference class:

- Intel Arc B580.
- NVIDIA RTX 4060 or a materially equivalent desktop 4060-class card.

Senrigan is the primary GPU workload for the initial performance promise. Other GPUs, shared-GPU gaming, and virtual machines are best-effort until measured.

## 2. Product principles

1. **One camera must be worthwhile.** Extra cameras may help later, but they cannot be required for the main experience.
2. **Jank matters more than centimetres.** Stable timing, readable motion, grounded feet, bounded corrections, and predictable failure matter more than biomedical accuracy.
3. **Do not conceal missing evidence.** The UI and output status distinguish fresh, predicted, held, and fallback motion.
4. **User setup stays board-free.** Development may use calibration targets and measured references. The shipping workflow may not require them.
5. **Fit once, then lock.** Performer proportions must not breathe frame to frame. Slow adaptation is allowed only when it cannot absorb pose or camera error.
6. **Local first.** No video upload, account, cloud inference, or subscription is required.
7. **Interoperate early.** A release that only animates its own preview is not the first useful release. VMC output is required.
8. **Measure before specializing.** Model choice, refinement paths, acceleration, and learned upgrades follow replayed comparisons on both reference GPUs.

## 3. Intended use

### Primary use

- One seated or standing performer talking, gesturing, reacting, and moving within a webcam-sized capture area.
- A fixed camera for a session and usually across repeat sessions.
- A VRM humanoid avatar, including avatars whose proportions differ substantially from the performer.
- Local output to a VMC-compatible renderer or VTuber application.

### Later experiments

- Higher-detail face, gaze, and finger tracking.
- Contact-aware instrument performance, beginning with one instrument family.
- Multiple GPUs for independent modality inference.
- Multiple cameras for additional observations.

### Non-goals

- Medical, biomechanical, ergonomic, or centimetre-accurate measurement.
- Guaranteed recovery of fully hidden motion.
- Crowd tracking or persistent biometric identity.
- A general avatar renderer, streaming studio, or VRChat-specific tracker.
- Tracker-equivalent 360-degree dance capture from one frontal webcam.
- Supporting every GPU, avatar format, or output protocol in the first release.

## 4. User workflows

### 4.1 First run

1. The application verifies the selected model package and displays its source, version, licence label, and relevant restrictions.
2. The user selects a camera mode. The UI warns about low resolution, unstable frame delivery, severe blur, and insufficient full-body framing.
3. The user imports a VRM. Import validates the humanoid map, rest pose, required bones, axes, and basic reachability before tracking begins.
4. The user may enter their height. Skipping it uses a declared population prior and lowers metric-scale confidence without preventing a basic session.
5. In modes with site calibration, the UI guides upright standing, a few steps, and a slow turn. No printed target is requested.
6. A short performer fit estimates proportions and then locks them.
7. Preview and VMC diagnostics confirm that the receiving application sees the same final pose.

The first usable release may provide the live baseline before every advanced calibration feature exists, but it must label which quality features are active.

### 4.2 Repeat session

If the camera and camera mode match saved site state, the application validates the previous calibration rather than silently trusting it. A current performer fit remains ephemeral by default. A new session begins with provisional proportions and can run a short fit for higher quality.

### 4.3 Camera movement

A persistent disagreement between the image and saved site geometry marks site state stale. Tracking may continue in camera-relative fallback mode, but grounded-world claims and hard floor contacts are disabled until recalibration succeeds. Site state is never silently rewritten to accommodate a moved camera.

### 4.4 Performer handoff

When a new person enters, Senrigan can begin provisional tracking within seconds using conservative population priors. It retains camera/site state and the selected avatar, but clears performer, spatial-cache, temporal, contact, and retarget history. A guided fit is offered for correct metric scale and best grounding.

The application does not claim to recognize a returning person. A short grace window may reconnect a geometrically and temporally compatible track, but it is session continuity rather than biometric identity.

### 4.5 Partial or complete loss

- Visible joints continue updating when trustworthy.
- Unsupported joints predict briefly, then damp and hold with rising uncertainty.
- A hand, foot, or face alone cannot rescale or translate the whole performer.
- Predicted joints cannot create new hard contacts.
- Complete loss does not stop the 60 Hz output clock or destroy valid site/avatar state.
- Re-entry is confirmed as a candidate before bounded blending begins.
- The preview exposes lifecycle, stale age, truncated regions, fallback, and reacquisition progress.

## 5. Experience and performance targets

Targets are hypotheses until Phase 0 measures an exported model on both reference GPUs. Measured results replace estimates. The documents must not silently turn an estimate into a claim.

| Area | Initial target |
|---|---|
| First setup | Under five minutes for a valid VRM and supported camera |
| Repeat setup | Under one minute when the camera has not moved |
| Warm performer acquisition | Provisional tracking within five seconds |
| Avatar/VMC output | 60 Hz, independent of spatial-observation rate |
| Output continuity | At least 99.9% of scheduled ticks over a normal 30-minute replay |
| Capture-to-VMC body latency | Provisional median ≤100 ms and p95 ≤150 ms on both reference systems |
| Lost subject | Finite, status-labelled output. No accumulated root drift after the hold transition |
| Camera moved | Explicit stale-calibration warning and camera-relative fallback |
| Unsupported hardware/provider | Clear no-go report. No silent slow provider fallback |
| Avatar import failure | Actionable rig/bone/axis error rather than a corrupt live pose |

Quality gates for jitter, foot skating, root correction, reacquisition, and retargeting live in `file_architecture.md`, where they can be tied to named replays and baselines.

## 6. Fundamental limitations

Monocular depth along the viewing ray is not observable from one image. Known scale, limb lengths, a floor estimate, contacts, and temporal history reduce the set of plausible motions. They do not reveal hidden truth.

Expected weak cases include:

- Reaching directly toward or away from the lens.
- Limbs hidden behind the torso or another object.
- Fast turns and near-symmetric silhouettes.
- Crouching, floor work, or both feet leaving their ordinary support region.
- Fingers pointed at the lens.
- Permanent desk crops and narrow webcam fields of view.
- Motion blur, low light, rolling-shutter distortion, and dropped frames.
- Unusual rigs, missing VRM bones, or avatars whose geometry cannot support the requested contact.
- A GPU shared with a game or other heavy inference workload.

Senrigan may generate a plausible continuation during missing evidence, but the status must not describe that output as freshly observed.

## 7. Hardware and compatibility policy

Both reference GPU classes are first-release gates. Passing on one does not make the other optional.

Provider-specific preprocessing, graphs, precision, kernels, and batching are allowed when all of the following hold:

- They implement the same canonical motion schema.
- Numerical differences remain within declared tensor and end-metric tolerances.
- Neither path uses a materially worse model without an explicit configuration label.
- Model/provider/device identity is present in recordings and diagnostics.
- Provider failure is visible and never becomes an unreported CPU fallback.

One GPU is required for the initial application. Later multi-GPU mode may assign independent body, face, hand, instrument, or experimental-prior sessions to separate device IDs. It is not a promise of tensor parallelism, automatic heterogeneous balancing, or distributed-machine execution.

## 8. Privacy and recordings

Tracking is local, but local recordings are still sensitive. Raw webcam frames can reveal a face, body, room, possessions, and other people.

- Routine tracking does not automatically create an MCAP recording.
- Recording requires an explicit user action and shows an unmistakable active indicator.
- Files remain local and are never uploaded automatically.
- The UI exposes the recording directory and supports deletion.
- Diagnostic export lists whether it includes raw frames, crops, audio, avatar assets, proportions, or only derived metrics.
- Raw video is excluded from a diagnostic bundle unless the user explicitly includes it.
- Performer state is ephemeral by default. The project does not build a persistent identity index.
- Test recordings require consent from recorded participants and a documented intended use.

Encryption-at-rest is not promised for the first hobby release. The application relies on operating-system file permissions and states that limitation plainly.

## 9. Model and dependency release posture

The application aims to offer a convenient bundle, but model artefacts retain their own terms. “Low enforcement risk” is not recorded as licence permission. Restricted models may still be valuable local experiments when their terms allow that use.

| Artifact | Current planning status | Release handling |
|---|---|---|
| Senrigan code | GPLv3 intent | Add the actual repository licence before distributing code |
| ONNX Runtime | MIT | Bundle notices with the pinned runtime |
| `ort` | MIT/Apache-2.0 | Pin and record the selected version |
| SAM 3D Body / MHR | Custom SAM Licence for body, feet, and hands | Lead technical candidate, subject to exact redistribution review before bundling |
| Multi-HMR `multiHMR_672_L_anny` | Non-commercial checkpoint | Research baseline. Not described as licence-clean |
| NLF | MIT code with released weights reported as non-commercial | Research baseline unless a separately approved weight exists |
| RTMPose/RTMW | Apache-2.0 code with checkpoint terms checked separately | Optional fast-2D candidate |
| MediaPipe Face Landmarker | Apache-2.0 implementation | Phase 2 face baseline whose coefficients require avatar mapping |
| Sapiens/Sapiens2 | Restricted/custom model terms | Optional research probe, never a Phase 1 dependency |

Before a public bundle, record for every included model: exact file hash, source URL, version, code licence, checkpoint licence, body-model terms, required notices, redistribution decision, and known use restrictions. This table is a planning aid, not legal advice.

The body-model-to-skeleton adapter is an excellent replacement boundary. It does not remove obligations created by using or distributing the model on the other side of that boundary.

## 10. Baselines and claims

Existing webcam products demonstrate that the workflow is useful. They are baselines, not targets to dismiss.

| Baseline class | Why compare |
|---|---|
| XR Animator or another VMC-capable webcam tracker | Incumbent one-webcam user experience and resource use |
| MediaPipe-class per-frame pose plus smoothing | Minimum reproducible technical baseline |
| Raw selected spatial model | Shows what temporal, grounding, and containment add |
| Analytical Senrigan state estimator | Required baseline before training a temporal model |
| Rotation transfer plus terminal IK | Required baseline before geometry-aware retargeting |
| Tracker or multi-camera reference | Optional north-star comparison, not assumed ground truth |

Every statement about quality is labelled as one of:

- **Hypothesis:** A proposed claim that has not yet been measured.
- **Prototype observation:** An informal observation from named hardware and clips.
- **Measured result:** A result from a documented replay, metric, configuration, and version.
- **Public comparison:** A measurement against a named external baseline with the setup and failures disclosed.

“Parity,” “solved,” “nobody has shipped this,” and similar categorical claims are avoided unless the evidence actually supports them.

## 11. Release outline

1. Useful live body tracker on both reference GPU classes.
2. Incremental single-camera stability and retargeting releases.
3. Face and hand refinement.
4. Optional multi-GPU placement after independent body, face, and hand sessions exist.
5. One-family instrument experiment followed by optional generalization.
6. Optional multi-camera mode.

[`file_architecture.md`](file_architecture.md) defines the engineering order and exit gates. [`engine_concept.md`](engine_concept.md) defines the engine behavior.
