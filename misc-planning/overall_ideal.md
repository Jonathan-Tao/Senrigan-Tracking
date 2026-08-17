# Senrigan — Product Definition

**Status:** Design. No implementation yet.
**Authority:** this is the authoritative product-direction document. The companion documents must not contradict it.
**Purpose:** define the vision, experience, supported workflows, quality bar, limitations, privacy posture, and release posture.

Algorithm and code-structure decisions belong in [`engine_concept.md`](engine_concept.md). Phases, owned numbers, and gates belong in [`file_architecture.md`](file_architecture.md). Research and product comparisons belong in [`prior_art.md`](prior_art.md).

---

## Glossary

Use these names. Do not rotate synonyms.

| Name | Meaning |
|---|---|
| performer | The human in front of the camera for this session |
| site | Saved camera, floor, and capture-volume state |
| spatial model | The per-frame or near-per-frame body network |
| first useful release | The first public application. Phase 0 plus Phase 1 plus Phase 2 |
| live baseline | Internal Phase 1 checkpoint. Not a public release |
| rotation and IK baseline | Normalized rotation transfer, root mapping, and terminal inverse kinematics |

Define each short form once. Then use the short form.

| Short form | Full form |
|---|---|
| VRM | Virtual Reality Model |
| VMC | Virtual Motion Capture |
| MCAP | MCAP recording container |
| ONNX | Open Neural Network Exchange |
| ORT | ONNX Runtime |
| GPU | graphics processing unit |
| CPU | central processing unit |
| ROI | region of interest |
| IK | inverse kinematics |
| OSC | Open Sound Control |
| EP | execution provider |
| VRAM | video memory |
| UDP | User Datagram Protocol |
| HMR | human mesh recovery |
| V4L2 | Video4Linux2 |
| UI | user interface |

American English: `license`, `behavior`, `normalize`, `meters`, `centimeters`, `artifacts`.

---

## 1. What Senrigan is

Senrigan is a local application that converts one ordinary webcam into live 3D humanoid-avatar motion. A user selects a camera, imports a VRM, completes any setup requested by the current tracking mode, and sends the result to another application through VMC or views it in the built-in preview.

It is a hobby and research-engineering project. The objective is stable, expressive, useful motion. Establishing a company or maximizing a commercial market is not a project goal. Engineering comparisons answer whether the result looks and behaves better on named replays.

The evaluation target is single-camera motion that a seated or standing VTuber can use for ordinary streaming. This is not a claim that one camera removes monocular depth ambiguity. It is not a claim that one camera matches every tracked motion.

The first supported deployment is Windows on one dedicated desktop GPU from either reference class:

- Intel Arc B580.
- NVIDIA RTX 4060 or a materially equivalent desktop 4060-class card.

One GPU is enough to develop. Both classes are first-useful-release gates. Senrigan is the primary GPU workload for that performance promise. Other GPUs, shared-GPU gaming, and virtual machines are best-effort until measured.

## 2. Product principles

1. **One camera must be worthwhile.** Extra cameras may help later. They cannot be required for the main experience.
2. **Stable timing and planted feet outrank millimeter joint error.** Biomedical accuracy is not the objective.
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

- Medical, biomechanical, ergonomic, or centimeter-accurate measurement.
- Guaranteed recovery of fully hidden motion.
- Crowd tracking or persistent biometric identity.
- A general avatar renderer, streaming studio, or VRChat-specific tracker.
- Tracker-equivalent 360-degree dance capture from one frontal webcam.
- Supporting every GPU, avatar format, or output protocol in the first useful release.

## 4. User workflows

### 4.1 First run

1. The application verifies the selected model package and displays its source, version, license label, and relevant restrictions.
2. The user selects a camera mode. The UI warns about low resolution, unstable frame delivery, severe blur, and insufficient full-body framing.
3. The user imports a VRM. Import validates the humanoid map, rest pose, required bones, axes, and basic reachability before tracking begins.
4. The user may enter their height. Skipping it uses a declared population prior and lowers metric-scale confidence without preventing a basic session.
5. In modes with site calibration, the UI guides upright standing, a few steps, and a slow turn. No printed target is requested.
6. A short performer fit estimates proportions and then locks them.
7. Preview and VMC diagnostics confirm that the receiving application sees the same final pose.

The live baseline may exist before every Phase 2 quality feature is finished. It is an internal checkpoint. The first useful release must run those features and must label which quality features are active.

### 4.2 Repeat session

If the camera and camera mode match saved site state, the application validates the previous calibration rather than silently trusting it. A current performer fit remains ephemeral by default. A new session begins with provisional proportions and can run a short fit for higher quality.

### 4.3 Camera movement

A persistent disagreement between the image and saved site geometry marks site state stale. Tracking may continue in camera-relative fallback mode. Grounded-world claims and hard floor contacts stay disabled until recalibration succeeds. Site state is never silently rewritten to accommodate a moved camera.

### 4.4 Performer handoff

When a new performer enters, Senrigan can begin provisional tracking using conservative population priors. It retains site state and the selected avatar. It clears performer, spatial-cache, temporal, contact, and retarget history. A guided fit is offered for correct metric scale and best grounding.

The application does not claim to recognize a returning performer. A short grace window may reconnect a geometrically and temporally compatible track. That is session continuity. It is not biometric identity.

### 4.5 Partial or complete loss

- Visible joints continue updating when trustworthy.
- Unsupported joints predict briefly, then damp and hold with rising uncertainty.
- A hand, foot, or face alone cannot rescale or translate the whole performer.
- Predicted joints cannot create new hard contacts.
- Complete loss does not stop the output clock or destroy valid site or avatar state.
- Re-entry is confirmed as a candidate before bounded blending begins.
- The preview exposes lifecycle, stale age, truncated regions, fallback, and reacquisition progress.

Timing values live in [`file_architecture.md`](file_architecture.md).

## 5. Experience and performance targets

Targets are hypotheses until Phase 0 measures an exported model on the development GPU, and until both reference GPUs are measured for the first useful release. Measured results replace estimates. The documents must not silently turn an estimate into a claim.

Numeric values live in [`file_architecture.md`](file_architecture.md). This table states the owned target names only.

| Area | Owned target |
|---|---|
| First setup | First-setup duration |
| Repeat setup | Repeat-setup duration |
| Warm performer acquisition | Provisional-tracking duration |
| Avatar and VMC output | Output clock, independent of spatial-observation rate |
| Output continuity | Continuity over the endurance replay |
| Capture-to-VMC body latency | Median and p95 latency hypotheses |
| Lost performer | Finite, status-labeled output. No accumulated root drift after the hold transition |
| Camera moved | Explicit stale-calibration warning and camera-relative fallback |
| Unsupported hardware or provider | Clear no-go report. No silent slow provider fallback |
| Avatar import failure | Actionable rig, bone, or axis error rather than a corrupt live pose |

Quality gates for jitter, foot skating, root correction, reacquisition, and retargeting live in `file_architecture.md`, where they are tied to named replays and baselines.

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

Senrigan may generate a plausible continuation during missing evidence. The status must not describe that output as freshly observed.

## 7. Hardware and compatibility policy

Both reference GPU classes are first-useful-release gates. Passing on one does not make the other optional. Development may proceed on one class.

Provider-specific preprocessing, graphs, precision, kernels, and batching are allowed when all of the following hold:

- They implement the same canonical motion schema.
- Numerical differences remain within declared tensor and end-metric tolerances.
- Neither path uses a materially worse model without an explicit configuration label.
- Model, provider, and device identity is present in recordings and diagnostics.
- Provider failure is visible and never becomes an unreported CPU fallback.

One GPU is required for the initial application. Later multi-GPU mode may assign independent body, face, hand, instrument, or experimental-prior sessions to separate device IDs. It is not a promise of tensor parallelism, automatic heterogeneous balancing, or distributed-machine execution.

## 8. Privacy and recordings

Tracking is local. Local recordings are still sensitive. Raw webcam frames can reveal a face, body, room, possessions, and other people.

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

The application aims to offer a convenient bundle. Model artifacts retain their own terms. “Low enforcement risk” is not recorded as license permission. Restricted models may still be valuable local experiments when their terms allow that use.

| Artifact | Current planning status | Release handling |
|---|---|---|
| Senrigan code | GPLv3 intent | Add the actual repository license before distributing code |
| ONNX Runtime | MIT | Bundle notices with the pinned runtime |
| `ort` | MIT and Apache-2.0 | Pin and record the selected version |
| SAM 3D Body / MHR | Custom SAM license for body, feet, and hands | Lead technical candidate, subject to exact redistribution review before bundling |
| Fast SAM 3D Body | MIT code reported. Weights follow SAM 3D Body terms | First-class Phase 0 candidate. Not automatically bundle-clean |
| Multi-HMR `multiHMR_672_L_anny` | Non-commercial checkpoint | Research baseline. Not described as license-clean |
| NLF | MIT code with released weights reported as non-commercial | Research baseline unless a separately approved weight exists |
| RTMPose / RTMW | Apache-2.0 code with checkpoint terms checked separately | Optional fast-2D candidate |
| MediaPipe Face Landmarker | Apache-2.0 implementation | Phase 5 face baseline whose coefficients require avatar mapping |
| Sapiens / Sapiens2 | Restricted or custom model terms | Optional research probe, never a Phase 1 dependency |

Before a public bundle, record for every included model: exact file hash, source URL, version, code license, checkpoint license, body-model terms, required notices, redistribution decision, and known use restrictions. This table is a planning aid. It is not legal advice.

The body-model-to-skeleton adapter is the replacement boundary. It does not remove obligations created by using or distributing the model on the other side of that boundary.

## 10. Baselines and claims

Existing webcam products demonstrate that the workflow is useful. They are baselines, not targets to dismiss. Named products live in [`prior_art.md`](prior_art.md).

| Baseline class | Why compare |
|---|---|
| XR Animator or another VMC-capable webcam tracker | Incumbent one-camera user experience and resource use |
| MediaPipe-class per-frame pose plus smoothing | Minimum reproducible technical baseline |
| Raw selected spatial model | Shows what temporal, grounding, and containment add |
| Analytical Senrigan state estimator | Required baseline before training a temporal model |
| Rotation and IK baseline | Required baseline before geometry-aware retargeting |
| Tracker or multi-camera reference | Optional later comparison. Not assumed ground truth |

Every statement about quality is labeled as one of:

- **Hypothesis:** A proposed claim that has not yet been measured.
- **Prototype observation:** An informal observation from named hardware and clips.
- **Measured result:** A result from a documented replay, metric, configuration, and version.
- **Public comparison:** A measurement against a named external baseline with the setup and failures disclosed.

“Parity,” “solved,” “nobody has shipped this,” and similar categorical claims are avoided unless the evidence actually supports them.

## 11. Replacement scope

Fact: the first useful release aims at one-camera VTuber tracking with VRM import and VMC output.

Recommendation: after measurement, treat XR Animator, a MediaPipe-class baseline, and Webcam Motion Capture as the replacement class. Do not claim that class is replaced before a public comparison exists.

Wearable inertial kits, tracked-point systems, multi-camera tools, cloud video services, and professional measurement stages stay out of scope for the first useful release. A later VRChat adapter may consume the same `RetargetResult`. It is not a first-useful-release output.

Details live in [`prior_art.md`](prior_art.md).

## 12. Release outline

There is one phase map. It lives in [`file_architecture.md`](file_architecture.md).

The first useful release is Phase 0 plus Phase 1 plus Phase 2 on both reference GPU classes. Later phases are optional research. They are not a second product roadmap.
