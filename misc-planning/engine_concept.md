# Commodity-Camera Avatar Tracking

## Detailed Technical Architecture Specification

**Revision 1.0 — research-cited architecture draft**  
**Status:** Proposed implementation architecture; not a claim of achieved performance  
**Research sweep current to:** August 16, 2026  
**Companion document:** `Commodity_Camera_Avatar_Tracking_Design_Person_Agnostic.md`  
**Citation audit:** 184 numbered architecture decisions (`TA-001`–`TA-184`); every numbered decision cites at least one research paper.

> **North star:** The user chooses a quality mode and camera count, imports a humanoid avatar, performs a few guided poses, and receives stable, convincing face, hand, and full-body animation on Windows or Linux—even from inexpensive, mixed-quality webcams. A calibrated camera site must work for any subsequent person without training on, identifying, or retaining the original calibration performer.

> **Optimization target:** Preserve the visible meaning, timing, contacts, balance, and expressiveness of a human performance after retargeting to a stylized VTuber or VRChat avatar. Centimeter-level physical error is acceptable when the avatar remains visually faithful. Jitter, foot skating, root pumping, joint flips, delayed gestures, hallucinated translation, and broken hand–prop contact are not.

---


# Table of contents

- [0. Evidence and decision policy](#0-evidence-and-decision-policy)
- [1. System goals, non-goals, and service-level objectives](#1-system-goals-non-goals-and-service-level-objectives)
- [2. End-to-end architecture](#2-end-to-end-architecture)
- [3. State domains and data contracts](#3-state-domains-and-data-contracts)
- [4. Capture, timing, and low-quality webcam handling](#4-capture-timing-and-low-quality-webcam-handling)
- [5. Person-agnostic site calibration](#5-person-agnostic-site-calibration)
- [6. Guest acquisition and performer-state estimation](#6-guest-acquisition-and-performer-state-estimation)
- [7. Avatar import and Avatar Profile generation](#7-avatar-import-and-avatar-profile-generation)
- [8. Per-camera perception stack](#8-per-camera-perception-stack)
- [9. Multi-camera geometry and learned fusion](#9-multi-camera-geometry-and-learned-fusion)
- [10. Causal temporal state estimator](#10-causal-temporal-state-estimator)
- [11. Root translation, floor, contacts, and hallucination containment](#11-root-translation-floor-contacts-and-hallucination-containment)
- [12. Full-body spatial representation](#12-full-body-spatial-representation)
- [13. Face, gaze, and lip tracking](#13-face-gaze-and-lip-tracking)
- [14. Hand and finger tracking](#14-hand-and-finger-tracking)
- [15. Guitar and instrument-performance mode](#15-guitar-and-instrument-performance-mode)
- [16. Avatar-aware retargeting and final animation solve](#16-avatar-aware-retargeting-and-final-animation-solve)
- [17. Failure containment and confidence state machine](#17-failure-containment-and-confidence-state-machine)
- [18. OS- and GPU-neutral inference architecture](#18-os--and-gpu-neutral-inference-architecture)
- [19. Realtime scheduler and quality controller](#19-realtime-scheduler-and-quality-controller)
- [20. Training and data architecture](#20-training-and-data-architecture)
- [21. Loss functions and supervision](#21-loss-functions-and-supervision)
- [22. Latency engineering in detail](#22-latency-engineering-in-detail)
- [23. Evaluation framework](#23-evaluation-framework)
- [24. Observability, debugging, and replay](#24-observability-debugging-and-replay)
- [25. Implementation roadmap](#25-implementation-roadmap-for-an-open-source-spare-time-project)
- [26. Repository and component interfaces](#26-recommended-repository-and-component-interfaces)
- [27. Security, update, and reproducibility](#27-security-update-and-reproducibility-architecture)
- [28. Architecture risks and mitigation](#28-architecture-risks-and-mitigation)
- [29. Technical release gates](#29-technical-release-gates)
- [30. Research references](#30-research-references)
- [31. Official implementation references](#31-official-implementation-references)

# 0. Evidence and decision policy

The user asked for a paper behind every technical decision. This document therefore distinguishes three kinds of statements:

1. **Normative architecture decisions** are labeled `TA-###`. Every normative decision cites at least one research paper.
2. **Implementation mappings** name current APIs, operating-system facilities, and inference providers. Those use official documentation in addition to papers because a paper cannot establish whether a particular runtime or driver is supported in 2026.
3. **Targets and budgets** are engineering hypotheses informed by prior latency and avatar studies. They are not reported benchmark results for this proposed system and must be validated on prototype hardware.

A citation supports the principle behind a decision; it does not guarantee that the proposed implementation will meet its target. All normative requirements are contained in `TA-###` blocks. Tables, pseudocode, and implementation examples are explanatory and inherit the adjacent numbered decisions. Current API/provider claims use official documentation because papers cannot establish 2026 platform support. Preprints are identified in the reference ledger and must be rechecked before implementation or publication.

## 0.1 Architectural invariants

The following invariants may not be weakened to simplify an early implementation:

- The site calibration is independent of performer identity.
- No runtime gradient update or permanent model personalization is allowed for an ordinary guest.
- All state crossing a person handoff is explicitly classified as site state or avatar state; performer and temporal state are destroyed.
- Every body joint, facial channel, hand joint, camera, and prop observation carries confidence and age.
- The output loop is causal. It may predict forward, but it may not wait for future frames to interpolate routine motion.
- No single weak camera observation may teleport the root or a limb.
- Additional cameras must improve evidence without changing the public workflow or requiring a different model topology.
- The shipped application must not require CUDA. Vendor-specific acceleration is optional behind a stable backend abstraction.
- The application must run on Windows and Linux; feature parity is required except where an operating-system integration is inherently platform-specific.

## 0.2 Core research thesis

Modern single-image HMR can provide a strong full-body hypothesis, but realtime avatar quality requires temporal consistency, world/root estimation, uncertainty, contacts, and retargeting rather than independent-frame mesh recovery alone. SAM 3D Body supplies a strong spatial teacher; OnlineHMR, WHAM, HuMoR, and SAM-Body4D establish the need for causal temporal reasoning, world-grounded state, motion priors, and occlusion-aware consistency. [P01] [P02] [P03] [P04] [P05] [P07]

Two or more cameras should not merely produce multiple independent poses that are averaged. Their 2D rays, timing, visibility, local features, and single-view hypotheses should be fused through robust geometry and a permutation-invariant learned residual model. This follows the evidence from learnable/probabilistic triangulation, uncalibrated-camera HMR, joint-specific view weighting, and set-based attention. [P15] [P19] [P20] [P21] [P22] [P23] [P24]

The final model output should be avatar-space motion, not a claim of biomedical measurement. Motion-retargeting research shows that preserving semantics, contacts, and geometry across different skeletons requires explicit skeleton- and shape-aware correction rather than simple bone copying. [P36] [P37] [P38] [P39]

---

# 1. System goals, non-goals, and service-level objectives

## 1.1 Product modes

| Mode | Minimum input | Intended use | Heavy 3D observation target | Final avatar output |
|---|---:|---|---:|---:|
| Eco | 1 camera | Talking, sitting, modest gestures on weak hardware | 12–15 Hz | 60 Hz |
| Balanced | 1–2 cameras | General VTubing and VRChat | 20–24 Hz | 60 Hz desktop / 90 Hz VR |
| Quality | 2 cameras recommended | Dance, turns, crouches, stronger hands and face | 25–30 Hz | 60/90 Hz |
| Performance | 2–4 cameras | Dance and instrument performance | 30–45 Hz body where hardware permits; 45–60 Hz face/hands | 60/90 Hz |
| Maximum | 3–N cameras | Enthusiast/studio capture volume | Hardware-bounded, scheduler controlled | 60/90 Hz |

These are initial operating points, not fixed product limits. The scheduler may reduce spatial-model frequency, crop resolution, active heavy views, hand refinement, or face detail independently.

### TA-001 — Separate observation rate from avatar output rate

The system shall permit expensive spatial reconstruction to run below display/output rate. A causal temporal model shall emit 60 Hz desktop or 90 Hz VR poses from asynchronous observations rather than requiring every model to execute at the output rate. Online causal HMR demonstrates streaming inference from current and past inputs, while avatar studies show that rate mismatch and latency artifacts must be modeled explicitly rather than hidden. [P01] [P12]

### TA-002 — Use causal prediction and reconciliation, not future-frame interpolation

Routine output shall not wait for the next observation. The temporal state predicts the present output timestamp and reconciles new evidence using bounded corrections. Future-frame interpolation may exist only in an optional recording/offline mode. This avoids adding a full observation interval of latency and follows online-HMR requirements for system-level causality. [P01] [P09]

### TA-003 — Treat latency as a first-class quality dimension

The runtime shall measure capture-to-pose and capture-to-output latency per modality and report median, p95, and p99 values. A VR study found motor performance and simultaneity judgments degraded above roughly 75 ms, while avatar research shows desynchronization, noise, occlusion, and frame-rate mismatch materially damage reconstructed motion. [P11] [P12] [P13]

## 1.2 Initial latency targets

The targets below are ambitious but technically coherent. They must be measured from the camera exposure timestamp, not from the moment a frame is dequeued by application code.

| Path | Recommended median | Recommended p95 | Rationale |
|---|---:|---:|---|
| Head orientation / gaze / face expression | 35–55 ms | <75 ms | Face and gaze changes are perceptually immediate and should not wait for full-body HMR. |
| VR HMD/controller anchors | Native VR runtime cadence | Runtime-defined | Do not unnecessarily route already tracked head/hands through camera inference. |
| Body pose, one camera | 65–90 ms | <115 ms | Allows one frame of capture plus spatial/temporal processing on ordinary hardware. |
| Body pose, two-camera Quality | 55–80 ms | <100 ms | Recommended target for dance and direct visual comparison. |
| Fingers | 45–75 ms | <100 ms | Dedicated crops and temporal hand solve; resolution is the limiting factor. |
| Root translation correction | 70–100 ms | <130 ms | Prefer stable, observed correction over speculative low-latency teleportation. |
| Avatar output tick | 11.1 ms at 90 Hz / 16.7 ms at 60 Hz | No missed tick under normal load | Output prediction must remain decoupled from heavy inference stalls. |

### TA-004 — Use differentiated modality latency budgets

Face, gaze, HMD/controller anchors, hands, body, and root translation shall have separate latency budgets and queues. A single monolithic model would force fast modalities to inherit the slowest path. Evidence that avatar artifacts and asynchronous signals affect different motion components supports explicit modality timing rather than one shared frame lock. [P12] [P13] [P31] [P35]

### TA-005 — Optimize worst-case stability before maximum nominal FPS

A provider or quality mode shall be rejected when it achieves a high average inference rate but causes recurrent p95/p99 stalls that starve the output loop. Real-time inference architectures document first-run and runtime violations and motivate decoupling model execution from the time-critical output callback. [P58] [P59]

## 1.3 Non-goals

- The system is not a medical, ergonomic, or biomechanical measurement instrument.
- It does not promise millimeter-level world-coordinate accuracy.
- Version 1 tracks one active performer, not a crowd.
- It does not infer fully hidden fingers with certainty.
- It does not guarantee 360-degree performance from one camera.
- It does not require the same person who calibrated the site to perform.
- It does not depend on a cloud service or subscription.
- It does not scrape or train on videos merely because they are publicly viewable; usage rights remain mandatory.

---

# 2. End-to-end architecture

## 2.1 Runtime dataflow

```text
                         ┌───────────────────────────────────┐
CAMERA 0 ─┐              │          SITE STATE               │
CAMERA 1 ─┼─> CAPTURE ───┤ intrinsics / extrinsics / floor  │
CAMERA N ─┘    BROKER     │ distortion / timing / quality   │
                         └─────────────────┬─────────────────┘
                                           │
                      timestamped FramePacket streams
                                           │
                   ┌───────────────────────┴──────────────────────┐
                   │                                              │
          PER-VIEW FAST PATH                              SCHEDULED HEAVY PATH
       ROI / mask / 2D whole body                      spatial body HMR / features
       face box / hand boxes / prop                    selected views only
                   │                                              │
                   └───────────────────────┬──────────────────────┘
                                           │ ViewObservation set
                                           v
                           ROBUST GEOMETRY + SET FUSION
                         triangulation / view weighting / uncertainty
                                           │
                         HMD/controllers/audio/optional depth
                                           │
                                           v
                             CAUSAL TEMPORAL STATE ESTIMATOR
                   pose / root / velocity / contacts / confidence / prop
                                           │
                                           v
                         CONTACT, FLOOR, AND FAILURE-CONTAINMENT SOLVER
                                           │
                                           v
                             AVATAR-AWARE RETARGETING AND IK
                                           │
                ┌──────────────────────────┼─────────────────────────┐
                v                          v                         v
             VRM/VMC                VRChat/SteamVR              Preview/record
```

Face, gaze, lips, hands, and props are separate branches that join at the temporal and retarget layers. They share timestamps and body context but do not block one another.

### TA-006 — Implement a streaming directed acyclic graph with timestamped messages

Subsystems shall communicate through immutable, timestamped packets rather than direct synchronous calls. This permits independent rates, stale-result rejection, deterministic replay, and provider-specific concurrency. Streaming online HMR and real-time inference architectures motivate incremental state, cached computation, and decoupled inference threads. [P01] [P58]

### TA-007 — Keep perception, state estimation, and avatar synthesis separate

The architecture shall distinguish: `(a)` what cameras observed, `(b)` the estimated human performance, and `(c)` the pose that best animates the selected avatar. This prevents avatar proportions from contaminating camera geometry and permits visual cleanup without pretending it is physical measurement. Motion-retargeting and hybrid IK work support explicit mapping from estimated human motion to target-skeleton constraints. [P36] [P37] [P38] [P39]

## 2.2 Process and thread topology

Recommended native runtime: one process for the shipping application, with crash-isolated helper processes optional for camera drivers or experimental providers.

```text
UI / control thread
    ├── capture thread per camera
    ├── audio capture thread
    ├── preprocessing worker pool
    ├── inference scheduler
    │      ├── GPU submission worker(s)
    │      └── CPU fallback worker(s)
    ├── calibration / geometry worker
    ├── fusion + temporal state thread
    ├── 60/90 Hz output thread
    ├── protocol adapters
    └── recording / deterministic replay writer
```

### TA-008 — Give every camera an independent capture thread and bounded queue

USB webcams are not genlocked and may block or drop frames independently. Each camera shall have its own capture loop, monotonic timestamps, and a small bounded queue. Downstream inference consumes the newest useful frame instead of allowing old frames to accumulate. Kineo explicitly addresses unsynchronized consumer cameras; the self-avatar artifact literature shows that desynchronization degrades pose reconstruction. [P15] [P16] [P12]

### TA-009 — Use latest-useful-frame semantics for realtime paths

When inference falls behind, the realtime path shall discard superseded frames rather than preserve throughput at the cost of growing latency. Recording mode may preserve all frames separately. This decision follows real-time inference work that separates continuous callbacks from variable-duration inference and manages latency violations. [P58]

### TA-010 — Preserve deterministic replay inputs

The recorder shall store original timestamps, selected frames or compressed source video, calibration state, provider choice, model hashes, scheduler decisions, and observations. A deterministic replay harness is required to reproduce glitches and compare model revisions. Temporal HMR research depends on consistent sequence evaluation; a reproducible stream is also necessary to separate capture, inference, fusion, and retarget failures. [P01] [P04] [P10]

---

# 3. State domains and data contracts

## 3.1 Persistent and ephemeral state

| State domain | Lifetime | Examples | Identity policy |
|---|---|---|---|
| Site State | Until cameras move/change | camera intrinsics/extrinsics, distortion, time offset, floor, capture volume, quality history | Must be person-agnostic |
| Avatar Profile | Until avatar asset changes | hierarchy, rest transforms, bone axes, limits, foot geometry, expressions, tracker roles | Belongs to asset, not performer |
| Performer State | Current guest/session | estimated height ratios, limb proportions, neutral face, skin-independent hand scale, active track ID | Ephemeral; no gradient updates |
| Temporal State | Seconds | pose history, velocity, contacts, uncertainty, hidden-limb continuation, prop state | Destroy on handoff/restart |
| Runtime State | Process | provider sessions, memory pools, compiled engines, benchmark cache | Contains no performer identity |

### TA-011 — Hard-separate site, avatar, performer, temporal, and runtime state

No calibration output may silently include a body-shape code or appearance embedding from the calibration person. Explicit state separation is required to make cross-person site reuse testable. Human-based calibration research estimates camera parameters from moving people, while parametric-body work separates skeletal/surface parameters from scene geometry. [P15] [P16] [P18] [P40]

### TA-012 — Forbid per-guest weight updates

Guest adaptation shall estimate low-dimensional ephemeral parameters through forward inference or bounded optimization only. It shall not fine-tune neural weights. Motion models such as HuMoR generalize across diverse motions and body shapes, and SAM 3D Body separates rig and surface representation; those capabilities support session-level inference rather than identity-specific training. [P02] [P07]

## 3.2 Coordinate frames

The system uses explicit coordinate frames:

- `C_i`: camera `i` optical frame.
- `W`: persistent site/world frame; `+Y` up, floor at approximately `Y=0`.
- `H`: canonical performer frame centered at the pelvis in neutral stance.
- `B`: canonical body-model frame used by the spatial teacher/student.
- `A`: avatar rest-skeleton frame.
- `V`: VR tracking universe when VRChat/SteamVR mode is active.

Every transform is named by source and destination, e.g. `T_W_Ci` maps camera coordinates into site coordinates. Units are meters internally.

### TA-013 — Make world, performer, body-model, avatar, and VR frames explicit

The implementation shall never infer coordinate meaning from tensor position or module context. World-grounded HMR work demonstrates that camera-relative pose alone is insufficient for global movement, while retargeting research requires explicit source/target geometry. [P01] [P05] [P06] [P36]

## 3.3 Core packets

```cpp
struct FramePacket {
    CameraId camera_id;
    uint64_t sequence;
    TimePoint exposure_midpoint;
    TimePoint host_arrival;
    ImageView image;
    CameraMode delivered_mode;
    float exposure_ms;
    float estimated_blur;
    bool dropped_predecessor;
};

struct Joint2DObservation {
    Vec2 pixel;
    Mat2 covariance;
    float detector_confidence;
    float visibility_probability;
    float occlusion_probability;
};

struct ViewObservation {
    CameraId camera_id;
    TimePoint source_time;
    Roi person_roi;
    MaskHandle silhouette;
    std::array<Joint2DObservation, J> body_2d;
    Optional<SpatialBodyHypothesis> body_3d_camera;
    Optional<FaceObservation> face;
    Optional<HandObservation> left_hand;
    Optional<HandObservation> right_hand;
    Optional<PropObservation> prop;
    ViewQuality quality;
};

struct FusedHumanObservation {
    TimePoint observation_time;
    CanonicalPose pose;
    RootObservation root;
    ContactProbabilities contacts;
    std::array<JointUncertainty, J> uncertainty;
    VisibilityGraph visibility;
};

struct AvatarOutputState {
    TimePoint target_time;
    RootTransform root;
    BoneRotations bones;
    ExpressionChannels expressions;
    TrackerTargets virtual_trackers;
    OutputConfidence confidence;
};
```

### TA-014 — Carry covariance, visibility, source time, and age through every stage

A scalar confidence alone cannot represent anisotropic uncertainty, stale observations, or ray-direction ambiguity. Probabilistic triangulation and uncertainty-aware multiview pose estimation support carrying distributions rather than only point estimates. [P22] [P23]

---

# 4. Capture, timing, and low-quality webcam handling

## 4.1 Platform capture layer

Implementation mapping:

- Windows: Media Foundation first; optional DirectShow compatibility path for problematic cameras.
- Linux: V4L2 for direct capture; GStreamer optional for format conversion/network sources.
- Audio: WASAPI on Windows and PipeWire/PulseAudio/ALSA abstraction on Linux.
- All platform timestamps are converted immediately to one process-wide monotonic timebase.

These API selections are implementation details. The paper-backed decision is to treat cameras as asynchronous sensors and align them in software. [P15] [P16]

### TA-015 — Timestamp as close to exposure as the driver permits

The capture layer shall prefer hardware or driver presentation timestamps and estimate exposure midpoint. Host dequeue time is retained separately. Calibration and fusion use source time; profiler views both source and arrival latency. Unsynchronized-camera research shows temporal offset must be estimated rather than assumed. [P15] [P16]

### TA-016 — Estimate and continuously refine inter-camera temporal offsets

Initial site calibration shall estimate offsets from synchronized motion energy, keypoint trajectories, and audio where available. During tracking, a slow background estimator may refine offsets without changing the visible pose abruptly. Kineo uses audio synchronization, while spatiotemporal calibration work estimates time offsets from freely moving people. [P15] [P16]

### TA-017 — Preserve native-resolution frames for crops while running body detection on resized tensors

The full-body detector may operate at 384–768-pixel model input, but face and hand crops shall be extracted from the original frame. High-fidelity face and hand work relies on dedicated high-resolution regions; scaling a whole person to a small input discards finger and expression information. [P29] [P30] [P31] [P32]

## 4.2 Camera health estimation

Per camera, compute:

- delivered frame rate and jitter;
- exposure duration estimate;
- rolling blur score;
- under/overexposure fraction;
- effective person pixel height;
- foot, hand, and face visibility history;
- compression/blocking score;
- reprojection residual after calibration;
- duplicate-frame rate;
- end-to-end arrival delay.

### TA-018 — Make camera quality a model input and scheduler signal

A camera is not globally “good” or “bad”; it may be good for feet and poor for hands, or become stale under USB contention. View-quality features shall affect per-joint fusion weights and heavy-view scheduling. MUC learns body-part-specific camera weights, and uncertainty-aware multiview methods explicitly model observation quality. [P19] [P23]

### TA-019 — Train with webcam corruption rather than rely on clean-render transfer

Synthetic training shall randomize exposure, blur, compression, white balance, noise, resolution, lens distortion, rolling-shutter-like warping, frame duplication, drops, and asynchronous offsets. Domain randomization and synthetic human datasets show that broad randomized rendering can improve real-world transfer. [P45] [P47] [P52]

## 4.3 Capture presets

The application requests the highest useful mode that does not destabilize USB delivery. A 1080p30 camera can be preferable to nominal 1080p60 that drops frames; a dedicated hand camera may use 720p60 with short exposure.

Suggested initial policies:

| Use | Requested mode | Processing |
|---|---|---|
| General body camera | 720p60 or 1080p30 | Body detector on resized image; native crops retained |
| Two-camera Quality | Prefer matched 720p60 where reliable | Stagger spatial inference; fuse by timestamp |
| Hand/instrument view | 720p60 or 1080p60 if delivered consistently | Narrow ROI and high shutter priority |
| Weak hardware Eco | 720p30 | 12–15 Hz heavy model; 30/60 Hz lightweight tracking |

### TA-020 — Prefer predictable frame delivery over nominal resolution/FPS

The setup wizard shall benchmark actual delivery and select a stable mode rather than trust camera-advertised maxima. Avatar artifact research shows rate mismatch and latency harm pose reconstruction; a stable, timestamped 30 Hz stream is more useful than bursty 60 Hz input. [P12] [P58]


# 5. Person-agnostic site calibration

The user-facing calibration consists of ordinary body motions. Internally, it is a spatiotemporal camera-calibration and bundle-adjustment problem. “No fancy calibration” means the software hides the geometry; it does not mean geometry is unnecessary.

## 5.1 Guided sequence

A recommended first-site sequence is 12–25 seconds:

1. Neutral stance near the center.
2. A-pose or T-pose.
3. Quarter-turn left and right.
4. Raise each arm independently.
5. March or take one step forward/back.
6. Shallow crouch.
7. Optional slow rotation when three or more cameras are used.
8. Optional guitar/instrument hold for a prop-enabled site profile.

The wizard may stop early when observability thresholds are met. It should show only simple instructions and a placement score.

### TA-021 — Use the human performer as the primary calibration target

The site solver shall estimate camera parameters from articulated human correspondences produced during guided motion, eliminating checkerboards for normal users. Research on calibration-free metric capture, calibration from freely moving people, and humans-as-calibration demonstrates that human motion can constrain intrinsics, extrinsics, timing, and scale. [P15] [P16] [P17] [P18]

### TA-022 — Train and evaluate calibration across different identities

Synthetic and real calibration tests shall use identity A for site calibration and identity B for tracking. This is a required data split, not merely an evaluation variation. It prevents the calibrator from learning that a body-shape signature is part of the camera solution. [P15] [P18] [P40]

## 5.2 Calibration variables

For each camera `i`, estimate:

- rotation `R_W_Ci` and translation `t_W_Ci`;
- focal lengths `f_x, f_y` and principal point `c_x, c_y`;
- Brown–Conrady radial/tangential distortion coefficients;
- stream time offset `Δt_i` and optionally slow clock drift;
- delivered crop/scale transform from sensor mode to application frame;
- confidence and covariance for the solved parameters.

Global/site variables:

- floor plane and gravity/up direction;
- metric scale;
- usable capture-volume bounds;
- camera overlap graph;
- per-pair expected triangulation quality;
- static-scene feature map when available.

### TA-023 — Solve intrinsics, extrinsics, distortion, timing, and scale jointly enough to expose uncertainty

The site system shall not assume nominal webcam focal length or perfect synchronization. Kineo jointly recovers intrinsics, extrinsics, Brown–Conrady distortion, timing, and metric scale from sparse RGB cameras; related work estimates spatiotemporal calibration from human motion. [P15] [P16]

## 5.3 Calibration pipeline

### Step A — collect robust 2D tracks

Run the fast whole-body detector on all cameras. Keep only joints with high visibility, low covariance, and sufficient motion. Reject nearly static intervals for time-offset estimation.

### Step B — coarse temporal alignment

Compute velocity/acceleration signatures for torso, wrists, ankles, and global silhouette. Cross-correlate signatures across camera pairs; use audio transients as an additional cue when microphones share the performance sound.

### Step C — pairwise geometry hypotheses

Generate pairwise relative-camera hypotheses from 2D correspondences and monocular body priors. Use RANSAC/robust losses to suppress detector outliers and self-occluded joints.

### Step D — global camera graph

Build a camera-overlap graph whose edge weight reflects shared visibility, baseline observability, residual, and temporal confidence. Choose a stable root camera, initialize the global graph, then jointly optimize.

### Step E — articulated bundle adjustment

Optimize camera parameters and a sequence of latent human poses against 2D reprojection error, bone-length consistency, motion smoothness, gravity/floor constraints, and body-scale priors. Body shape is a nuisance variable and is discarded after solving site geometry.

### Step F — metric scale and floor

Recover scale from a population body prior, an optional user-entered approximate height, known avatar-independent calibration movement, or scene depth where available. Estimate the floor from ankle/foot contacts and/or static-scene geometry.

### Step G — validation with held-out motion

Use the last seconds of the guided sequence as validation frames. Report reprojection residual, triangulation angle, temporal alignment quality, floor consistency, and expected visibility coverage.

### TA-024 — Use confidence-driven robust graph optimization for calibration

Calibration shall choose high-confidence correspondences and optimize a connected camera graph rather than trust every detected joint equally. Kineo reports confidence-driven sampling and graph-based optimization for uncalibrated consumer cameras. [P15]

### TA-025 — Include an articulated body prior but discard performer-specific calibration variables

The optimizer may use a parametric body or fixed-bone articulated model to stabilize geometry, especially scale, but body shape and appearance are nuisance variables. Human-based calibration and SmartMocap-style joint camera/motion reasoning support using the human as a geometric prior. [P18] [P61]

### TA-026 — Treat floor and gravity as persistent site state

The floor plane shall be estimated during calibration and refined only slowly. World-grounded HMR and contact-aware trajectory refinement show that gravity/floor/contact context is critical to stable root motion and foot placement. [P05] [P06] [P08]

## 5.4 Placement scoring

The setup wizard computes a quality score, not a strict geometry requirement.

Per camera-pair score:

```text
pair_score =
    overlap_quality
  × expected_joint_visibility
  × triangulation_baseline_quality
  × temporal_delivery_quality
  × image_quality
```

Per-joint expected coverage is estimated from the calibration sequence. The UI gives actionable text:

- “Camera 2 is too similar to Camera 1; move it farther sideways.”
- “Both cameras lose the feet during a crouch.”
- “Camera 3 is useful for the guitar neck but not the lower body.”
- “This camera is delivering duplicate frames; try another USB controller.”

### TA-027 — Recommend complementary views rather than fixed measured placements

The system shall score actual observed geometry and visibility instead of requiring users to measure camera angles. Learnable triangulation and uncalibrated multiview reconstruction show that view geometry determines depth quality, while set-based fusion allows arbitrary camera arrangements. [P20] [P21] [P24]

## 5.5 Continuous calibration health

During runtime, compare each view’s 2D observations with projections of the fused 3D state. Maintain exponentially weighted residual statistics. A camera enters:

- `HEALTHY`: normal fusion.
- `SUSPECT`: weight reduced; background refinement allowed.
- `QUARANTINED`: excluded from root translation and geometric triangulation; may still provide local face/hand crops.
- `RECALIBRATION_REQUIRED`: user receives a clear prompt.

### TA-028 — Detect moved cameras from persistent reprojection residual, not from one bad frame

Camera health shall use temporal residual evidence and quality metadata. Probabilistic triangulation and uncertainty-aware multiview methods motivate residual-weighted confidence rather than binary trust. [P22] [P23]

### TA-029 — Never silently rewrite site geometry during a performance

Online refinement may make small bounded updates, but a large geometry change requires quarantine and explicit recovery. This prevents calibration drift from appearing as performer translation. Online world-alignment research emphasizes physically plausible incremental correction rather than arbitrary global jumps. [P01]

---

# 6. Guest acquisition and performer-state estimation

## 6.1 Handoff state machine

```text
EMPTY
  │ person enters
  v
ACQUIRING ── insufficient coverage ──> GUIDANCE
  │ stable active person
  v
GENERIC_TRACKING
  │ optional 3–8 s quality fit
  v
QUALITY_TRACKING
  │ performer leaves / manual handoff / timeout
  v
CLEAR_TEMPORAL_AND_PERFORMER_STATE
  │
  └──────────────────────────────────> EMPTY
```

### TA-030 — Support instant guest and quality guest modes without retraining

A new person shall work immediately with a generic body prior, while a brief neutral/A-pose fit may estimate session proportions and face normalization. No camera recalibration or neural fine-tuning is required. Parametric body models and HMR methods support estimating pose/shape variables at inference time. [P02] [P40]

### TA-031 — Clear all performer and temporal state on handoff

Bone ratios, face normalization, hidden-limb state, contact flags, velocities, motion mode, and active appearance embeddings shall be destroyed when the active track changes. This is an architectural enforcement of person agnosticism; sequence models otherwise carry history that can bias a new performer. [P01] [P07]

## 6.2 Active-person association

Version 1 supports one active performer. Association uses:

- spatial continuity in site coordinates;
- torso and limb keypoint consistency;
- short-lived appearance features when necessary;
- user-selected capture volume;
- optional “step into frame” gesture.

Appearance embeddings are held only in memory and deleted at handoff.

### TA-032 — Make identity continuity ephemeral and subordinate to geometry

The tracker may use a short-lived appearance cue to avoid switching between people, but active-person association shall prioritize 3D location and motion. The product does not need face recognition or persistent identity. Multiview tracking/calibration research shows that geometry and pose correspondences are sufficient to associate observations in bounded scenes. [P15] [P16]

## 6.3 Ephemeral anthropometry

Estimate a low-dimensional performer code:

- total height scale;
- shoulder-to-hip ratio;
- upper/lower arm ratios;
- femur/tibia ratios;
- torso length;
- hand scale;
- neutral shoulder and hip widths;
- optional mobility/joint-range hints.

Update rapidly during the first seconds, then freeze or adapt only with a very slow robust filter.

### TA-033 — Lock performer bone proportions after initial convergence

Frame-wise body-shape fluctuation creates visible “breathing,” root-depth changes, and unstable retargeting. SAM 3D Body’s MHR representation separates skeletal structure and surface shape, while temporal pose research treats shape as sequence-consistent. [P02] [P04] [P10]

### TA-034 — Represent predicted joint rotations with a continuous 6D representation inside neural networks

Neural modules shall use a continuous rotation representation such as 6D and convert to orthonormal matrices/quaternions at boundaries. Rotation-representation research shows common low-dimensional representations have discontinuities that complicate learning. [P41]

---

# 7. Avatar import and Avatar Profile generation

## 7.1 Supported asset path

Initial support priority:

1. VRM 1.0 / VRM 0.x humanoid avatars.
2. GLB/GLTF humanoid rigs with mapping UI.
3. FBX import through an offline converter or compatible asset pipeline.
4. VRChat mode through a generated virtual-tracker profile when direct avatar bone control is unavailable.

The asset and output mappings follow the current VRM, VMC, and VRChat OSC specifications. [O07] [O08] [O09]

The application creates an `AvatarProfile` independent of performer identity.

## 7.2 Avatar Profile contents

```text
asset hash and format
humanoid hierarchy and semantic bone map
rest local/global transforms
bone axes and handedness
bone lengths and avatar height
joint limits and preferred bend planes
foot sole geometry and toe direction
hand hierarchy and finger axes
head/eye axes
gaze and expression/blendshape map
contact points: soles, toes, palms, fingertips
virtual tracker roles and offsets
retarget scale policies
preview validation results
```

### TA-035 — Normalize every avatar into a canonical semantic humanoid schema

Perception shall output a canonical human performance; asset-specific axes and topology belong in an Avatar Profile. Skeleton-aware retargeting supports topology-aware mapping across different skeletons, and SMPL-X provides a unified body/hand/face canonical representation for training. [P36] [P40]

### TA-036 — Validate rigs using canonical motion probes

Import shall automatically test reach, squat, sit, step, torso twist, wrist orientation, hand closure, gaze, and guitar-hold poses. Failures such as reversed axes, bad bend planes, and foot orientation must be reported before live tracking. Retargeting research explicitly evaluates motion semantics, interpenetration, and contact preservation across different geometries. [P36] [P39]

## 7.3 Automatic avatar scaling and fit

The solver does not scale the human into the avatar by one global height ratio. It derives semantic ratios:

- pelvis height and leg reach;
- arm reach relative to torso;
- shoulder width and clavicle behavior;
- head size and neck length;
- foot size and floor offset;
- hand size and prop contact span.

A chibi avatar may require reduced root travel, exaggerated knee flex, and altered arm reach to preserve the readable meaning of a movement.

### TA-037 — Optimize motion semantics and contacts, not centimeter correspondence

Avatar retargeting shall preserve direction, phase, end-effector intent, support contacts, balance, and joint plausibility under different proportions. R2ET and skeleton-aware retargeting show that shape- and skeleton-aware corrections are needed to preserve semantics and avoid contact/interpenetration failures. [P36] [P39]

### TA-038 — Use a two-stage retargeter

Stage 1 transfers normalized semantic rotations and root intent into the target hierarchy. Stage 2 solves constrained IK for feet, hands, head, prop contacts, balance, and joint limits. Hybrid analytical-neural IK work supports combining predicted structure with analytical kinematic constraints. [P37] [P38]

## 7.4 Avatar-specific learned correction

A compact retarget network may consume:

- canonical pose and velocities;
- avatar bone-length ratios;
- avatar rest-axis embeddings;
- current contacts and prop constraints;
- target joint limits;
- previous avatar pose.

It outputs residual joint rotations, root-scale correction, and confidence. This model is trained across procedurally generated avatar proportions and rig perturbations, not fine-tuned per user.

### TA-039 — Condition retargeting on avatar geometry while keeping weights shared

The retarget network shall generalize to unseen avatars by receiving explicit skeleton/shape descriptors rather than learning one model per avatar. Skeleton-aware networks and residual retargeting support shared models conditioned on source/target geometry. [P36] [P39]

---

# 8. Per-camera perception stack

The per-camera stack is deliberately modular. A single huge full-body model is not allowed to dictate the update rate of face, hands, or additional views.

## 8.1 Fast per-frame path

For every delivered frame:

1. Detect or propagate the active-person ROI.
2. Estimate whole-body 2D keypoints, visibility, and covariance.
3. Estimate a silhouette/person mask when budget permits.
4. Locate face and hand crops.
5. Locate known props/instruments in enabled modes.
6. Compute view quality and motion features.
7. Produce intermediate features only if the selected backend and memory budget permit reuse.

### TA-040 — Run a lightweight whole-body 2D observer on every useful camera frame

All cameras shall provide inexpensive geometric evidence at or near delivered frame rate. RTMPose, DWPose, and RTMO demonstrate practical realtime whole-body/keypoint architectures that can serve as compact observers or distillation teachers. [P26] [P27] [P28]

### TA-041 — Predict visibility and covariance, not only keypoint heatmap maxima

The observer shall output calibrated visibility/occlusion probabilities and uncertainty. These quantities are required for triangulation, view weighting, and failure containment. Probabilistic triangulation and uncertainty-aware pose estimation show the benefit of propagating uncertainty through multiview reconstruction. [P22] [P23]

## 8.2 Scheduled spatial body path

A stronger model processes selected views at a lower hardware-dependent rate. It outputs:

- canonical full-body pose/mesh hypothesis in camera coordinates;
- body/hand/foot joint rotations;
- camera-relative root depth and orientation;
- shape/rig estimate before session locking;
- spatial feature tokens for multiview fusion;
- confidence per body region;
- optional 2D reprojection and mask consistency.

SAM 3D Body is the preferred high-quality teacher because it targets body, feet, and hands and accepts keypoint/mask prompts. The shipping spatial student should be substantially smaller. [P02] [P03]

### TA-042 — Use SAM-class HMR as an offline teacher, not a mandatory per-camera 60 Hz runtime dependency

The project shall first exploit a strong pretrained HMR teacher for labels and evaluation, then distill a narrower one-person fixed-camera student. Fast SAM 3D Body demonstrates major speedups but remains expensive enough that multiplying it across cameras and 60 Hz would be impractical on ordinary hardware. [P02] [P03] [P51]

### TA-043 — Prompt the spatial model with the fast path’s keypoints and mask when supported

Keypoints and segmentation reduce crop ambiguity and let the expensive model focus on the active performer. SAM 3D Body explicitly supports 2D-keypoint and mask prompts. [P02]

### TA-044 — Schedule heavy HMR on the most informative views rather than all views every tick

Every camera contributes 2D evidence; only `K` cameras—normally one or two—run heavy spatial HMR at a given instant. The scheduler chooses views by coverage, per-joint confidence, triangulation baseline, image quality, and staleness. MUC’s joint-specific camera quality and set-based multiview models support view-dependent fusion rather than equal processing. [P19] [P20] [P21]

## 8.3 Shared feature policy

Intermediate image features can improve learned multiview fusion, but cross-provider tensor interop is fragile. The baseline product therefore defines two tiers:

- **Portable tier:** exchange only standardized tensors—2D keypoints, masks, compact spatial tokens, and hypotheses.
- **Accelerated tier:** reuse device-resident feature maps when the provider supports safe zero-copy interop.

### TA-045 — Keep the required model interface tensor-portable

The core topology shall not depend on vendor-specific feature handles. Portable ONNX tensors permit Windows/Linux and GPU-provider substitution, while optional zero-copy paths may optimize known providers. Cross-platform runtime research and ONNX conversion studies support a stable interchange representation but also require provider-specific validation. [P56] [P57] [P60]


# 9. Multi-camera geometry and learned fusion

## 9.1 Why fusion occurs after per-view perception but before temporal output

Pure early feature fusion tightly couples the network to a camera count, ordering, calibration convention, and backend. Pure late averaging discards geometric structure and can merge incompatible hypotheses. The proposed fusion uses explicit geometry first, then a learned residual set model.

### TA-046 — Combine robust triangulation with learned residual fusion

The baseline fused pose shall be produced from confidence-weighted multiview geometry; a learned set model then resolves soft-tissue/body-model ambiguity, imperfect calibration, occlusion, and conflicting monocular hypotheses. Learnable/probabilistic triangulation and uncalibrated HMR support this hybrid design. [P19] [P22] [P24] [P25]

## 9.2 Observation normalization

For each joint `j` in camera `i`, create a token:

```text
z_ij = [
    ray_origin_W,
    ray_direction_W,
    normalized_pixel_xy,
    2D_covariance,
    detector_confidence,
    visibility_probability,
    occlusion_probability,
    camera_quality,
    source_age_ms,
    monocular_3D_joint_W,
    monocular_joint_rotation_6D,
    monocular_region_confidence,
    compact_image_feature,
    camera_embedding,
    joint_semantic_embedding
]
```

Missing fields are masked, not filled with confident zeros.

### TA-047 — Represent each view as rays and observations in a shared world frame

The fusion model shall receive calibrated ray geometry explicitly rather than learn camera projection from scratch for every site. Epipolar/triangulation methods demonstrate that camera geometry provides a strong inductive bias and reduces data requirements. [P24] [P25]

### TA-048 — Include source age and timing offset in fusion tokens

A geometrically correct but stale wrist observation can be worse than a current noisy one during a strum. The model shall know observation age and temporal alignment confidence. Research on unsynchronized multiview capture and self-avatar desynchronization supports time-aware fusion. [P15] [P16] [P12]

## 9.3 Robust geometric layer

For a joint with observations `u_ij`, camera projection `π_i`, covariance `Σ_ij`, and candidate 3D point `X_j`, solve:

```text
X*_j = argmin_X Σ_i ρ( (u_ij - π_i(X))ᵀ Σ_ij⁻¹ (u_ij - π_i(X)) )
```

where `ρ` is a robust loss. Output point covariance from local curvature, pairwise triangulation spread, and learned calibration error.

For rotations and full-body shape, transform per-camera hypotheses into `W` and compute robust region-aware initial estimates.

### TA-049 — Use anisotropic confidence-weighted triangulation with robust loss

2D uncertainty is often directional and camera-dependent. The geometric layer shall account for covariance and reject outlier views rather than average rays. Probabilistic triangulation explicitly models uncertainty in multiview 3D pose. [P22]

### TA-050 — Degrade gracefully to one-camera hypotheses

If fewer than two trustworthy rays exist for a joint, the geometric layer shall emit an uncertain monocular hypothesis instead of failing the entire body. U-HMR and MUC are designed around arbitrary camera counts or uncalibrated single-view encoders, supporting one-to-many continuity. [P19] [P20]

## 9.4 Set-based learned fusion

Recommended architecture:

1. Per-joint token projection to `d=128–256`.
2. Two to four set-attention blocks across camera observations for each joint.
3. Cross-joint kinematic attention or graph layers over the body hierarchy.
4. Optional whole-view tokens carrying global camera quality and monocular body features.
5. Heads for fused 3D position, 6D rotation, uncertainty, visibility consensus, and view weights.
6. Residual output around the robust geometric estimate.

The same weights accept 1–N cameras. Camera ordering is randomized during training.

### TA-051 — Make camera fusion permutation invariant and camera-count agnostic

The fusion network shall consume an unordered masked set of view observations. Set Transformer provides an attention-based permutation-invariant architecture, while U-HMR demonstrates arbitrary camera counts/poses. [P20] [P21]

### TA-052 — Learn per-joint and per-region camera weights

The network shall not assign one global best camera. Feet, hands, face, and torso can favor different views. MUC specifically improves uncalibrated reconstruction through joint and surface reweighting according to view quality. [P19]

### TA-053 — Predict calibrated fused uncertainty

Every fused joint and region shall output covariance or a distribution parameter trained with a proper likelihood and calibration loss. The temporal model uses this uncertainty to determine correction strength. [P22] [P23]

## 9.5 Camera-count scaling

Approximate cost:

```text
C_total(N) ≈ N × C_fast2D
           + K × C_spatialHMR
           + C_fusion(N)
           + C_face/hand(selected crops)
```

`K` is normally `1` or `2`, not `N`. Set attention cost is small because the number of cameras is small and tokens are compact.

### TA-054 — Make compute scale sublinearly with camera count

Additional cameras shall always run lightweight evidence extraction, while expensive spatial HMR is limited to a scheduler-selected subset. Multi-view fusion research shows that independently reconstructed/view-weighted evidence can be combined without requiring all cameras to run identical high-capacity models. [P19] [P20]

### TA-055 — Stagger heavy observations across cameras

For two cameras at 20 Hz each, schedule A and B approximately half a period apart, yielding a strong spatial observation roughly every 25 ms even though each view updates every 50 ms. The temporal model aligns observations by source timestamp. Online cached inference and asynchronous sensor research support incremental processing of irregular observations. [P01] [P12]

## 9.6 View selection score

A scheduler score for camera `i` and region `r` can be:

```text
S(i,r) =
    w1 × visibility(i,r)
  + w2 × sharpness(i,r)
  + w3 × complementary_baseline(i,r)
  + w4 × novelty_since_last_heavy(i)
  + w5 × historical_accuracy(i,r)
  - w6 × source_age(i)
  - w7 × reprojection_residual(i)
  - w8 × expected_runtime(i,provider)
```

For body HMR, aggregate region scores with extra weight on currently uncertain regions.

### TA-056 — Select heavy views according to expected information gain, not round-robin alone

The scheduler shall prefer views that can reduce current uncertainty and complement recent observations. Joint-specific view-quality work and uncertainty-aware fusion motivate confidence-driven selection. [P19] [P23]

---

# 10. Causal temporal state estimator

The temporal subsystem is the core “does not randomly glitch” layer. It receives asynchronous fused observations and emits a pose at any requested output timestamp.

## 10.1 State vector

A practical latent state includes:

```text
root position and orientation in W
root linear/angular velocity and acceleration
canonical joint rotations and angular velocities
selected joint positions and velocities
left/right foot contact probabilities
left/right toe/heel subcontacts
seated/support contact state
left/right hand contact state
face expression latent and gaze
left/right hand pose latents
prop pose and contact graph
per-joint uncertainty
motion-mode latent: idle / locomotion / crouch / sit / dance / instrument
camera-consensus indicators
```

### TA-057 — Estimate motion and contact state jointly with pose

Pose alone is insufficient to distinguish a planted foot from a slowly moving foot or a supported hip from free motion. WHAM, HuMoR, and contact-aware world reconstruction show that motion and contact variables improve plausible trajectory estimation. [P05] [P07] [P08]

## 10.2 Recommended model topology

A compact causal transformer or state-space hybrid:

1. Observation encoder maps fused measurements, covariance, visibility, source age, and modality tags into tokens.
2. Causal temporal backbone keeps a bounded cached history, approximately 0.5–2 seconds depending on model tier.
3. State heads predict current pose, velocities, contacts, uncertainty, and a short forward horizon.
4. A deterministic constraint layer applies joint limits, fixed bone lengths, floor/contact constraints, and innovation gating.
5. Output predictor advances the state to the requested 60/90 Hz timestamp.

The temporal model must support irregular observation intervals and missing modalities.

### TA-058 — Use a causal cached temporal model with bounded history

The realtime temporal model shall consume only past and current observations and reuse cached keys/state rather than reprocess the full sequence. OnlineHMR explicitly uses causal key–value caching and sliding-window learning for streaming inference. [P01]

### TA-059 — Train on irregular, asynchronous, and missing observations

Training shall randomly drop views/modalities, vary observation intervals, inject source-age offsets, and change camera rate. This directly targets the desynchronization and occlusion artifacts documented in avatar reconstruction research. [P12] [P23]

### TA-060 — Predict a short future horizon but expose only causally available predictions

The model may predict 50–150 ms of future state for smooth high-rate output and collision/contact anticipation, but live output uses only predictions derived from current/past evidence. Causal motion generation/reconstruction work supports short-horizon predictive modeling without future input. [P09] [P07]

## 10.3 Observation update and innovation gating

Let predicted state be `x^-_t`, observation-derived state be `z_t`, and learned/analytic innovation gate be `G_t`:

```text
innovation = wrap_pose(z_t - H(x^-_t))
accepted   = clamp_by_uncertainty_and_anatomy(innovation)
x_t        = x^-_t + G_t ⊙ accepted
```

Large innovations require stronger multiview consensus or persistent evidence. The root has stricter limits than a visible wrist.

### TA-061 — Bound correction magnitude by uncertainty, joint semantics, and elapsed time

New observations shall not snap directly into the output state. Corrections are confidence-weighted and bounded, with faster recovery for clearly visible distal joints and stricter gating for root translation. Probabilistic pose and robust motion-prior work support uncertainty-aware correction of noisy/occluded observations. [P07] [P22] [P23]

### TA-062 — Use hysteresis for discrete contact and motion modes

Contact, seated, and support states shall require entry/exit evidence over several frames or a high-confidence event. This prevents flickering feet and mode oscillation. Contact-aware world-motion methods rely on temporally coherent contact estimates rather than independent binary outputs. [P05] [P08]

## 10.4 Micro-smoothing

A lightweight adaptive filter may be applied after the learned state estimator to remove quantization or protocol noise, but it must not conceal structural errors.

### TA-063 — Restrict adaptive low-pass filtering to small residual jitter

Use a speed-adaptive filter such as the 1€ filter only for low-amplitude residuals and per-channel tuning. It is designed to balance jitter and lag in interactive systems; it is not a substitute for temporal reconstruction or contact reasoning. [P14]

## 10.5 Shared-occlusion behavior

When all cameras lose a joint:

1. Increase uncertainty according to time since last observation.
2. Continue state using motion prior, kinematic chain, prop/contact state, and known constraints.
3. Limit acceleration and joint-range drift.
4. Reduce high-frequency finger articulation before gross hand pose.
5. After a configurable horizon, blend toward a safe continuation—not a generic T-pose.
6. Reacquire with bounded correction when evidence returns.

### TA-064 — Make uncertainty grow during unobserved motion

The system shall become less certain as occlusion persists and communicate that uncertainty to retargeting and output. HuMoR is designed to recover plausible motion under noise and occlusion, while uncertainty-aware multiview work supports confidence propagation. [P07] [P23]

### TA-065 — Prefer constrained continuation over arbitrary hallucination

The motion prior may continue a trajectory briefly, but it may not invent large root displacement, a new contact, or an extreme joint configuration without evidence. Motion priors are valuable for ambiguity but remain probabilistic; failure containment must bound their influence. [P07] [P09]

---

# 11. Root translation, floor, contacts, and hallucination containment

This subsystem receives special treatment because root translation errors move the entire avatar and are more damaging than modest local joint error.

## 11.1 Root state hierarchy

Use evidence in this order:

1. Trusted VR HMD/controller anchors when in VR mode.
2. Two-or-more-view geometric root/torso observations.
3. Persistent floor/contact constraints and triangulated feet.
4. Monocular world-depth estimate conditioned on fixed performer scale.
5. Motion prior for short gaps.

A lower-ranked source cannot override a high-confidence higher-ranked source without sustained evidence.

### TA-066 — Treat root translation as a separately fused state

Root translation shall not be read directly from a single frame’s body-mesh root. World-grounded HMR research identifies global trajectory and foot sliding as distinct problems requiring motion and contact context. [P01] [P05] [P06]

### TA-067 — Prefer multiview geometry for depth whenever two trustworthy views exist

In two-camera and higher modes, pelvis, torso, and feet shall be triangulated and jointly optimized. Monocular root depth becomes a prior, not the primary measurement. Learnable/probabilistic triangulation provides the geometric basis. [P22] [P24]

### TA-068 — Lock monocular scale per performer session

One-camera depth changes shall be constrained by the session’s fixed anthropometry, floor, and observed body pixel scale. The body is not allowed to grow or shrink to explain every frame. Sequence-consistent shape and world-grounded reconstruction motivate this constraint. [P04] [P05] [P40]

## 11.2 Contact-aware root solve

At each output tick, solve a small constrained optimization:

```text
minimize over root Δr and selected joint residuals Δq:
    observation_error
  + λv × velocity_discontinuity
  + λa × acceleration_discontinuity
  + λc × planted_contact_velocity
  + λf × floor_penetration
  + λb × balance/support_error
  + λp × prior_deviation

subject to:
    fixed bone lengths
    joint limits
    bounded root innovation
    active contact constraints
```

This may use a few Gauss–Newton/projected-gradient iterations or an analytic IK layer with a learned initialization.

### TA-069 — Use contact as a corrective signal for trajectory, not only an output label

Foot and support contacts shall actively correct root and limb state. WHAM refines global trajectory using foot contact, and newer contact-aware reconstruction explicitly treats contact as feedback for world motion. [P05] [P08]

### TA-070 — Model toe, heel, whole-foot, hand, prop, seated, and support contacts separately

A guitar performance, crouch, tiptoe, and seated pose require different constraints. Binary “foot contact” is insufficient. Physics/motion work with contact-rich interactions supports structured contact state. [P07] [P42]

## 11.3 Translation safety rules

Hard runtime rules:

- A low-confidence monocular frame cannot change root depth by more than a velocity-bounded amount.
- A quarantined camera cannot contribute root translation.
- A new root trajectory must agree with either two views, one view plus strong floor/contact evidence, or trusted VR anchors.
- During total occlusion, root velocity decays unless ongoing movement is strongly predicted and consistent with last contacts.
- Reacquisition applies a correction ramp; no one-frame teleport.
- The system logs every root correction exceeding a review threshold.

### TA-071 — Require observation consensus for large root innovations

Large root corrections need multiview agreement or sustained evidence. This is a deliberate safety policy derived from probabilistic fusion and world-alignment research: uncertain spatial evidence should not dominate a globally visible avatar state. [P01] [P22] [P23]

## 11.4 VR-specific anchor fusion

In VRChat mode:

- HMD defines head position/orientation and a high-rate world anchor.
- Controllers define hands when held; camera hands may refine elbows/fingers or take over when controllers are absent.
- Cameras solve chest, hips, knees, feet, and visible limb geometry.
- A learned/analytic whole-body solver reconciles anchors without adding camera latency to HMD/controller motion.

### TA-072 — Pass trusted VR anchors through at native cadence and use cameras as asynchronous corrections

Do not delay HMD/controller motion to wait for cameras. Self-avatar research documents degradation from asynchronous sensor fusion, so the temporal model must explicitly timestamp and reconcile sparse/camera signals. [P12] [P13]

---

# 12. Full-body spatial representation

## 12.1 Canonical performer representation

The runtime canonical body should be lighter than a production mesh but expressive enough for retargeting:

- root transform;
- approximately 22–30 body joints;
- articulated feet/toes;
- wrists plus hand roots;
- clavicles and spine segments;
- optional SMPL-X/MHR-compatible pose code for teacher interchange;
- fixed session bone lengths;
- surface/shape code optional and low priority.

### TA-073 — Use a canonical articulated skeleton as the realtime contract and reserve dense mesh for teacher/debug paths

Avatar output needs stable bone rotations and contacts, not a full dense human mesh every tick. HMR/SMPL-X provide rich teacher representations, while retargeting and IK operate on skeletons and end effectors. [P02] [P37] [P40]

### TA-074 — Preserve articulated feet and clavicles

Do not reduce the body to coarse hips/knees/ankles and shoulders/elbows/wrists. Feet and clavicles materially affect dancing, weight transfer, arm gestures, and guitar posture; SAM 3D Body explicitly includes feet and hands, and whole-body datasets include expressive full-body articulation. [P02] [P49]

## 12.2 Spatial student architecture

Proposed student tiers:

```text
input: person crop + mask/keypoint prompts
backbone: efficient conv/ViT hybrid, selectable width/resolution
neck: multi-scale feature pyramid
body query decoder: canonical joints + root + shape + uncertainty
optional mesh head: low-rate/debug only
outputs: 6D rotations, camera-relative joints, root depth prior,
         compact fusion tokens, visibility/confidence
```

The student is trained from synthetic ground truth and teacher pseudo-labels. It should expose width/resolution variants sharing the same tensor contract.

### TA-075 — Train a family of elastic spatial students rather than one fixed-size model

Hardware varies widely. Once-for-All-style progressive shrinking supports subnetworks with different depth, width, kernel size, and resolution under different latency constraints. [P55]

### TA-076 — Distill region confidence and intermediate structure, not only final joints

The student shall imitate teacher pose, rotations, masks/keypoints, uncertainty, and compact feature targets where practical. Knowledge distillation works by transferring softened/structured teacher knowledge rather than only hard labels. [P51]


# 13. Face, gaze, and lip tracking

The face branch must feel immediate and expressive enough that users do not need a phone or wearable face tracker for ordinary streaming. It should not wait for full-body HMR.

## 13.1 Face branch

Per selected face view:

1. Track a high-resolution face ROI.
2. Estimate dense landmarks/mesh and head pose.
3. Estimate identity-neutral expression coefficients.
4. Estimate eye openness, brow motion, cheek/jaw state, and gaze.
5. Fuse audio features for lip timing and hidden/blurred mouth frames.
6. Temporally stabilize expression channels.
7. Map to avatar expressions/blendshapes.

### TA-077 — Run face tracking as an independent high-rate branch

Face inference shall target 45–60 Hz where hardware permits and remain operational when body HMR is reduced. Attention Mesh demonstrates realtime high-fidelity face regions, while avatar latency studies justify keeping expressive channels off the slow body path. [P31] [P11] [P12]

### TA-078 — Separate identity shape from expression state

The face model shall estimate a temporary neutral/identity code and a dynamic expression code. Only expression and head/gaze motion are sent to the avatar; the neutral code is discarded at handoff. DECA and EMOCA explicitly separate detailed face geometry/identity from expression and emotion-driven animation. [P32] [P33]

## 13.2 Multi-camera face selection

Usually one camera provides the best face detail. Additional cameras serve as fallback during head turns and occlusion. The scheduler prefers:

- highest face pixel size;
- near-frontal or useful complementary angle;
- low blur/exposure;
- recent confidence;
- minimal hand/instrument occlusion.

### TA-079 — Fuse face views selectively rather than averaging expressions

The face branch shall choose or confidence-fuse views per region, because an oblique view may help jaw/head orientation while a frontal view better captures lips and brows. View-specific weighting follows the broader multiview reweighting principle. [P19] [P31]

## 13.3 Gaze

Gaze outputs:

- left/right eye orientation;
- vergence estimate when reliable;
- blink/eyelid state;
- confidence;
- optional screen-target smoothing mode for desktop VTubing.

### TA-080 — Use a dedicated unconstrained gaze estimator conditioned on head pose

Dense face landmarks alone do not provide robust gaze. A gaze-specific network shall estimate eye direction and uncertainty. L2CS-Net targets fine-grained gaze in unconstrained environments. [P34]

## 13.4 Audio-visual lips

Audio processing extracts causal features such as phoneme/viseme evidence, voice activity, onset, and energy. Video remains authoritative for visible mouth closure, asymmetry, and non-speech expressions.

### TA-081 — Fuse causal audio features with visual mouth evidence

Audio shall improve timing and resolve blurred/occluded lip states but shall not override a clearly visible mouth. AVFace demonstrates detailed audio-visual face reconstruction and motivates cross-modal fusion. [P35]

### TA-082 — Keep audio-to-mouth alignment independently calibrated

Audio device latency can differ from camera latency. The application shall estimate and expose audio offset, train with offset perturbations, and permit bounded automatic refinement. Multisignal avatar research shows desynchronization degrades motion quality. [P12] [P35]

## 13.5 Expression retargeting

Avatar expression mapping may use:

- direct standardized blendshape mappings when available;
- a linear/MLP mapping from canonical facial action channels;
- avatar-specific limits and nonlinear response curves;
- blink and gaze safety rules;
- optional stylization gain.

### TA-083 — Retarget semantic facial actions rather than copy raw vertex motion

A shared semantic expression space permits different stylized faces to respond consistently, while avatar-specific curves control exaggeration and prevent mesh breakage. EMOCA’s expression/emotion representation and face-animation research support semantic expression transfer. [P33] [P35]

---

# 14. Hand and finger tracking

Hands are resolution-limited in a full-body frame and require a dedicated path. The system should be honest: gross hand pose can be excellent from ordinary cameras; convincing individual fretting fingers require sufficient pixels, view coverage, and short exposure.

## 14.1 Hand pipeline

For each hand:

1. Predict wrist location and hand box from body state.
2. Expand box according to uncertainty and velocity.
3. Crop the native-resolution image.
4. Run a hand detector/mesh model.
5. Resolve handedness using body-chain context.
6. Transform hand pose into world/canonical coordinates.
7. Fuse views and instrument contacts.
8. Temporally estimate finger rotations and confidence.
9. Retarget to avatar hand hierarchy.

### TA-084 — Use dedicated hand localization and mesh reconstruction

The body model’s hand output is a prior, not the final finger tracker. WiLoR and HaMeR demonstrate dedicated in-the-wild hand localization/mesh reconstruction and robustness to diverse viewpoints and occlusion. [P29] [P30]

### TA-085 — Condition hand inference on body wrist pose and handedness

Body context provides crop prediction, left/right identity, wrist orientation, and plausible arm-to-hand continuity. Whole-body and hand reconstruction should be coupled through shared kinematics rather than independent detections. [P02] [P29]

## 14.2 Multi-camera hand fusion

Per-hand tokens include:

- 2D/3D hand joints;
- MANO/hand pose latent when available;
- wrist transform from body path;
- hand-view confidence;
- prop-contact candidates;
- source age;
- image feature token.

Fuse using the same set-based principle as the body but with a shorter, higher-rate temporal model.

### TA-086 — Fuse hands independently from whole-body view choice

A camera that is poor for full-body reconstruction may be excellent for a fretting hand. Hand-heavy views shall be scheduled independently. Per-region camera weighting and arbitrary-view set fusion support this design. [P19] [P21]

### TA-087 — Gate finger detail according to evidence

When finger confidence falls, preserve gross palm/wrist orientation and reduce high-frequency articulation rather than output arbitrary finger poses. Hand-reconstruction research shows occlusion remains difficult even for strong models; uncertainty must control detail. [P29] [P30]

## 14.3 Hand temporal model

State includes finger joint rotations, velocities, contact probabilities, palm pose, and hand-object relation. Use 200–800 ms of causal history. Rapid strums require low lag; stationary fretting requires strong jitter suppression.

### TA-088 — Use motion-dependent hand smoothing

The hand path shall reduce smoothing during fast motion and increase stabilization during slow contact. The 1€ filter provides a simple speed-dependent jitter/lag principle, while the learned temporal model handles larger structural uncertainty. [P14] [P30]

## 14.4 Hand limitations and UX

The setup wizard estimates hand pixel coverage. It may recommend:

- move one camera closer;
- use a higher shutter/lighting level;
- designate a side camera as a hand/instrument view;
- accept “gesture quality” rather than “finger quality” when hands are too small.

The product should never claim finger-level accuracy when the image contains too few pixels.

---

# 15. Guitar and instrument-performance mode

Guitar is the primary stress test because it couples root/torso posture, two hands, an external rigid object, rapid strumming, fine fretting, and frequent occlusion.

## 15.1 Instrument representation

A guitar profile contains:

- body center and approximate oriented bounding volume;
- neck axis and length;
- fretboard plane;
- bridge/string region;
- optional instrument scale entered or inferred once;
- hand-contact regions;
- avatar-specific guitar attachment transform when rendering a virtual prop.

The system can use a generic guitar model plus an optional short instrument calibration: show front, hold playing pose, point neck toward one camera.

### TA-089 — Track the instrument as an explicit object, not background occlusion

The system shall estimate guitar pose and geometry so that hidden wrists/fingers are constrained relative to the instrument. Physics-based guitar work demonstrates that synchronized dual-hand performance depends on explicit instrument interaction. [P42]

## 15.2 Guitar visual path

1. Detect guitar/body/neck segmentation in each relevant view.
2. Estimate neck axis, body pose, and confidence.
3. Fuse multi-view object pose.
4. Associate left hand with fretboard and right hand with string/bridge region.
5. Feed object-relative hand coordinates into hand temporal models.
6. Maintain contact graph and reject impossible penetrations.

### TA-090 — Express hand state in both world and object-relative coordinates

A fretting hand is easier to reason about relative to the neck/fretboard than in unconstrained world coordinates. Interaction/retargeting research supports contact- and geometry-aware representations. [P39] [P42]

## 15.3 Audio cues

Extract:

- note/attack onset;
- rhythmic energy;
- spectral/string-performance features where feasible;
- voice-vs-instrument separation confidence.

Use audio to refine event timing, not to invent pose. A visible strumming hand remains authoritative.

### TA-091 — Use audio as a probabilistic timing cue for string interaction

String-performance capture research reports that audio improves markerless motion capture for instrument performance. The fusion shall be confidence-gated and causal. [P43]

### TA-092 — Train musical-motion priors on multimodal music datasets where licenses permit

Motion and music semantics can improve plausible rhythm, posture, and missing-frame continuation. MOSA and related multimodal datasets provide a research basis, subject to dataset licensing. [P44]

## 15.4 Guitar contact graph

```text
left palm/wrist  ── near ── neck
left fingertips ─ contact ─ fretboard/string zones
right wrist     ── near ── bridge/string region
right hand path ─ crosses ─ string plane at audio-consistent times
guitar body     ─ supported by ─ torso/strap/lap state
```

Contacts are soft probabilities with hysteresis. The model can be visually convincing without identifying the exact played fret on low-resolution video; it must maintain plausible hand placement and timing.

### TA-093 — Optimize gross contact correctness before exact fret identity

The product shall prioritize convincing hand-to-neck placement, strum timing, wrist orientation, elbow posture, and body support. Fine fret classification is enabled only when visual evidence supports it. This follows the product’s avatar-fidelity objective and the difficulty demonstrated by dexterous guitar control research. [P42] [P43]

## 15.5 Synthetic guitar generation

Blender generation varies:

- acoustic/electric/bass body shapes;
- neck lengths and scales;
- straps, seated/standing poses;
- left/right-handed playing;
- strumming/picking/fingerstyle gross motion;
- chord/fret hand placements;
- sleeves, long hair, occlusion, reflections;
- camera positions optimized and randomized around front/side views;
- audio-aligned event labels when procedural or MIDI-derived motion exists.

### TA-094 — Generate exact object, hand, contact, and camera labels synthetically

Real guitar footage rarely provides synchronized 3D hands, object pose, camera geometry, and contacts. Synthetic multiview generation can provide all of these labels and has demonstrated transfer value in human reconstruction. [P45] [P47] [P52]

---

# 16. Avatar-aware retargeting and final animation solve

## 16.1 Retarget objective

At time `t`, solve avatar pose `q_A` and root `r_A` from canonical performer state `x_H`:

```text
E(q_A, r_A) =
    λsem  E_motion_semantics
  + λori  E_bone_orientation
  + λee   E_end_effector_direction_and_reach
  + λcon  E_active_contacts
  + λbal  E_balance_and_support
  + λlim  E_joint_limits
  + λint  E_self/prop_interpenetration
  + λtmp  E_temporal_continuity
  + λsty  E_avatar_style_policy
```

The objective does not demand identical meter coordinates when proportions differ.

### TA-095 — Retarget in normalized semantic space before target-space IK

First preserve relative rotations, movement phase, direction, and normalized reach. Then solve target contacts and limits. Skeleton-aware and residual retargeting methods support separating motion semantics from target geometry correction. [P36] [P39]

### TA-096 — Keep contact constraints authoritative through retargeting

If the performer foot is planted, the avatar foot should remain planted unless target proportions make the constraint impossible. Retargeting research explicitly identifies contact loss and interpenetration as target-geometry problems. [P39]

## 16.2 Root scaling policy

Root horizontal and vertical travel are scaled separately:

- horizontal step length according to avatar leg reach and motion intent;
- vertical pelvis motion according to avatar stance and squat readability;
- head trajectory constrained for VR comfort/anchor consistency;
- chibi avatars may use reduced translation with stronger limb articulation;
- tall/long-legged avatars may require stride expansion but limited acceleration.

### TA-097 — Learn or solve non-uniform root scaling from avatar geometry

A single height ratio cannot preserve squat, step, reach, and seated motion across extreme avatars. Shape-aware retargeting motivates geometry-dependent residual correction. [P39]

## 16.3 Balance and bend planes

The solver maintains:

- knee and elbow preferred bend planes;
- projected center-of-mass/support plausibility;
- hip/chest orientation consistency;
- clavicle contribution to reach;
- foot sole orientation;
- wrist twist limits.

### TA-098 — Combine neural initialization with analytical kinematic constraints

A learned retarget proposal handles stylized motion, while analytical IK enforces bone lengths, joint limits, and end-effector constraints. HybrIK and NIKI demonstrate hybrid/neural IK approaches for recovering anatomically coherent pose. [P37] [P38]

## 16.4 Output representations

Internal output is full avatar pose. Adapters derive:

- VRM bone rotations and expressions;
- VMC packets;
- VRChat OSC/SteamVR virtual tracker targets;
- preview renderer transforms;
- BVH/GLTF animation recording where supported.

Virtual tracker output is synthesized from the solved avatar, not directly from raw human joints. This lets the solver place trackers where the target avatar’s IK expects them. The concrete adapters must conform to the current VRM, VMC, and VRChat OSC specifications. [O07] [O08] [O09]

### TA-099 — Generate protocol outputs from the final avatar solve

Protocol targets shall reflect the retargeted avatar state so that downstream IK does not receive human-proportion coordinates that conflict with the selected model. Retargeting research supports solving target geometry before output. [P36] [P39]

---

# 17. Failure containment and confidence state machine

The system is designed around graceful degradation, not the assumption that the model is always correct.

## 17.1 Confidence states

Each joint/region uses:

- `GREEN`: directly observed with agreement.
- `YELLOW`: partially observed, conflicting, stale, or prior-assisted.
- `RED`: unobserved beyond the safe prediction horizon or geometrically inconsistent.

Output behavior is region-specific:

| State | Position/rotation behavior | Detail behavior |
|---|---|---|
| GREEN | normal low-lag correction | full detail |
| YELLOW | bounded temporal continuation; lower correction gain | suppress high-frequency fingers/face noise |
| RED | safe constrained hold/decay; no large translation | preserve gross silhouette, disable speculative detail |

### TA-100 — Expose confidence to the final animation solver

Retargeting must know which constraints are measurements and which are predictions. Uncertainty-aware multiview estimation and motion-prior work motivate confidence-conditioned continuation and correction. [P07] [P23]

## 17.2 Catastrophic-event guards

Examples:

- knee rotation beyond anatomical/Avatar Profile limit;
- root velocity jump beyond configured bound;
- bone-length inconsistency;
- left/right limb swap;
- foot crossing floor while contact is active;
- hand teleport through guitar;
- face expression spike from a bad crop;
- stale inference result older than current state by threshold;
- camera reprojection residual spike.

The guard may reject, clamp, or quarantine the responsible observation and logs a structured event.

### TA-101 — Validate physical and temporal invariants after every learned module

Learned outputs are proposals, not unquestioned truth. Hybrid IK, contact-aware reconstruction, and robust motion priors all demonstrate the value of explicit constraints around neural estimates. [P05] [P07] [P37]

## 17.3 Reacquisition

When a lost joint returns:

1. Verify spatial consistency with chain length and other views.
2. Compare observation to predicted distribution.
3. Use a correction ramp whose duration depends on discrepancy and visibility.
4. Preserve active contacts unless strong release evidence exists.
5. Escalate to body-level reset only when multiple core joints disagree persistently.

### TA-102 — Reacquire locally before resetting globally

A lost wrist must not reset the pelvis or entire temporal model. Per-joint uncertainty and view weighting support localized recovery. [P19] [P23]


# 18. OS- and GPU-neutral inference architecture

“GPU-neutral” does not mean one generic kernel path that performs equally on every vendor. It means:

1. the application and model contracts do not depend on CUDA;
2. models are distributed in a portable representation;
3. vendor runtimes are selected behind one interface;
4. Linux and Windows receive the same tracking features;
5. provider performance and fallback are measured and visible;
6. compact fallback models exist when a high-performance provider is unavailable.

## 18.1 Native core

Recommended shipping core:

- C++20 for capture, scheduling, geometry, state estimation orchestration, retargeting, protocol output, and provider integration;
- a cross-platform UI toolkit chosen independently of the inference core;
- Python retained for training, data generation, conversion, and research tooling—not required for end users;
- C ABI boundaries for optional provider or protocol plugins.

### TA-103 — Keep the realtime core native and decouple inference from the output thread

The output loop shall have predictable allocation and scheduling behavior, while neural inference runs on separate workers. ANIRA demonstrates a cross-platform architecture that decouples variable neural inference from a time-critical callback and includes latency management/benchmarking. [P58]

## 18.2 Inference abstraction

```cpp
class IInferenceBackend {
public:
    virtual BackendCapabilities probe() = 0;
    virtual CompiledModel compile(const ModelPackage&, const CompileProfile&) = 0;
    virtual InferenceTicket enqueue(
        CompiledModel&,
        TensorBindings inputs,
        Deadline deadline,
        StreamClass stream) = 0;
    virtual InferenceResult poll(InferenceTicket&) = 0;
    virtual RuntimeStats stats(const CompiledModel&) const = 0;
};
```

The application interacts with model-semantic names, not provider-specific operator APIs.

### TA-104 — Use an application-level inference-provider interface

The runtime shall isolate hardware-specific compilation, memory, stream, and profiling behavior behind a stable interface. TVM, MNN, and ONNX Runtime’s execution-provider architecture all demonstrate separating model graphs from hardware-specific execution. [P56] [P57] [P59]

## 18.3 Canonical model format

The distributable model package uses ONNX as the canonical graph format, accompanied by a manifest and optional provider-specific caches.

### TA-105 — Use ONNX as the portable model interchange contract

Training may occur in PyTorch or other frameworks, but release artifacts shall include validated ONNX graphs. An empirical conversion study found ONNX conversion errors relatively small in its tested models, while cross-platform systems use ONNX-style interchange to separate training from deployment. Conversion parity must still be tested per model. [P60] [P58]

### Important limitation

ONNX portability is not automatic performance portability. Operator coverage, graph partitioning, layout, dynamic shapes, and precision support differ among providers. Every release requires provider-specific correctness and latency tests.

## 18.4 Default runtime: ONNX Runtime

ONNX Runtime is the primary backend because its execution-provider abstraction can partition graphs across hardware-specific libraries while preserving one C/C++ API. Current official documentation lists CUDA, TensorRT, DirectML, OpenVINO, MIGraphX, WebGPU, and other providers. [O01]

### TA-106 — Use ONNX Runtime as the default cross-platform execution host

The default shipping backend shall be ONNX Runtime, but the application-level interface must permit alternative runtimes for compact models or future providers. The decision combines the cross-platform execution-provider principle with evidence that runtime/provider choice materially changes latency and energy. [P59] [P60]

## 18.5 Provider strategy by platform

### Windows

| Hardware | Preferred path | Secondary path | Compatibility path |
|---|---|---|---|
| NVIDIA GPU | Windows ML NvTensorRT RTX or standalone TensorRT RTX EP-ABI plugin where validated | CUDA EP | DirectML / CPU compact models |
| AMD Radeon GPU | Windows ML MIGraphX where supported | DirectML or optional compact Vulkan backend | CPU compact models |
| AMD Ryzen AI NPU | Windows ML Vitis AI where supported | DirectML/CPU for unsupported models | CPU compact models |
| Intel GPU/NPU/CPU | Windows ML or bundled OpenVINO EP | DirectML | CPU |
| Unknown D3D12 GPU | DirectML compatibility | experimental WebGPU or compact Vulkan backend after validation | CPU |
| CPU only | ORT CPU/OpenVINO CPU where advantageous | reduced model tier | — |

### Linux

| Hardware | Preferred path | Secondary path | Compatibility path |
|---|---|---|---|
| NVIDIA GPU | TensorRT or CUDA EP | compact Vulkan backend | CPU compact models |
| AMD Radeon GPU | MIGraphX/ROCm stack | compact Vulkan backend | CPU compact models |
| Intel GPU/NPU/CPU | OpenVINO EP | compact Vulkan backend | CPU |
| Other Vulkan-capable GPU | compact MNN/TVM Vulkan backend | CPU | — |
| CPU only | ORT CPU/OpenVINO CPU where supported | reduced model tier | — |

Current implementation facts must be checked against official provider documentation at build/release time. DirectML is Windows-only and is currently in sustained-engineering mode; Windows ML is the preferred new Windows integration and can dynamically acquire hardware-specific execution providers. Standalone ONNX Runtime MIGraphX packages remain closely tied to the ROCm/Linux stack, while OpenVINO supports Intel CPU/GPU/NPU paths on Windows and Linux. For NVIDIA RTX, the built-in TensorRT RTX EP is deprecated; use the standalone EP-ABI plugin or the Windows ML-managed provider when validated. [O02] [O03] [O04] [O05] [O06]

### TA-107 — Select providers per model, not once for the entire application

The body spatial model, 2D detector, face model, hand model, fusion transformer, and temporal model may favor different providers. Startup benchmarking shall select a provider/profile per model subject to correctness and memory constraints. Runtime studies show that performance and energy are difficult to predict and vary by model/provider; TensorRT, CUDA, ORT, and other runtimes do not have one universal ranking. [P59]

### TA-108 — Keep a compact Vulkan-capable fallback for broad GPU reach

For small 2D, face, hand, or temporal models, the project should maintain an optional MNN- or TVM-based Vulkan path when native ORT providers are unavailable. MNN and TVM are designed for portable optimized deployment across heterogeneous hardware. [P56] [P57]

The heavy HMR model is not required to support the generic Vulkan path in the first release. If operator coverage or latency is inadequate, the application uses a smaller spatial tier or CPU and clearly reports the limitation.

### TA-109 — Do not promise equivalent speed across vendors

The product shall promise feature portability and graceful quality scaling, not identical throughput. Runtime research shows provider performance varies with model, batch size, and hardware. [P59]

## 18.6 No silent fallback

At session creation, record:

- provider requested;
- provider actually assigned per graph node/subgraph where available;
- CPU fallback percentage;
- compile/cache time;
- warm and steady-state latency;
- memory usage;
- numerical parity result.

The UI displays a concise summary such as:

```text
Body spatial: NVIDIA TensorRT, 96% accelerated, 22.4 ms p50
Hand model: CUDA, 100% accelerated, 4.8 ms p50
Temporal: CPU, 0.7 ms p50
Warning: Face model fell back 41% to CPU; using Small profile.
```

### TA-110 — Reject hidden heavy-model CPU partitioning when it violates latency

A model that silently partitions expensive operators onto CPU can appear “supported” while missing realtime targets. The runtime shall enforce a maximum fallback ratio or measured deadline. ONNX Runtime’s provider architecture permits heterogeneous partitioning, and runtime studies show the chosen provider materially affects performance. [P59] [P60]

## 18.7 Startup benchmark and auto-tuning

On first launch or hardware/driver/model change:

1. Enumerate devices/providers.
2. Validate numerical parity against reference test vectors.
3. Warm each candidate model/profile.
4. Measure p50/p95 latency, memory, and concurrent interference.
5. Benchmark representative one-camera and two-camera schedules.
6. Store a signed local performance profile keyed by hardware, driver, runtime, and model hashes.
7. Choose default quality mode and expose the result to the user.

### TA-111 — Benchmark steady-state and tail latency before selecting a provider

The selector shall not rely on vendor identity alone. Real-time inference work reports initial-inference and runtime violations, while comparative runtime research shows performance depends on model/provider configuration. [P58] [P59]

### TA-112 — Re-benchmark after material software or hardware changes

Driver, runtime, model graph, precision, or camera-count changes invalidate the cached profile. Cross-platform deployment studies show conversion/runtime behavior must be empirically validated. [P60]

## 18.8 Model package format

Suggested package:

```text
model_name/
  manifest.json
  fp32/model.onnx
  fp16/model.onnx                 optional
  int8_qdq/model.onnx             optional
  test_vectors/
  calibration_stats.json
  provider_profiles/
  licenses/
  provenance.json
```

Manifest fields:

```json
{
  "semantic_name": "body_spatial_medium",
  "version": "0.3.0",
  "opset": 20,
  "inputs": {"image": [1, 3, 512, 384], "keypoints": [1, 1, 133, 3]},
  "outputs": {"pose6d": [1, 30, 6], "joint_cov": [1, 30, 6]},
  "supported_precisions": ["fp32", "fp16", "int8_qdq"],
  "max_numerical_delta": 0.002,
  "stateful": false,
  "deadline_class": "spatial_heavy",
  "license_manifest": "licenses/index.json"
}
```

### TA-113 — Ship fixed-shape, provider-compatible realtime profiles

Realtime model variants shall use fixed batch and spatial shapes wherever possible, with separate exported profiles for common resolutions. The broad-compatibility package shall target the lowest ONNX opset required by the release provider matrix; as of this research sweep, the DirectML documentation supports through opset 20, so the example manifest uses opset 20. Higher-opset provider-specific variants may ship only when conversion and parity tests justify them. Static shapes improve compilation and memory planning on many accelerators; elastic network training can provide multiple profiles without independent training. [P55] [P56] [P60] [O02]

### TA-114 — Keep stateful temporal cache tensors explicit in the model contract

The temporal model’s cache/state shall be named inputs/outputs so every provider executes identical causal semantics. OnlineHMR’s causal cache design supports explicit incremental state. [P01]

## 18.9 Precision policy

Recommended default:

- FP16 on GPUs that execute it efficiently and pass parity tests;
- BF16 where hardware/provider support is stronger;
- INT8 QDQ for selected convolutional detectors and MLPs after quantization-aware training;
- FP32 for numerically sensitive geometry, covariance, small optimizers, and CPU fallbacks;
- never quantize camera matrices or final constraint solves merely for uniformity.

### TA-115 — Use mixed precision selectively

Mixed precision shall target neural layers with validated numerical stability, while geometry and state-estimation math remain at sufficient precision. Mixed-precision research demonstrates speed/memory benefits with maintained training quality when scaling is handled correctly. [P54]

### TA-116 — Use quantization-aware training for production INT8 paths

Post-training quantization may be evaluated, but release INT8 models shall use QAT when accuracy or temporal stability degrades. Integer-only inference research shows QAT can preserve accuracy while enabling efficient deployment. [P53]

### TA-117 — Calibrate precision against avatar-space behavior, not only tensor error

Parity tests shall include foot contact, root trajectory, face expression, and hand continuity. A small tensor difference can cause a large IK/contact change. Avatar fidelity research shows motion artifacts can dominate perceived quality. [P12] [P13]

## 18.10 Memory and tensor movement

Policies:

- preallocate recurrent and per-frame tensor arenas;
- reuse pinned host buffers where supported;
- avoid image copies between crop and model stages;
- batch crops only when deadline permits;
- keep geometry/fusion tokens compact;
- permit device-resident chaining only as an optimization, never a required contract;
- report device-to-host transfer time separately.

### TA-118 — Preallocate realtime buffers and avoid allocation on the output path

Variable memory allocation and inference setup can create tail-latency spikes. Real-time inference architecture work motivates static pools and decoupling from time-critical callbacks. [P58]

### TA-119 — Optimize end-to-end latency rather than isolated model throughput

A faster kernel that adds copies, batching delay, or synchronization may hurt avatar latency. Runtime benchmarking must include preprocessing, transfers, inference, and postprocessing. [P58] [P59]

## 18.11 Compiled-engine caching

TensorRT, OpenVINO, MIGraphX, and other providers may compile or optimize graphs. Cache artifacts by:

```text
model hash + provider version + driver + device ID + precision + input profile
```

Invalid caches are rebuilt safely.

### TA-120 — Warm and cache provider-specific compiled artifacts

First-run compilation cannot occur during live output. Real-time inference work observes longer initial inferences, and current accelerator providers expose compilation/cache behavior. [P58]

## 18.12 Cross-platform build and packaging

Recommended repository layout:

```text
/apps/desktop
/core/capture
/core/time
/core/calibration
/core/geometry
/core/fusion
/core/temporal
/core/retarget
/core/output
/inference/api
/inference/ort
/inference/mnn_or_tvm_optional
/platform/windows
/platform/linux
/models/manifests
/tools/replay
/tools/benchmark
/training
/simulation
```

Binary packages:

- common application/core;
- CPU runtime;
- optional provider packs detected/installed separately where licensing or size requires;
- model packs by quality tier;
- debug/replay tools.

### TA-121 — Keep provider dependencies modular

The base application shall not require every vendor SDK. Provider packs are loadable modules with explicit versions. Execution-provider systems are designed to abstract hardware libraries behind stable interfaces. [P56] [P57] [P59]

---

# 19. Realtime scheduler and quality controller

## 19.1 Deadline classes

| Class | Examples | Behavior when late |
|---|---|---|
| Critical | 60/90 Hz output, VR anchors | Never wait for inference; predict from current state |
| High | face, gaze, hand crops | Drop stale request; run newest crop |
| Medium | fast body 2D | Drop superseded frames; maintain cross-camera coverage |
| Heavy | spatial HMR | Reschedule or skip; never block output |
| Background | calibration refinement, diagnostics | Pause under pressure |

### TA-122 — Schedule by deadlines and information value, not FIFO

Realtime inference requests shall be prioritized according to output deadline and expected uncertainty reduction. Decoupled real-time inference architectures motivate deadline-aware queues over simple sequential execution. [P58]

## 19.2 Dynamic quality controller

Every one to five seconds, estimate:

- output missed ticks;
- p50/p95 per-model latency;
- GPU/CPU utilization;
- memory pressure;
- camera queue age;
- body/hand/face uncertainty;
- active motion intensity;
- current camera count.

Available knobs:

- spatial HMR rate;
- spatial input resolution;
- number of heavy views `K`;
- fast-detector resolution/rate;
- hand crop rate/resolution;
- face detail tier;
- temporal model tier;
- optional mesh head;
- batch size;
- output preview quality.

### TA-123 — Adapt model width, resolution, and rate independently

The quality controller shall choose among elastic/subnetwork profiles instead of only changing one global FPS slider. Once-for-All networks support specialized subnetworks across depth, width, kernel, and resolution under hardware constraints. [P55]

### TA-124 — Protect face and output cadence before heavy body quality

When overloaded, first disable debug mesh, reduce nonessential heavy views, or lower spatial resolution. The output loop, HMD anchors, face, and fast body observations retain priority. Latency and animation-fidelity research support prioritizing responsiveness and continuity. [P11] [P13]

## 19.3 Motion-aware scheduling

Motion intensity is estimated from 2D keypoint velocity, optical flow, audio/instrument events, and temporal state.

- Idle/talking: lower body HMR rate; stronger stabilization.
- Dance: increase body observation rate; prioritize feet/root and complementary views.
- Guitar: prioritize hand/prop views and face; body can remain moderate.
- Crouch/sit transition: temporarily increase pelvis/feet observation rate.
- Turn/spin: prioritize cameras with emerging visibility.

### TA-125 — Allocate compute according to current motion uncertainty

A fixed schedule wastes compute while idle and undersamples rapid/contact-rich motion. View-quality and uncertainty-aware fusion support directing effort to uncertain regions and times. [P19] [P23]

## 19.4 Batching policy

Microbatching may combine:

- same-model crops from multiple cameras at the same deadline;
- left/right hand crops;
- face crops from two candidate views.

It must not wait beyond a small configurable window merely to fill a batch.

### TA-126 — Batch only when the measured end-to-end deadline improves

Throughput optimization is subordinate to latency. Runtime research shows performance varies with batch size and provider; benchmarking must decide. [P59]

## 19.5 Example schedules

### One camera, Balanced

```text
camera 60/30 Hz
fast body 2D: 30–60 Hz
spatial body: 20–24 Hz
face: 45–60 Hz
hands: 30–45 Hz when visible
fusion/temporal: observation-driven
avatar output: 60 Hz desktop / 90 Hz VR
```

### Two cameras, Quality

```text
camera A/B: 30–60 Hz each
fast body 2D: all useful frames
spatial body A: 15–20 Hz
spatial body B: 15–20 Hz, staggered
combined strong observation opportunity: 30–40 Hz
face: best view 45–60 Hz; fallback view lower rate
hands: selected views 30–60 Hz according to instrument/motion
avatar output: 60/90 Hz
```

### Four cameras, Performance

```text
fast body 2D: all cameras 30–60 Hz
spatial body: two selected views per cycle, total 30–45 jobs/s
fusion: set of current observations from all cameras
face: best one/two views
hands/prop: independent view scheduling
avatar output: 60/90 Hz
```

### TA-127 — Cap heavy-view count even as cameras increase

The default scheduler shall keep `K ≤ 2` heavy spatial views per observation cycle, increasing only when benchmarks show value. Set-based fusion and per-view weighting allow cheap observations from all cameras to inform the solve. [P19] [P21]


# 20. Training and data architecture

The data plan assumes no private capture studio and no ability to commission a proprietary mocap dataset. The project therefore combines:

1. licensed/public research datasets;
2. procedural synthetic multiview data;
3. teacher-generated pseudo-labels on legally usable real video;
4. legally usable VTuber/avatar clips as a motion-distribution source;
5. self-supervised consistency on unlabeled footage where the license permits training;
6. later optional community contributions, never required for initial feasibility.

Publicly viewable is not the same as licensed for model training. Every sample must have a recorded provenance and allowed-use category.

## 20.1 Data tiers

| Tier | Source | Primary use | Label quality |
|---|---|---|---|
| A | Synthetic Blender scenes | calibration, multiview fusion, contacts, props, corruption, exact camera labels | Exact |
| B | Public 3D motion datasets | motion prior, temporal model, full-body/hand/face motion diversity | Dataset ground truth |
| C | Public synthetic-human datasets | spatial generalization and synthetic-to-real transfer | Exact/controlled |
| D | Licensed real human video datasets | spatial student, real appearance, validation | Mixed 2D/3D |
| E | Licensed/permissioned VTuber videos | avatar motion distribution, animation critic, motion mining | Pseudo/semantic |
| F | Unlabeled legally usable multiview video | reprojection, temporal, teacher consistency | Self-supervised |

### TA-128 — Use synthetic data as the primary source of multiview/calibration/contact supervision

Exact camera, timing, visibility, depth, contact, and prop labels are prohibitively expensive in real footage. BEDLAM and Syn4D show synthetic data can train or improve human reconstruction and multiview tasks with transfer to real imagery. [P45] [P47]

### TA-129 — Use public mocap corpora for motion dynamics, not camera appearance

AMASS, Motion-X, and Motion-X++ provide diverse body or expressive whole-body motion that can drive simulation and temporal priors. Their motion labels are separated from rendered appearance. [P48] [P49] [P50]

### TA-130 — Preserve source licenses and provenance at sample level

The dataset builder shall produce a machine-readable manifest containing source, license, permitted use, transformations, model-derived labels, and redistributability. This is an engineering governance decision; the research basis for combining many heterogeneous datasets comes from scaled hand/whole-body training work. [P30] [P49]

## 20.2 Recommended public research sources

Potential sources, subject to their current licenses and access terms:

- AMASS for broad body motion. [P48]
- Motion-X / Motion-X++ for expressive whole-body, hands, face, text, and multimodal motion. [P49] [P50]
- BEDLAM / BEDLAM2 for synthetic clothed bodies and lifelike motion. [P45] [P46]
- Syn4D for exact multiview camera/depth/tracking supervision. [P47]
- Existing hand datasets aggregated by HaMeR/WiLoR, where their licenses permit derivative training. [P29] [P30]
- Face datasets used by DECA/EMOCA/AVFace, where permitted. [P32] [P33] [P35]
- Music and string-performance data such as MOSA or the cited string-performance dataset, where access terms permit. [P43] [P44]

The project should avoid coupling its legal viability to any single restricted body model or dataset. The source manifest must allow replacement.

## 20.3 VTuber-clip motion-mining pipeline

VTuber final-output video does not reveal the original human body. It is still useful for learning the distribution of convincing avatar motion.

```text
licensed VTuber/avatar video
        │
        ├─> shot/character/visibility segmentation
        ├─> 2D avatar landmarks + optical flow + silhouette
        ├─> approximate rig/pose recovery with temporal prior
        ├─> motion-quality filter and deduplication
        ├─> canonical semantic motion sequence
        └─> retarget to synthetic human rigs
                     │
                     └─> re-render from known random cameras
                              => exact synthetic labels
```

### TA-131 — Use VTuber clips primarily as motion-distribution evidence

The system shall not treat a rendered avatar’s pixels as ground truth for the original performer. It shall mine timing, gesture, posture, face–body coordination, dance style, and avatar-motion priors, then regenerate exact labels through synthetic rendering. Motion-X demonstrates scalable automatic whole-body motion annotation from video, while retargeting work supports canonicalizing motion across skeletons. [P49] [P36]

### TA-132 — Train an avatar-motion critic only on legally usable output clips

A critic may score temporal naturalness, stability, and style of final avatar sequences, but it cannot justify unauthorized dataset use. Whole-body motion datasets and avatar-fidelity studies support sequence-level quality objectives. [P49] [P13]

## 20.4 Blender synthetic-data generator

The generator must be reproducible, headless, shardable, and able to resume from manifests.

### Scene dimensions

Randomize:

- 1–8 cameras;
- camera position, height, roll, focal length, crop, distortion, resolution, and timing offset;
- room scale, floor, walls, furniture, desks, chairs, microphones, stands, guitars, keyboards;
- lighting direction, intensity, color temperature, flicker, and background contrast;
- human height, proportions, skin tone, hair, clothing, sleeves, skirts, coats, loose fabric approximations;
- avatar target skeletons and proportions;
- motion source, speed, mirroring, blending, and transitions;
- occluders and self-occlusion;
- exposure, blur, noise, compression, rolling shutter, dropped/duplicated frames;
- multiple bystanders during calibration tests while retaining one active performer.

### Exact outputs per frame/camera

```text
RGB before/after webcam corruption
depth and metric point map
person/part/prop segmentation
optical flow and motion vectors
camera intrinsics/extrinsics/distortion
exposure midpoint and stream time offset
2D keypoints with visibility/occlusion
3D canonical joints and rotations
body mesh/shape when licensed representation permits
root trajectory and velocities
foot/toe/heel/hand/seat contacts
floor and support surfaces
face expression and gaze
hand mesh/joints
instrument pose and hand–instrument contacts
source canonical motion
retargeted avatar pose and contacts
```

### TA-133 — Generate clean physical truth first, then apply sensor corruption

The pipeline shall preserve pristine labels and apply a separate webcam simulation layer. This permits controlled curriculum and corruption ablations. Domain randomization and synthetic-human datasets support broad appearance/sensor randomization around exact labels. [P45] [P47] [P52]

### TA-134 — Randomize camera count and ordering on every training sample

Fusion training shall see one through N views, arbitrary missing cameras, duplicated/similar views, poor baselines, and randomized token ordering. Set Transformer and arbitrary-view HMR motivate permutation-invariant, camera-count-agnostic training. [P20] [P21]

### TA-135 — Simulate unsynchronized and unreliable USB cameras

Each synthetic camera receives independent frame rate, phase, delay, drop pattern, duplicate frames, exposure, and clock drift. Kineo and spatiotemporal calibration research explicitly address unsynchronized consumer cameras. [P15] [P16]

### TA-136 — Randomize calibrator and performer independently

A synthetic site sequence contains calibrator identity/shape/clothing A, then performer B through N using the same cameras. The network and tests must succeed without retaining A. [P15] [P18]

## 20.5 Procedural avatar generator

Generate target skeletons with:

- height from chibi to tall stylized;
- head-to-body ratios;
- leg/torso/arm ratios;
- shoulder and hip width;
- hand/foot scale;
- different rest poses and bone axes;
- missing optional bones;
- extra twist bones;
- varied joint limits;
- skirt/tail/secondary motion excluded from core solve but represented as downstream physics.

Run canonical motions through an offline constrained retarget teacher to produce target labels.

### TA-137 — Train retargeting across procedural target skeletons

The retarget model must learn geometry-conditioned behavior before seeing user avatars. Skeleton-aware and shape-aware retargeting research supports broad source/target geometry variation. [P36] [P39]

## 20.6 Teacher stack

The offline teacher can be slow:

```text
strong 2D whole-body detector
+ SAM 3D Body / strong spatial HMR per selected frame
+ dedicated high-capacity hand model
+ high-quality face/expression/gaze model
+ known or estimated multiview camera geometry
+ robust triangulation
+ long-window temporal optimization / motion prior
+ contact and floor optimization
+ prop/instrument optimization
+ avatar retarget teacher
```

Teacher outputs include uncertainty and disagreement flags, not only final pose.

### TA-138 — Use an ensemble/optimization teacher and a compact student

The training teacher may combine high-capacity spatial models, long windows, multiview geometry, and optimization unavailable at runtime. Distillation transfers its outputs into efficient students. [P02] [P03] [P07] [P51]

### TA-139 — Reject low-consensus pseudo-labels rather than force supervision

Pseudo-labels with large multiview residual, hand disagreement, implausible contacts, or temporal instability are masked or downweighted. SAM 3D Body itself uses a multi-stage annotation/data engine, and uncertainty-aware methods support confidence-filtered supervision. [P02] [P23]

## 20.7 Training curriculum

### Stage 0 — Dataset and replay infrastructure

Before training custom models, generate reproducible shards, manifests, exact labels, corruption variants, and deterministic evaluation sequences.

### Stage 1 — Site calibrator

Train/evaluate camera/timing/floor estimators on synthetic camera graphs and public multiview sequences. Inputs are 2D tracks, confidence, timestamps, and optional audio features. Output is Site State and covariance.

### Stage 2 — Spatial body student

Distill strong HMR on real and synthetic crops. Train pose, root prior, confidence, compact features, and prompt use. Freeze or slowly fine-tune the base encoder depending on data scale.

### Stage 3 — Geometry and fusion

Start with exact camera geometry and synthetic 2D observations. Add detector noise, calibration error, missing cameras, and teacher hypotheses progressively. Train the set fusion residual and uncertainty heads.

### Stage 4 — Temporal/contact model

Train first on exact AMASS/Motion-X sequences with synthetic observation corruption, then on teacher-labeled real sequences. Add root/contact constraints and asynchronous modalities.

### Stage 5 — Face, gaze, audio, and hands

Train dedicated branches and compact temporal fusion. Condition hands on body and props; face on audio and head state.

### Stage 6 — Avatar retargeter

Train on procedural avatars with exact source motion, target contacts, and offline IK/optimization targets.

### Stage 7 — Guitar/instrument mode

Train prop pose, hand–prop contacts, audio timing, and instrument-specific temporal priors.

### Stage 8 — End-to-end sequence fine-tuning

Unroll short causal sequences through fusion, temporal state, contacts, and retargeting. Optimize avatar-space continuity while preserving modular testability.

### Stage 9 — Distillation, pruning, QAT, and provider validation

Produce Small/Medium/Large profiles, quantize selected modules, export ONNX, and validate every provider.

### TA-140 — Train modules progressively before end-to-end sequence tuning

A staged curriculum isolates camera geometry, spatial perception, fusion, motion, contacts, and retargeting before coupling them. Existing HMR systems similarly integrate specialized spatial, temporal, and trajectory modules rather than learn the entire task from raw video in one step. [P01] [P05]

### TA-141 — Unroll causal state during training exactly as it runs live

Teacher forcing alone can hide accumulated prediction errors. Training shall include autoregressive rollout, observation gaps, and correction events. Causal online-HMR and motion-prior work motivate matching live temporal conditions. [P01] [P07]

---

# 21. Loss functions and supervision

No single loss captures “looks right on an avatar.” Training uses a weighted family of losses and evaluates downstream behavior.

## 21.1 Calibration losses

```text
L_calib =
    λreproj × robust 2D reprojection
  + λepi    × epipolar consistency
  + λcam    × synthetic camera-parameter error
  + λtime   × temporal-offset error
  + λdist   × lens-distortion error
  + λfloor  × floor/gravity error
  + λscale  × metric-scale error
  + λgraph  × camera-graph consistency
```

### TA-142 — Supervise calibration with both parameter error and reprojection behavior

Camera parameters can differ numerically while projecting similarly, and low reprojection error can hide poor depth geometry. Both parameter and reconstruction losses are needed. Calibration-free multiview research evaluates camera and reconstruction quality jointly. [P15] [P16]

## 21.2 Spatial body losses

```text
L_spatial =
    λ2d      × keypoint reprojection
  + λ3d      × canonical joint error
  + λrot     × rotation geodesic/6D loss
  + λmesh    × optional mesh/vertex loss
  + λbone    × fixed-bone consistency
  + λmask    × silhouette agreement
  + λshape   × sequence/session shape consistency
  + λunc     × uncertainty likelihood/calibration
  + λteacher × teacher distillation
```

### TA-143 — Supervise rotations and joints, not only mesh vertices

Avatar animation consumes bone rotations and end-effectors. Direct rotation/joint supervision reduces reliance on mesh-to-skeleton conversion. HMR and hybrid IK research support joint/rotation-aware prediction. [P02] [P37] [P41]

## 21.3 Fusion losses

```text
L_fusion =
    λtri  × fused 3D joint/rotation error
  + λrep  × per-view reprojection
  + λview × view-weight supervision or ranking
  + λcons × cross-view consistency
  + λunc  × Gaussian/Laplace negative log-likelihood
  + λcal  × expected calibration error / coverage
  + λdrop × camera-dropout robustness
```

### TA-144 — Train uncertainty with a proper probabilistic objective

Confidence shall correspond to expected error, not merely detector score. Probabilistic triangulation and uncertainty-aware multiview estimation use likelihood-based uncertainty. [P22] [P23]

## 21.4 Temporal and root losses

```text
L_temporal =
    λpose  × pose error
  + λvel   × velocity error
  + λacc   × acceleration error
  + λjerk  × jerk regularization
  + λroot  × root trajectory/orientation error
  + λcont  × contact classification
  + λslide × planted-foot velocity
  + λfloor × penetration/hover error
  + λinnov × correction magnitude under noisy observations
  + λgap   × occlusion-gap rollout error
  + λfuture× short-horizon prediction error
```

### TA-145 — Optimize velocity, acceleration, contacts, and root trajectory explicitly

Frame-wise pose accuracy does not guarantee stable animation. WHAM and avatar-artifact studies identify foot sliding, trajectory, and velocity as critical. [P05] [P12]

### TA-146 — Penalize visible foot skating in world and avatar space

A planted foot should have near-zero world velocity, then remain planted after retargeting. Contact-aware trajectory and retargeting research support both source- and target-space contact objectives. [P05] [P39]

## 21.5 Face and hand losses

```text
L_face = expression + landmarks + head pose + gaze + blink
       + audio/visual lip sync + temporal continuity + confidence

L_hand = 2D/3D joints + rotations + mesh/shape + wrist continuity
       + handedness + temporal continuity + object contact + uncertainty
```

### TA-147 — Train face and hand quality at crop resolution and final avatar channels

Dedicated high-resolution hand/face models should be supervised both geometrically and through the semantic channels delivered to avatars. [P30] [P31] [P33] [P35]

## 21.6 Instrument losses

```text
L_guitar =
    prop pose/axis error
  + hand-to-prop relative pose
  + fingertip/fretboard proximity
  + strum-plane crossing timing
  + audio-onset alignment
  + penetration penalty
  + dual-hand coordination
```

### TA-148 — Train dual-hand coordination and audio alignment jointly

Guitar quality depends on coordinated left/right hands and timing relative to sound. Physics-based guitar and audio-enhanced string-performance research directly support this objective. [P42] [P43]

## 21.7 Avatar-space losses

```text
L_avatar =
    semantic direction/phase
  + target end-effector placement
  + target joint limits
  + target contacts
  + target foot skating
  + balance/support
  + self/prop interpenetration
  + temporal continuity
  + optional perceptual critic score
```

### TA-149 — Include the final avatar solve in sequence training/evaluation

A human-space improvement that creates worse target IK is not a product improvement. Retargeting research and avatar-fidelity studies support evaluating final animated motion. [P36] [P39] [P13]

## 21.8 Loss scheduling

Weights vary by curriculum and motion mode. Contact losses should not dominate before contact labels are reliable; hand detail should not force hallucination under low visibility. Uncertainty-aware masking is required.

### TA-150 — Weight supervision according to observability and label trust

Exact synthetic labels, high-consensus pseudo-labels, and weak motion-mined labels shall not receive equal weight. SAM 3D Body’s data engine and uncertainty-aware pose methods support confidence-aware annotation use. [P02] [P23]


---

# 22. Latency engineering in detail

## 22.1 Latency equation

For a camera-derived output:

```text
L_total =
    L_sampling
  + L_exposure_and_sensor
  + L_driver_and_transport
  + L_capture_queue
  + L_preprocess
  + L_model_queue
  + L_inference
  + L_fusion
  + L_temporal_prediction
  + L_constraint_and_retarget
  + L_protocol
  + L_consumer_render
```

The application directly controls only part of this chain. Every trace event must carry the original source timestamp so that hidden driver buffering is measurable.

### TA-151 — Measure source-time latency, not only CPU function duration

A model benchmark that ignores camera buffering and queuing does not represent avatar responsiveness. VR latency research evaluates end-to-end feedback delay, and real-time inference work emphasizes complete latency management. [P11] [P58]

## 22.2 Sampling and prediction

At 30 Hz, a randomly timed movement waits an average of approximately 16.7 ms for the next frame before exposure/transport; at 60 Hz, approximately 8.3 ms. Long exposure adds blur and can shift the effective observation time. The temporal model predicts from source time to the intended output presentation time.

```text
prediction_horizon =
    measured_capture_to_state_latency
  + expected_protocol/render_lead
```

### TA-152 — Predict to the output target timestamp

The output state shall represent the best causal estimate at the time it will be consumed, not the old source-frame time. Online causal reconstruction and latency-sensitive VR interaction support time-aligned prediction. [P01] [P11]

### TA-153 — Clamp prediction horizon under degraded latency

If a provider stalls and prediction horizon becomes too long, the system shall reduce model quality or hold/decay uncertain components rather than extrapolate indefinitely. Motion-prior work supports short ambiguous gaps but not unbounded hallucination. [P07] [P09]

## 22.3 Example one-camera Quality budget

Illustrative target on capable hardware with a reliable 60 Hz camera:

| Stage | Target p50 |
|---|---:|
| Sampling + exposure midpoint | 8–12 ms |
| Driver/USB/dequeue | 4–10 ms |
| Crop/preprocess | 1–3 ms |
| Fast body/ROI | 3–7 ms |
| Scheduled spatial HMR | 18–32 ms |
| Fusion/temporal update | 1–4 ms |
| Contact/retarget/output packaging | 1–3 ms |
| Output tick alignment | 0–11 ms |
| **Approximate camera-to-output** | **45–82 ms** |

The heavy HMR may not execute on every source frame. Fast observations update the temporal state between heavy passes.

## 22.4 Example two-camera Quality budget

Two cameras are processed asynchronously:

```text
time 0 ms     fast A + optional heavy A
     8 ms     fast B
    16 ms     fast A
    25 ms     heavy B opportunity
    33 ms     fast A/B updates
    50 ms     heavy A opportunity
```

Fusion uses the newest timestamp-aligned evidence rather than waiting for a perfect A/B pair. When fast 2D rays from both cameras exist, geometry can update even without a new heavy HMR result.

### TA-154 — Do not barrier-synchronize ordinary camera frames

A hard barrier adds the delay of the slowest camera and can amplify USB jitter. Fusion shall align observations by timestamps and uncertainty. Unsynchronized-camera research is specifically designed around asynchronous streams. [P15] [P16]

## 22.5 Separate apparent smoothness from measurement freshness

The 60/90 Hz output loop can be visually smooth while underlying evidence is stale. The UI and logs must show both:

- output tick rate;
- age of the newest body, hand, face, and root observation;
- prediction horizon;
- uncertainty.

### TA-155 — Never market interpolated output Hz as inference Hz

The application shall report observation rates and output rates separately. Avatar artifact research shows frame mismatch can degrade quality; transparent telemetry prevents a high output rate from hiding stale evidence. [P12]

## 22.6 Latency test method

Three test levels:

1. **Software trace:** timestamp every stage and queue.
2. **Visual loop test:** filmed LED/physical motion and rendered avatar on a high-speed camera.
3. **Instrument/audio test:** compare visible pick/string event, audio onset, and avatar strum.

### TA-156 — Validate instrumentation with an external end-to-end measurement

Internal clocks can miss camera or display buffering. VR latency research measures action-to-feedback delay, so release validation requires an external visual loop test. [P11]

---

# 23. Evaluation framework

The project succeeds when users see a faithful avatar performance, not when one benchmark number improves.

## 23.1 Evaluation layers

| Layer | Question |
|---|---|
| Perception | Did each camera detect visible body/face/hand/prop evidence correctly? |
| Geometry | Are camera parameters, rays, and triangulated joints consistent? |
| Human state | Is the canonical human performance plausible and faithful? |
| Temporal | Is motion stable, responsive, and free of discontinuities? |
| Contact/world | Are feet, root, support, and prop contacts credible? |
| Avatar | Does the selected stylized model perform the same readable action? |
| Product | Does setup work for ordinary users and unseen people? |
| Runtime | Are latency and behavior consistent across OS/GPU providers? |

### TA-157 — Evaluate every subsystem and the final avatar separately

End-to-end output alone cannot identify whether a failure came from capture, calibration, perception, fusion, temporal state, or retargeting. Modular HMR architectures and avatar-fidelity studies motivate layered evaluation. [P01] [P12] [P13]

## 23.2 Core quantitative metrics

### Camera/site

- rotation/translation/intrinsic error on synthetic and known-camera data;
- reprojection error;
- temporal-offset error;
- floor normal/height error;
- triangulation angle/condition;
- camera-move detection precision/recall.

### Human pose

- MPJPE/PA-MPJPE where appropriate;
- joint rotation geodesic error;
- root translation/orientation error;
- bone-length variance within session;
- hand joint/mesh error;
- face landmark/expression/gaze error.

### Temporal/avatar

- joint velocity/acceleration/jerk error;
- maximum one-frame joint displacement;
- root-jump count and magnitude;
- foot-skate distance while contact active;
- floor penetration/hover duration;
- contact precision/recall/F1;
- reacquisition overshoot;
- visible failure events per hour;
- avatar-space end-effector/contact error;
- dance phase/timing similarity;
- guitar hand-to-prop and strum-event timing.

### Runtime

- camera/source age;
- p50/p95/p99 latency;
- missed output ticks;
- provider fallback percentage;
- memory use;
- power/thermal degradation where measurable.

### TA-158 — Treat velocity, contact, root, and failure-event metrics as first-class

Average pose error alone misses the artifacts users notice. WHAM and self-avatar studies specifically identify foot sliding, velocity, synchronization, and occlusion as critical. [P05] [P12]

## 23.3 Perceptual evaluation

Blind A/B protocol:

- same avatar, same camera angle, same lighting;
- compare proposed output with current webcam baseline, IMU/sparse-tracker output, and a high-quality reference when available;
- motion categories: idle/talk, reach, turn, crouch, sit/stand, dance, guitar;
- ask viewers to rate faithfulness, stability, responsiveness, expressiveness, foot/root quality, hands, face, and overall preference;
- include experts and ordinary viewers;
- report confidence intervals and failure clips, not only averages.

### TA-159 — Make blind avatar-space preference the primary north-star evaluation

The product is optimized for the final stylized avatar, so perceptual comparisons after identical retarget/render conditions are essential. SAM 3D Body reports user-preference evaluation alongside quantitative metrics, and avatar-fidelity research measures the perceptual impact of animation quality. [P02] [P13]

## 23.4 Cross-person site-transfer test

Protocol:

1. Person A calibrates the site.
2. A leaves.
3. At least ten people with varied height, build, clothing, skin tone, mobility, and hair enter separately.
4. Each uses Instant Guest and Quality Guest modes.
5. No site geometry is recalibrated.
6. Measure time-to-good, proportion leakage, root/floor behavior, and calibration residual.
7. Repeat with a different calibrator and same guests.

### TA-160 — Gate release on cross-person calibration transfer

A site system that only works for its calibration human violates the product requirement. Human-based calibration research establishes the geometric objective; this test ensures implementation does not leak identity-specific state. [P15] [P18]

## 23.5 Low-quality webcam suite

Test dimensions:

- 480p/720p/1080p;
- 30/60 Hz and bursty rates;
- high compression;
- low light and long exposure;
- motion blur;
- narrow/wide FOV;
- different color balance across cameras;
- duplicate/drop patterns;
- 10–80 ms inter-camera offset;
- USB congestion;
- partial feet/hand coverage;
- similar camera baselines;
- camera movement after calibration.

### TA-161 — Maintain a corruption-stratified benchmark

Aggregate results can hide that the model fails on the exact cheap-camera conditions it targets. Synthetic-domain-randomization research supports controlled corruption sweeps and real transfer tests. [P45] [P47] [P52]

## 23.6 Dance suite

Include:

- rapid arm crossing;
- spins/half-turns;
- side steps;
- kicks;
- squat/bounce;
- floor contact changes;
- fast tempo;
- loose clothing;
- hands near face;
- temporary total limb occlusion.

Evaluate motion phase, foot contacts, root trajectory, reacquisition, and failure events.

### TA-162 — Evaluate long continuous dance sequences, not isolated poses

Temporal drift, contact flicker, and rare catastrophic events emerge over time. Video HMR and motion-prior work evaluate sequences because independent-frame accuracy is insufficient. [P04] [P05] [P07]

## 23.7 Guitar suite

Include:

- standing and seated;
- acoustic and electric body shapes;
- left/right-handed;
- slow chords, fast strumming, picking, hand shifts;
- face looking down at the fretboard;
- sleeves and hair occlusion;
- one camera, two general cameras, and optional hand camera.

Metrics:

- guitar 6DoF/axis stability;
- left-hand neck proximity and orientation;
- right-hand string-plane crossings;
- audio-to-avatar timing;
- elbow/wrist/body posture;
- subjective “convincing performance” rating.

### TA-163 — Use guitar as an integrated acceptance test

Guitar performance exercises body, hands, prop pose, audio, occlusion, contacts, and retargeting simultaneously. Dual-hand guitar and audio-enhanced string-performance research make it a technically grounded stress test. [P42] [P43]

## 23.8 Provider parity matrix

Every model release is tested on representative:

- Windows NVIDIA;
- Windows AMD;
- Windows Intel;
- Linux NVIDIA;
- Linux AMD;
- Linux Intel/CPU;
- CPU-only reduced tier.

For each: numerical parity, avatar metrics, p50/p95 latency, memory, fallback, 30-minute stability.

### TA-164 — Gate model releases on provider parity at avatar level

Cross-platform conversion can produce small numerical differences or large performance differences. Provider validation must include the final animation, not merely model output tensors. [P60] [P59] [P13]

## 23.9 Failure-event taxonomy

Every severe event is tagged:

```text
CALIBRATION_DRIFT
CAMERA_STALE
CAMERA_MOVED
LEFT_RIGHT_SWAP
ROOT_DEPTH_JUMP
ROOT_YAW_JUMP
FOOT_CONTACT_BREAK
KNEE_FLIP
ELBOW_FLIP
HAND_TELEPORT
FINGER_EXPLOSION
FACE_SPIKE
GAZE_SPIKE
PROP_LOSS
PROP_PENETRATION
HANDOFF_STATE_LEAK
PROVIDER_STALL
OUTPUT_TICK_MISS
```

### TA-165 — Track severe failures per hour and time-to-recovery

Rare catastrophic glitches are more damaging than small average error and can be hidden by aggregate metrics. The product must explicitly count them. Avatar-artifact studies document major degradation from noise, occlusion, and desynchronization. [P12]

---

# 24. Observability, debugging, and replay

## 24.1 Developer overlays

Toggleable views:

- source timestamps and frame age;
- 2D joints/covariance ellipses;
- camera rays and triangulated joints;
- per-joint camera weights;
- fused uncertainty;
- root evidence hierarchy;
- contact probabilities and active constraints;
- temporal prediction vs accepted observation;
- avatar IK residuals;
- provider assignment and latency timeline;
- camera calibration residual/health.

### TA-166 — Make uncertainty and fusion decisions visible

A black-box “bad pose” is difficult to fix. Visualizing view weights, uncertainty, and residuals is essential for diagnosing probabilistic multiview systems. [P19] [P22] [P23]

## 24.2 Structured trace

Use a compact binary trace with events:

```text
FRAME_CAPTURED
FRAME_DROPPED
MODEL_QUEUED
MODEL_STARTED
MODEL_FINISHED
OBSERVATION_FUSED
TEMPORAL_UPDATE
CONTACT_CHANGED
ROOT_CORRECTION
OUTPUT_EMITTED
FAILURE_GUARD_TRIGGERED
PROVIDER_RECONFIGURED
```

Every event includes monotonic time, source time, camera/model ID, and sequence ID.

### TA-167 — Record causal provenance for every emitted pose

The debugger should be able to answer which observations and predictions produced an output. Online cached state and asynchronous fusion make causal provenance necessary. [P01] [P12]

## 24.3 Privacy modes

- Default local-only processing.
- Raw video recording off by default.
- Debug traces may store observations without images.
- Performer state cleared on handoff.
- Dataset contribution requires explicit opt-in and a visible manifest.

These are product/privacy decisions rather than paper-derived accuracy choices. The architecture supports them through state separation and local inference.

---

# 25. Implementation roadmap for an open-source spare-time project

The roadmap is structured to invalidate risky assumptions early. Do not begin by training a giant end-to-end model.

## Phase 0 — Replay, metrics, and avatar output foundation

Deliverables:

- canonical skeleton and coordinate frames;
- VRM preview and one output protocol;
- deterministic recording/replay;
- latency trace;
- basic avatar import/profile;
- metric/failure framework.

Exit test: recorded canonical animation produces stable output on at least three very different avatars.

### TA-168 — Build replay and avatar-space evaluation before custom perception

Without deterministic playback, improvements cannot be attributed or reproduced. Temporal HMR and avatar-fidelity research depend on sequence-level comparison. [P01] [P13]

## Phase 1 — One-camera offline teacher pipeline

Deliverables:

- strong existing 2D detector;
- SAM-class/offline HMR;
- fixed performer proportions;
- temporal smoothing and contact prototype;
- VRM output on recorded video.

Exit test: materially better stability than direct MediaPipe-style landmarks on standard clips.

### TA-169 — Validate the final animation thesis with existing teachers before training a student

The project should first prove that strong spatial estimates plus temporal/contact/retarget cleanup yield better avatar output. Distillation is justified only after the teacher pipeline is valuable. [P02] [P05] [P51]

## Phase 2 — Two cameras with manual development calibration

Deliverables:

- timestamped dual-camera recording;
- conventional/calibration-target development path allowed internally;
- robust triangulation;
- fused canonical pose;
- side-by-side single-vs-dual evaluation.

Exit test: clear reduction in depth ambiguity, occlusion failures, foot/root instability, and guitar hand placement.

### TA-170 — Prove multiview value before solving calibration-free UX

Manual calibration is acceptable only as a development control. Learnable triangulation establishes the expected benefit; this phase determines whether the chosen cameras/models realize it. [P24]

## Phase 3 — Human-motion automatic site calibration

Deliverables:

- guided motion sequence;
- timing/intrinsic/extrinsic/distortion/floor solve;
- placement score;
- persistent Site State;
- person A/B transfer tests.

Exit test: calibration transfer to unseen people without quality collapse.

### TA-171 — Treat automatic calibration as an independent shippable subsystem

Calibration-free capture research provides a separate, measurable camera-recovery problem. It should be testable without the final neural fusion network. [P15] [P16]

## Phase 4 — Causal temporal/root/contact system

Deliverables:

- streaming state model;
- root hierarchy;
- contact solver;
- bounded corrections;
- 60/90 Hz output prediction;
- severe-event guards.

Exit test: 30-minute scripted session without uncontrolled root jump or limb explosion.

### TA-172 — Stabilize root and contacts before adding more visual detail

World-grounded HMR and avatar studies show root/foot/temporal artifacts dominate perceived quality. [P05] [P12]

## Phase 5 — Cross-platform inference skeleton

Deliverables:

- C++ `IInferenceBackend`;
- ONNX Runtime CPU plus at least one GPU path on each OS;
- provider benchmark/selection;
- no-silent-fallback diagnostics;
- fixed-shape model packaging.

Exit test: identical replay produces acceptable parity on Windows and Linux.

### TA-173 — Establish portability before final model optimization

A CUDA-only prototype can become structurally difficult to port. Cross-platform inference systems support separating graph/model contracts from hardware backends. [P56] [P57] [P58]

## Phase 6 — Dedicated face and hands

Deliverables:

- high-rate face/gaze/audio path;
- dedicated hand crops/mesh;
- confidence-gated fingers;
- avatar expression/hand mapping.

Exit test: ordinary talking/gesturing no longer requires a separate face tracker for acceptable output; hands fail gracefully.

### TA-174 — Add dedicated modality paths after the body timing/state foundation

Face and hand models benefit from dedicated high-resolution processing, but should join a stable timestamped architecture. [P30] [P31] [P35]

## Phase 7 — Synthetic generator and first compact students

Deliverables:

- headless Blender generator;
- exact labels and corruption layer;
- dataset manifest/provenance;
- small spatial student;
- set fusion model;
- temporal/contact model training.

Exit test: student approaches teacher avatar-space quality at materially lower latency.

### TA-175 — Distill only against a measured teacher target and exact synthetic labels

Knowledge distillation and synthetic human datasets support this combination. [P45] [P47] [P51]

## Phase 8 — N-camera scheduler

Deliverables:

- permutation-invariant fusion;
- dynamic view selection;
- per-region scheduling;
- camera quarantine;
- compute scaling tests from 1–8 cameras.

Exit test: adding a useful camera never degrades quality under the supported compute budget.

### TA-176 — Require monotonic evidence benefit from added cameras

Set-based fusion and uncertainty weighting should ignore useless views rather than average them into the result. [P19] [P21] [P23]

## Phase 9 — Guitar mode

Deliverables:

- guitar detection/pose;
- hand–instrument contact graph;
- audio timing;
- synthetic guitar corpus;
- guitar acceptance suite.

Exit test: blind viewers rate the avatar as convincingly playing, with stable prop and plausible two-hand contact.

### TA-177 — Treat instrument mode as a specialized layer on the general body/hand system

Guitar research supports explicit dual-hand/object/audio coordination, but the core body pipeline remains shared. [P42] [P43]

## Phase 10 — Product hardening

Deliverables:

- one-screen setup wizard;
- cross-person demo/kiosk mode;
- crash recovery;
- provider packs/install diagnostics;
- long-session tests;
- signed model manifests;
- public benchmark clips and failure disclosures.

Exit test: an unfamiliar user can install, import a model, perform guided poses, and obtain tracking without developer intervention.

---

# 26. Recommended repository and component interfaces

## 26.1 Modules

```text
capture_core
    CameraDevice, CaptureSession, TimestampMapper

site_calibration
    TrackCollector, TimeAligner, CameraGraphSolver, FloorSolver,
    CalibrationValidator, SiteHealthMonitor

perception_fast
    PersonTracker, WholeBody2D, MaskEstimator, CropPlanner, ViewQuality

perception_spatial
    SpatialBodyScheduler, SpatialBodyModel, SpatialFeatureContract

face_stack
    FaceTracker, ExpressionModel, GazeModel, AudioVisualLipModel

hand_stack
    HandCropScheduler, HandModel, HandTemporalState

prop_stack
    PropDetector, GuitarPose, ContactGraph, AudioEventExtractor

fusion_core
    RayBuilder, RobustTriangulator, SetFusionModel, UncertaintyCalibrator

temporal_core
    CausalStateModel, PredictionClock, InnovationGate, MotionMode

world_contact
    RootState, ContactState, ConstraintSolver, FailureGuards

avatar_core
    AvatarImporter, AvatarProfile, RetargetModel, IKConstraintSolver

outputs
    VRMPreview, VMCAdapter, VRChatTrackerAdapter, RecordingAdapter

inference
    IInferenceBackend, OrtBackend, OptionalVulkanBackend,
    ProviderProfiler, ModelPackage, MemoryArena

tools
    Recorder, Replay, Benchmark, DatasetInspector, ProviderParity
```

## 26.2 Versioned interfaces

Every packet/model contract includes a schema version. Model package upgrades require converters or explicit incompatibility errors.

### TA-178 — Version semantic model contracts independently from implementation

Modular research architectures evolve components independently; versioned contracts prevent a new hand or temporal model from silently changing coordinate or confidence semantics. [P01] [P58]

## 26.3 Plugin boundaries

Suitable plugins:

- output protocols;
- optional inference providers;
- prop/instrument modes;
- dataset readers;
- preview/render integrations.

Core geometry, temporal state, confidence semantics, and Avatar Profile format should remain centrally versioned to prevent incompatible behavior.

### TA-179 — Keep high-risk state semantics in the core and low-risk integrations as plugins

The causal pose state and uncertainty contract must remain consistent across providers and outputs. Cross-platform runtime papers support pluggable inference while maintaining a stable host architecture. [P56] [P58]

---

# 27. Security, update, and reproducibility architecture

## 27.1 Model trust

Model packages include:

- cryptographic hash;
- manifest signature for official builds;
- source/provenance/license manifest;
- expected input/output schema;
- resource limits;
- test vectors;
- supported provider profiles.

Untrusted custom models run only after explicit user opt-in and resource validation.

### TA-180 — Treat models as executable computational artifacts

Model graphs can consume unbounded memory/compute or use unsupported operators. Portable deployment studies emphasize validation of converted models and runtime behavior. [P60]

## 27.2 Update policy

- Application, model packs, and provider packs version separately.
- A new model does not replace the old one until provider parity and replay tests pass.
- Rollback remains available.
- Site and Avatar Profile migrations are versioned.

### TA-181 — Gate updates through deterministic replay and provider matrices

Cross-platform numerical/performance differences require release-specific validation, not assumed compatibility. [P59] [P60]

## 27.3 Reproducible training

Store:

- source manifests and licenses;
- generator commit and Blender version;
- random seeds and scene manifests;
- teacher/model hashes;
- training configuration;
- evaluation clips and metric versions;
- export/quantization tool versions.

### TA-182 — Make synthetic data regenerable from scene manifests

Synthetic datasets are most useful when exact conditions and labels can be reproduced and ablated. BEDLAM/Syn4D-style synthetic generation provides the underlying precedent. [P45] [P47]

---

# 28. Architecture risks and mitigation

| Risk | Why it matters | Mitigation |
|---|---|---|
| Spatial teacher too slow | Cannot ship directly | Use teacher offline; distill elastic students; schedule few heavy views. |
| Synthetic-to-real gap | Fusion or hands fail on cheap webcams | Start from real-image pretrained models; aggressive corruption; small licensed real validation sets. |
| Auto-calibration instability | Root/depth failures | Separate calibrator; validation holdout; camera quarantine; optional developer target calibration. |
| Unsynchronized webcams | Dance/guitar ghosting | Source timestamps, offset estimation, asynchronous fusion, age-aware tokens. |
| Root hallucination | Entire avatar teleports | Root hierarchy, consensus gating, contacts, bounded innovations. |
| Hand resolution | Instrument looks fake | Native crops, hand-focused view, prop constraints, confidence-gated detail. |
| Extreme avatar proportions | Generic retarget breaks | Procedural-avatar training, two-stage retarget, target-space contacts/limits. |
| Provider fragmentation | Linux/AMD/Intel users suffer | ONNX contract, provider packs, compact Vulkan/CPU tier, visible benchmarks. |
| Tail-latency stalls | Smooth average but random freezes | Deadline queues, stale drop, preallocation, provider p95 selection. |
| Public-data licensing | Project cannot distribute/train legally | Per-sample provenance, replaceable sources, permissive/opt-in content only. |
| Person-state leakage | Demo works only for calibrator | hard state separation, cross-identity synthetic sequences, release gate. |
| Overclaiming professional parity | Trust failure | publish bounded scenarios, blind tests, failure clips, raw latency and event rates. |

### TA-183 — Optimize for bounded, disclosed operating conditions

The project should publish where it performs well and where visual information is insufficient. Uncertainty-aware reconstruction and avatar-artifact research support graceful degradation rather than false certainty. [P12] [P23]

---

# 29. Technical release gates

## Gate A — Architectural prototype

- deterministic replay;
- explicit time/coordinate contracts;
- one camera to canonical skeleton to avatar;
- latency trace;
- no permanent performer state.

## Gate B — Two-view proof

- calibrated two-view triangulation;
- per-joint confidence;
- clear offline improvement over single camera;
- root/feet stability on dance and guitar clips.

## Gate C — Automatic site calibration

- guided human-motion calibration;
- unseen-person transfer;
- moved-camera detection;
- no checkerboard in user flow.

## Gate D — Realtime quality

- 60/90 Hz causal output;
- hardware-dependent 15–30+ Hz spatial observations;
- body median target under 90 ms in recommended mode;
- no uncontrolled failure in scripted 30-minute test.

## Gate E — One-stop avatar system

- automatic Avatar Profile;
- face, gaze, lips, hands, body;
- VRM/VMC and VRChat/SteamVR path;
- stylized avatar retarget suite.

## Gate F — Portable release

- Windows/Linux packages;
- NVIDIA/AMD/Intel provider coverage or explicit compact fallback;
- provider parity tests;
- no silent heavy CPU fallback.

## Gate G — Instrument/dance claim

- blind guitar and dance tests;
- published failure-event rate;
- public comparison clips and reproducible settings.

### TA-184 — Advance claims only after scenario-specific gates pass

The proposed quality is a research target. Quantitative and perceptual sequence evaluation must precede claims of parity with expensive capture. [P02] [P13]

# 30. Research references
The keys below are the research evidence used by the numbered `TA-###` decisions. A link points to the paper or its primary proceedings page. “Preprint” means the architecture should treat the result as promising evidence rather than settled production guidance. Complete author lists and version histories are available on the linked pages.
- [P01] **OnlineHMR: Video-based Online World-Grounded Human Mesh Recovery.** arXiv:2603.17355; accepted CVPR 2026. [https://arxiv.org/abs/2603.17355](https://arxiv.org/abs/2603.17355)
- [P02] **SAM 3D Body: Robust Full-Body Human Mesh Recovery.** arXiv:2602.15989 (2026 preprint). [https://arxiv.org/abs/2602.15989](https://arxiv.org/abs/2602.15989)
- [P03] **Fast SAM 3D Body: Accelerating SAM 3D Body for Real-Time Full-Body Human Mesh Recovery.** arXiv:2603.15603 (2026 preprint). [https://arxiv.org/abs/2603.15603](https://arxiv.org/abs/2603.15603)
- [P04] **SAM-Body4D: Training-Free 4D Human Body Mesh Recovery from Videos.** arXiv:2512.08406 (2025 preprint). [https://arxiv.org/abs/2512.08406](https://arxiv.org/abs/2512.08406)
- [P05] **WHAM: Reconstructing World-grounded Humans with Accurate 3D Motion.** arXiv:2312.07531; CVPR 2024. [https://arxiv.org/abs/2312.07531](https://arxiv.org/abs/2312.07531)
- [P06] **World-Grounded Human Motion Recovery via Gravity-View Coordinates.** arXiv:2409.06662 (2024). [https://arxiv.org/abs/2409.06662](https://arxiv.org/abs/2409.06662)
- [P07] **HuMoR: 3D Human Motion Model for Robust Pose Estimation.** arXiv:2105.04668; ICCV 2021. [https://arxiv.org/abs/2105.04668](https://arxiv.org/abs/2105.04668)
- [P08] **UniCon3R: Unified Contact-aware 4D Human-Scene Reconstruction from Monocular Video.** arXiv:2604.19923 (2026 preprint). [https://arxiv.org/abs/2604.19923](https://arxiv.org/abs/2604.19923)
- [P09] **DiffusionPoser: Real-time Human Motion Reconstruction From Arbitrary Sparse Sensors Using Autoregressive Diffusion.** arXiv:2308.16682; CVPR 2024. [https://arxiv.org/abs/2308.16682](https://arxiv.org/abs/2308.16682)
- [P10] **VIBE: Video Inference for Human Body Pose and Shape Estimation.** arXiv:1912.05656; CVPR 2020. [https://arxiv.org/abs/1912.05656](https://arxiv.org/abs/1912.05656)
- [P11] **The Impact of Latency on Perceptual Judgments and Motor Performance in Closed-loop Interaction in Virtual Reality.** VRST 2016. [https://doi.org/10.1145/2993369.2993381](https://doi.org/10.1145/2993369.2993381)
- [P12] **Self-Avatar Animation in Virtual Reality: Impact of Motion Signals Artifacts on the Full-Body Pose Reconstruction.** arXiv:2404.18628 (2024). [https://arxiv.org/abs/2404.18628](https://arxiv.org/abs/2404.18628)
- [P13] **Animation Fidelity in Self-Avatars: Impact on User Performance and Sense of Agency.** arXiv:2304.05334 (2023). [https://arxiv.org/abs/2304.05334](https://arxiv.org/abs/2304.05334)
- [P14] **1€ Filter: A Simple Speed-based Low-pass Filter for Noisy Input in Interactive Systems.** CHI 2012. [https://doi.org/10.1145/2207676.2208639](https://doi.org/10.1145/2207676.2208639)
- [P15] **Kineo: Calibration-Free Metric Motion Capture From Sparse RGB Cameras.** arXiv:2510.24464 (2025 preprint). [https://arxiv.org/abs/2510.24464](https://arxiv.org/abs/2510.24464)
- [P16] **Spatiotemporal Multi-Camera Calibration using Freely Moving People.** arXiv:2502.12546 (2025). [https://arxiv.org/abs/2502.12546](https://arxiv.org/abs/2502.12546)
- [P17] **Human Pose as Calibration Pattern: 3D Human Pose Estimation with Multiple Unsynchronized and Uncalibrated Cameras.** CVPR Workshops 2018. [https://openaccess.thecvf.com/content_cvpr_2018_workshops/w34/html/Takahashi_Human_Pose_As_CVPR_2018_paper.html](https://openaccess.thecvf.com/content_cvpr_2018_workshops/w34/html/Takahashi_Human_Pose_As_CVPR_2018_paper.html)
- [P18] **Humans as a Calibration Pattern: Dynamic 3D Scene Reconstruction from Unsynchronized and Uncalibrated Videos.** arXiv:2412.19089; ICCV 2025. [https://arxiv.org/abs/2412.19089](https://arxiv.org/abs/2412.19089)
- [P19] **MUC: Mixture of Uncalibrated Cameras for Robust 3D Human Body Reconstruction.** arXiv:2403.05055 (2024). [https://arxiv.org/abs/2403.05055](https://arxiv.org/abs/2403.05055)
- [P20] **Human Mesh Recovery from Arbitrary Multi-view Images.** arXiv:2403.12434 (2024). [https://arxiv.org/abs/2403.12434](https://arxiv.org/abs/2403.12434)
- [P21] **Set Transformer: A Framework for Attention-based Permutation-Invariant Neural Networks.** arXiv:1810.00825; ICML 2019. [https://arxiv.org/abs/1810.00825](https://arxiv.org/abs/1810.00825)
- [P22] **Probabilistic Triangulation for Uncalibrated Multi-View 3D Human Pose Estimation.** arXiv:2309.04756; ICCV 2023. [https://arxiv.org/abs/2309.04756](https://arxiv.org/abs/2309.04756)
- [P23] **UPose3D: Uncertainty-Aware 3D Human Pose Estimation with Cross-View and Temporal Cues.** arXiv:2404.14634; ECCV 2024. [https://arxiv.org/abs/2404.14634](https://arxiv.org/abs/2404.14634)
- [P24] **Learnable Triangulation of Human Pose.** arXiv:1905.05754; ICCV 2019. [https://arxiv.org/abs/1905.05754](https://arxiv.org/abs/1905.05754)
- [P25] **Epipolar Transformers.** arXiv:2005.04551; CVPR 2020. [https://arxiv.org/abs/2005.04551](https://arxiv.org/abs/2005.04551)
- [P26] **RTMPose: Real-Time Multi-Person Pose Estimation based on MMPose.** arXiv:2303.07399 (2023). [https://arxiv.org/abs/2303.07399](https://arxiv.org/abs/2303.07399)
- [P27] **Effective Whole-body Pose Estimation with Two-stages Distillation.** arXiv:2307.15880 (2023). [https://arxiv.org/abs/2307.15880](https://arxiv.org/abs/2307.15880)
- [P28] **RTMO: Towards High-Performance One-Stage Real-Time Multi-Person Pose Estimation.** arXiv:2312.07526 (2023). [https://arxiv.org/abs/2312.07526](https://arxiv.org/abs/2312.07526)
- [P29] **WiLoR: End-to-end 3D Hand Localization and Reconstruction in-the-wild.** arXiv:2409.12259 (2024). [https://arxiv.org/abs/2409.12259](https://arxiv.org/abs/2409.12259)
- [P30] **Reconstructing Hands in 3D with Transformers.** arXiv:2312.05251; CVPR 2024. [https://arxiv.org/abs/2312.05251](https://arxiv.org/abs/2312.05251)
- [P31] **Attention Mesh: High-fidelity Face Mesh Prediction in Real-time.** arXiv:2006.10962 (2020). [https://arxiv.org/abs/2006.10962](https://arxiv.org/abs/2006.10962)
- [P32] **Learning an Animatable Detailed 3D Face Model from In-The-Wild Images.** arXiv:2012.04012; CVPR 2021. [https://arxiv.org/abs/2012.04012](https://arxiv.org/abs/2012.04012)
- [P33] **EMOCA: Emotion Driven Monocular Face Capture and Animation.** arXiv:2204.11312; CVPR 2022. [https://arxiv.org/abs/2204.11312](https://arxiv.org/abs/2204.11312)
- [P34] **L2CS-Net: Fine-Grained Gaze Estimation in Unconstrained Environments.** arXiv:2203.03339 (2022). [https://arxiv.org/abs/2203.03339](https://arxiv.org/abs/2203.03339)
- [P35] **AVFace: Towards Detailed Audio-Visual 4D Face Reconstruction.** arXiv:2304.13115; CVPR 2023. [https://arxiv.org/abs/2304.13115](https://arxiv.org/abs/2304.13115)
- [P36] **Skeleton-Aware Networks for Deep Motion Retargeting.** arXiv:2005.05732; SIGGRAPH 2020. [https://arxiv.org/abs/2005.05732](https://arxiv.org/abs/2005.05732)
- [P37] **HybrIK: A Hybrid Analytical-Neural Inverse Kinematics Solution for 3D Human Pose and Shape Estimation.** arXiv:2011.14672; CVPR 2021. [https://arxiv.org/abs/2011.14672](https://arxiv.org/abs/2011.14672)
- [P38] **NIKI: Neural Inverse Kinematics with Invertible Neural Networks for 3D Human Pose and Shape Estimation.** arXiv:2305.08590; CVPR 2023. [https://arxiv.org/abs/2305.08590](https://arxiv.org/abs/2305.08590)
- [P39] **Skinned Motion Retargeting with Residual Perception of Motion Semantics & Geometry.** arXiv:2303.08658 (2023). [https://arxiv.org/abs/2303.08658](https://arxiv.org/abs/2303.08658)
- [P40] **Expressive Body Capture: 3D Hands, Face, and Body from a Single Image.** arXiv:1904.05866; CVPR 2019. [https://arxiv.org/abs/1904.05866](https://arxiv.org/abs/1904.05866)
- [P41] **On the Continuity of Rotation Representations in Neural Networks.** arXiv:1812.07035; CVPR 2019. [https://arxiv.org/abs/1812.07035](https://arxiv.org/abs/1812.07035)
- [P42] **Synchronize Dual Hands for Physics-Based Dexterous Guitar Playing.** arXiv:2409.16629; SIGGRAPH Asia 2024. [https://arxiv.org/abs/2409.16629](https://arxiv.org/abs/2409.16629)
- [P43] **Audio Matters Too! Enhancing Markerless Motion Capture with Audio Signals for String Performance Capture.** arXiv:2405.04963 (2024). [https://arxiv.org/abs/2405.04963](https://arxiv.org/abs/2405.04963)
- [P44] **MOSA: Music Motion with Semantic Annotation Dataset for Cross-Modal Music Processing.** arXiv:2406.06375 (2024). [https://arxiv.org/abs/2406.06375](https://arxiv.org/abs/2406.06375)
- [P45] **BEDLAM: A Synthetic Dataset of Bodies Exhibiting Detailed Lifelike Animated Motion.** arXiv:2306.16940; CVPR 2023. [https://arxiv.org/abs/2306.16940](https://arxiv.org/abs/2306.16940)
- [P46] **BEDLAM2.0: Synthetic Humans and Cameras in Motion.** arXiv:2511.14394 (2025 preprint). [https://arxiv.org/abs/2511.14394](https://arxiv.org/abs/2511.14394)
- [P47] **Syn4D: A Multiview Synthetic 4D Dataset.** arXiv:2605.05207 (2026 preprint). [https://arxiv.org/abs/2605.05207](https://arxiv.org/abs/2605.05207)
- [P48] **AMASS: Archive of Motion Capture as Surface Shapes.** arXiv:1904.03278; ICCV 2019. [https://arxiv.org/abs/1904.03278](https://arxiv.org/abs/1904.03278)
- [P49] **Motion-X: A Large-scale 3D Expressive Whole-body Human Motion Dataset.** arXiv:2307.00818 (2023). [https://arxiv.org/abs/2307.00818](https://arxiv.org/abs/2307.00818)
- [P50] **Motion-X++: A Large-Scale Multimodal 3D Whole-body Human Motion Dataset.** arXiv:2501.05098 (2025). [https://arxiv.org/abs/2501.05098](https://arxiv.org/abs/2501.05098)
- [P51] **Distilling the Knowledge in a Neural Network.** arXiv:1503.02531; NeurIPS Deep Learning Workshop. [https://arxiv.org/abs/1503.02531](https://arxiv.org/abs/1503.02531)
- [P52] **Domain Randomization for Transferring Deep Neural Networks from Simulation to the Real World.** arXiv:1703.06907; IROS 2017. [https://arxiv.org/abs/1703.06907](https://arxiv.org/abs/1703.06907)
- [P53] **Quantization and Training of Neural Networks for Efficient Integer-Arithmetic-Only Inference.** arXiv:1712.05877; CVPR 2018. [https://arxiv.org/abs/1712.05877](https://arxiv.org/abs/1712.05877)
- [P54] **Mixed Precision Training.** arXiv:1710.03740; ICLR 2018. [https://arxiv.org/abs/1710.03740](https://arxiv.org/abs/1710.03740)
- [P55] **Once-for-All: Train One Network and Specialize it for Efficient Deployment.** arXiv:1908.09791; ICLR 2020. [https://arxiv.org/abs/1908.09791](https://arxiv.org/abs/1908.09791)
- [P56] **TVM: An Automated End-to-End Optimizing Compiler for Deep Learning.** arXiv:1802.04799; OSDI 2018. [https://arxiv.org/abs/1802.04799](https://arxiv.org/abs/1802.04799)
- [P57] **MNN: A Universal and Efficient Inference Engine.** arXiv:2002.12418 (2020). [https://arxiv.org/abs/2002.12418](https://arxiv.org/abs/2002.12418)
- [P58] **ANIRA: An Architecture for Neural Network Inference in Real-Time Audio Applications.** arXiv:2506.12665 (2025 preprint). [https://arxiv.org/abs/2506.12665](https://arxiv.org/abs/2506.12665)
- [P59] **Deep Learning Inference Frameworks Benchmark.** arXiv:2210.04323 (2022). [https://arxiv.org/abs/2210.04323](https://arxiv.org/abs/2210.04323)
- [P60] **An Empirical Study of Challenges in Converting Deep Learning Models.** arXiv:2206.14322 (2022). [https://arxiv.org/abs/2206.14322](https://arxiv.org/abs/2206.14322)
- [P61] **SmartMocap: Joint Estimation of Human and Camera Motion using Uncalibrated RGB Cameras.** arXiv:2209.13906 (2022). [https://arxiv.org/abs/2209.13906](https://arxiv.org/abs/2209.13906)

# 31. Official implementation references

These sources define current runtime, provider, file-format, and protocol behavior. They are operational references, not substitutes for the paper citations attached to architecture decisions. Provider support is time-sensitive and must be revalidated for every release.

- [O01] **ONNX Runtime — Execution Providers overview.** [https://onnxruntime.ai/docs/execution-providers/](https://onnxruntime.ai/docs/execution-providers/)
- [O02] **ONNX Runtime — DirectML Execution Provider; sustained-engineering and Windows ML migration note.** [https://onnxruntime.ai/docs/execution-providers/DirectML-ExecutionProvider.html](https://onnxruntime.ai/docs/execution-providers/DirectML-ExecutionProvider.html)
- [O03] **Microsoft — Windows ML overview and dynamic execution-provider acquisition.** [https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/overview](https://learn.microsoft.com/en-us/windows/ai/new-windows-ml/overview)
- [O04] **ONNX Runtime — AMD MIGraphX Execution Provider.** [https://onnxruntime.ai/docs/execution-providers/MIGraphX-ExecutionProvider.html](https://onnxruntime.ai/docs/execution-providers/MIGraphX-ExecutionProvider.html)
- [O05] **ONNX Runtime — Intel OpenVINO Execution Provider.** [https://onnxruntime.ai/docs/execution-providers/OpenVINO-ExecutionProvider.html](https://onnxruntime.ai/docs/execution-providers/OpenVINO-ExecutionProvider.html)
- [O06] **ONNX Runtime — NVIDIA TensorRT RTX Execution Provider; standalone EP-ABI plugin recommendation.** [https://onnxruntime.ai/docs/execution-providers/TensorRTRTX-ExecutionProvider.html](https://onnxruntime.ai/docs/execution-providers/TensorRTRTX-ExecutionProvider.html)
- [O07] **VRM Consortium — VRM humanoid avatar format.** [https://vrm.dev/en/](https://vrm.dev/en/)
- [O08] **Virtual Motion Capture — VMC Protocol specification.** [https://protocol.vmc.info/english.html](https://protocol.vmc.info/english.html)
- [O09] **VRChat — OSC Trackers specification.** [https://docs.vrchat.com/docs/osc-trackers](https://docs.vrchat.com/docs/osc-trackers)


ChatGPT said:
￼Worked for 122m 11s
Comprehensive design document
￼Download the editable DOCX

The document includes:

The complete product thesis and target user experience.

One-camera, two-camera, and scalable multi-camera operating modes.

Human-pose-based automatic camera calibration and synchronization.

Body, face, eye, lip, and hand-tracking architecture.

VRChat, SteamVR, OSC, VRM, and VMC integration strategies.

Avatar-aware retargeting for highly stylized proportions.

Confidence-aware fusion, contact locking, temporal prediction, interpolation, and failure containment.

Realistic inference-rate and hardware-scaling plans.

A synthetic-data, public-dataset, teacher–student, and self-supervised training strategy.

A staged implementation roadmap suitable for a free-time open-source project.

Quantitative and perceptual evaluation plans.

A detailed grievance-to-design-response matrix.

A comprehensive market sweep covering free webcam tracking, multi-camera applications, software estimation, IMUs, Lighthouse trackers, inside-out trackers, depth cameras, inertial suits, cloud mocap, professional optical systems, face tracking, and hand tracking.

Clickable references to official product documentation, pricing pages, research papers, datasets, support documentation, Steam pages, and clearly labeled anecdotal community evidence.

￼
Commodity_Camera_Avatar_Tracking_Design.docx
Document
￼
￼
￼
￼
￼
￼

You said:
￼
Pasted markdown.md
File
Here is an agent.md that I wrote for a differnent project, granted it's target is a bit different, but it shoudl still give you an idea what the type of structure and philosphy I like. Modify the API modular architecture document with info from this: 
￼

ChatGPT said:
The key tradeoff is reproducibility and repo-wide dependency enforcement versus forcing contributors to learn and maintain an additional packaging layer over the native language ecosystems.

￼
￼
Searched 5 websites
￼
￼
Searched 3 websites
￼
Inspected architecture notes for build-system guidance
I’m folding Rust into that decision too: a Rust-heavy runtime strengthens Bazel’s case somewhat, but GPU/inference ecosystem maturity matters more than language purity.

￼
￼
Searched 1 website

You said:
lowkey wanna just replace cpp with rust, no cpp, rust please!!! 
￼
￼
￼

ChatGPT said:
￼Worked for 10m 28s
Recommendation
Use Rust instead of C++ for the maintained runtime and backend. Do not use Bazel as the primary build system initially.

Use:

￼
Rust/Cargo workspace     Runtime, backend, calibration, fusion, retargeting
Python/uv                Training, evaluation, synthetic-data generation
TypeScript/pnpm          Frontend
Protobuf/OpenAPI         Public contracts
just                     Small root command surface
C/C++                    Narrow native compatibility shims only
Bazel is a good theoretical match for the final monorepo, but it is a poor practical match for the first several versions of this project—especially when native Windows support is mandatory and Rust becomes the primary runtime language.

Rust is a good fit
Nothing in this runtime inherently requires C++.

The expensive numerical work should occur inside:

ONNX Runtime execution providers.

Vendor GPU libraries.

Image-processing kernels.

Precompiled inference models.

Carefully bounded geometry kernels.

The maintained application code is primarily:

Camera scheduling.

Frame synchronization.

Session and calibration state.

Typed packet routing.

Confidence propagation.

Temporal state management.

Multiview fusion orchestration.

Retargeting.

API serving.

Recording and replay.

Diagnostics.

Output adapters.

Rust is particularly suitable for that layer.

I would use Rust for the entire headless production application, including the realtime graph engine. Python remains the research and training language. TypeScript remains the frontend language.

Recommended language ownership
￼
┌─────────────────────────────────────────────────────────┐
│ TypeScript frontend                                     │
│                                                         │
│ Setup, previews, diagnostics, avatar configuration       │
└────────────────────────┬────────────────────────────────┘
                         │
                 HTTP / WebSocket
                 OpenAPI / Protobuf
                         │
┌────────────────────────▼────────────────────────────────┐
│ Rust backend and realtime runtime                       │
│                                                         │
│ Sessions                                                │
│ Site calibration                                        │
│ Camera scheduling                                       │
│ Tracking graph                                          │
│ Geometry and fusion                                     │
│ Temporal reconstruction                                 │
│ Avatar retargeting                                      │
│ VMC / OSC / SteamVR outputs                             │
│ Recording, replay, metrics                              │
└────────────────────────┬────────────────────────────────┘
                         │
                   Narrow C ABI
                         │
┌────────────────────────▼────────────────────────────────┐
│ Native libraries and execution providers                │
│                                                         │
│ ONNX Runtime                                             │
│ Windows ML                                               │
│ CUDA / TensorRT                                          │
│ OpenVINO / MIGraphX                                      │
│ Media Foundation / V4L2 / GStreamer                     │
│ Vendor SDKs                                              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Python offline toolchain                                │
│                                                         │
│ Training                                                │
│ Synthetic data                                          │
│ Blender automation                                      │
│ Evaluation                                              │
│ Export to ONNX                                          │
│ Calibration fitting                                     │
└─────────────────────────────────────────────────────────┘
That is not “rewrite every native dependency in Rust.” It is Rust owns the product; foreign libraries sit behind small adapters.

Rust has explicit FFI support for C ABIs, and Cargo supports linking native static and dynamic libraries through build scripts. For difficult C++ interfaces, cxx provides a typed Rust–C++ bridge, while bindgen can generate low-level bindings to C and some C++ APIs. 
￼
Rust Programming Language
+3
￼
Rust Documentation
+3
￼
Rust Documentation
+3

ONNX Runtime does not force you to use C++
ONNX Runtime exposes a C API that supports session creation, model execution, provider registration, allocator configuration, shared thread pools, and custom operations. The execution-provider architecture is below the application language boundary: the same ONNX Runtime model can be partitioned across CPU, CUDA, TensorRT, OpenVINO, DirectML-class providers, and other supported providers. 
￼
ONNX Runtime
+1

The official documentation does not currently list Rust as a first-party binding with execution-provider API parity. Therefore, I would not let a third-party Rust wrapper define the system architecture. Instead:

￼
crates/inference-api
    Pure Rust task-level interface

crates/inference-ort
    Safe Rust implementation

crates/onnxruntime-sys
    Minimal generated/raw C binding

vendor/onnxruntime
    Versioned native runtime packages
The public Rust interface might look conceptually like:

Rust
￼
pub trait InferenceSession: Send {
    fn metadata(&self) -> &ModelMetadata;

    fn run(
        &mut self,
        request: InferenceRequest<'_>,
        outputs: &mut InferenceOutputs,
    ) -> Result<InferenceReport, InferenceError>;
}

pub trait InferenceBackend: Send + Sync {
    fn capabilities(&self) -> BackendCapabilities;

    fn compile(
        &self,
        model: &ModelPackage,
        profile: &InferenceProfile,
    ) -> Result<Box<dyn InferenceSession>, InferenceError>;
}
No ONNX Runtime pointer, CUDA stream, DirectML object, TensorRT context, or foreign tensor type escapes inference-ort.

The wrapper would own:

OrtEnv.

Sessions.

Session options.

Execution-provider registration.

Native memory handles.

Tensor lifetime.

Provider caches.

Thread-pool configuration.

Error translation.

Dynamic-library lifetime.

The rest of the application sees only canonical tensor descriptions and task outputs.

On Windows, Windows ML can dynamically obtain and register hardware-specific execution providers with ONNX Runtime. Rust can reach that through a small C/C++ or WinRT adapter without changing the rest of the runtime. 
￼
Microsoft Learn
+1

Do not require everything to be pure Rust
A hard “zero C++” rule would be counterproductive.

Use C or C++ where a dependency requires it:

￼
native/
├── onnxruntime_bridge/
├── windows_ml_bridge/
├── media_foundation_bridge/
├── vendor_camera_sdk/
├── steamvr_driver/
└── custom_gpu_ops/
Each native component should expose one of:

A small versioned C ABI.

A cxx bridge for tightly integrated first-party C++.

A separate process with Protobuf IPC for crash-prone or untrusted components.

Do not expose C++ STL types through the general plugin interface. Do not expose Rust trait objects through a binary plugin interface either. Rust’s native ABI is not a stable cross-version plugin contract; use extern "C", an IPC protocol, or a separately versioned executable boundary. 
￼
Rust Documentation
+1

A useful rule would be:

Rust by default. Native shim only when an external dependency requires one.

Keep asynchronous code out of the actual tracking hot path
Rust’s async ecosystem is useful for:

API requests.

Model downloads.

File access.

Frontend connections.

Session control.

Telemetry export.

Background compilation.

Device discovery.

It should not dictate the realtime tracking architecture.

Use dedicated worker threads for:

￼
camera capture
preprocessing
inference submission
multiview geometry
temporal reconstruction
avatar output
Use bounded channels between them.

The hot path should follow these constraints:

Preallocate frame and tensor pools.

Do not allocate per joint or per avatar bone.

Do not block on frontend or API activity.

Do not hold a global application mutex.

Do not place network I/O in the tracking loop.

Use source timestamps rather than arrival order.

Bound every queue.

Define an explicit drop policy.

Drop stale observations instead of building latency.

Warm models and providers before entering tracking mode.

Keep foreign-resource ownership inside RAII wrappers.

Catch panics at thread and FFI boundaries.

Never unwind through a non-unwinding C ABI boundary.

Rust’s FFI documentation specifically requires care around unwinding across foreign boundaries, so native adapters should convert failures to explicit status values before returning to Rust. 
￼
Rust Documentation
+1

Bazel: good capabilities, wrong starting point
Bazel would provide real benefits:

One build graph for Rust, Python, TypeScript, Protobuf, native libraries, and packaging.

Declared inputs and outputs.

Hermetic toolchains.

Cross-platform configuration.

Remote caching.

Remote execution.

Precise cross-language code generation dependencies.

Reproducible release artifacts.

Bazel’s remote cache can reuse reproducible action outputs across developers and CI machines, and its remote-execution model can provide consistent execution environments. 
￼
Bazel Documentation
+1

Those are legitimate advantages.

The problem is the cost of getting there.

The decisive issue: Rust-on-Windows support
Bazel itself supports Windows, including MSVC and Clang-based C++ toolchains. 
￼
Bazel Documentation

However, the current rules_rust documentation says its maintainers aim to support Linux and macOS and do not have sufficient maintainer expertise to properly support Windows; many Windows tests have been disabled. That is a poor foundation for a project whose normal contributors and users will heavily include Windows systems. 
￼
Bazel

That alone is enough for me to avoid Bazel as the initial source of truth for a Rust-first Windows/Linux application.

Every language adds another Bazel subsystem
A Bazel version would need at least:

￼
Bazel / Bzlmod
rules_rust
Crate Universe
rules_python
rules_js
rules_ts
protobuf rules
packaging rules
native toolchains
Windows platform definitions
Linux platform definitions
GPU-provider packaging rules
The components exist. rules_rust can import Cargo dependency graphs through Crate Universe, and the Rust rules include support for Protobuf, bindgen, Clippy, rustfmt, and rust-analyzer setup. 
￼
Bazel
+3
￼
Bazel
+3
￼
Bazel
+3

But maintaining those rule layers becomes a project of its own.

JavaScript support is not native “just run pnpm” behavior. rules_js deliberately recreates a pnpm-style dependency layout under Bazel’s output tree and documents trade-offs caused by Bazel’s filesystem model. 
￼
GitHub

Bazel is also completing its transition to Bzlmod and externally maintained language rules. Those are sensible long-term changes, but they mean the surrounding ecosystem has recently carried substantial transition work. 
￼
Bazel Documentation

For a small open-source project, this risks spending more time maintaining build infrastructure than improving tracking quality.

That conflicts with the principles in your existing project guidance: use the smallest clear implementation, add abstraction only for an existing reader or measurable complexity reduction, and keep root commands limited to operations used constantly on maintained paths. ￼
Pasted markdown +1


Recommended build arrangement
Use native tools as the source of truth for their own ecosystems.

￼
repo/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
│
├── crates/
├── apps/
│
├── frontend/
│   ├── package.json
│   ├── pnpm-lock.yaml
│   └── ...
│
├── ml/
│   ├── pyproject.toml
│   ├── uv.lock
│   └── ...
│
├── contracts/
│   ├── proto/
│   ├── openapi/
│   └── buf.yaml
│
├── native/
├── models/
├── packaging/
├── tools/
├── justfile
└── .github/workflows/
Cargo workspaces are explicitly designed to manage related packages together, sharing one lockfile and one output directory while allowing workspace-wide commands. 
￼
Rust Documentation
+1

The root command surface should remain small:

Makefile
￼
just bootstrap
just generate
just check
just test
just build
just run
just package
Internally:

￼
just check
    cargo fmt --check
    cargo clippy --workspace
    cargo test --workspace
    uv run ruff check
    uv run pytest
    pnpm lint
    pnpm test
    contract compatibility checks
Each ecosystem still owns its dependency graph:

￼
Rust        Cargo.lock
Python      uv.lock
Frontend    pnpm-lock.yaml
Contracts   pinned generators
Native      pinned binary manifests or source revisions
Models      signed model manifests and hashes
This is less hermetic than a carefully engineered Bazel build, but it is substantially easier for ordinary Rust, Python, and frontend contributors to understand.

Reproducibility can still be strong:

Pin Rust with rust-toolchain.toml.

Pin Python and dependencies in uv.lock.

Pin Node and pnpm versions.

Pin code generators.

Build releases in fixed Windows and Linux CI images.

Record native dependency checksums.

Publish model and runtime manifests.

Test clean builds in CI.

Produce releases only from clean tagged commits.

When Bazel would become worth adopting
Revisit Bazel when the project demonstrates actual pain in at least several of these areas:

Cross-language generation regularly becomes inconsistent.

Full clean builds become materially expensive.

CI repeatedly rebuilds identical native or generated artifacts.

The contributor team becomes large enough that environment drift costs real time.

Many binaries share a large native dependency graph.

Several platform and GPU permutations must be built from the same source graph.

Reproducible source builds become a release requirement.

Remote caching would save more maintainer time than the Bazel configuration consumes.

Packaging native provider combinations becomes unmanageable.

The project has someone willing to own Windows Bazel support.

Do not maintain Cargo and Bazel as two equally supported production build systems. That creates two dependency graphs, two feature-resolution systems, and two sets of linker behavior.

When Bazel wins, promote it by replacement.

That follows the same “winning experiment replaces the old path” rule already present in your engineering philosophy. ￼
Pasted markdown


Proposed Rust repository boundary
I would revise the runtime part of the architecture to approximately this:

￼
apps/
├── tracker-daemon/
├── tracker-cli/
└── replay-runner/

crates/
├── api-contracts/
├── domain/
├── coordinates/
├── timebase/
├── packets/
│
├── graph/
├── scheduler/
├── diagnostics/
├── recording/
├── replay/
│
├── capture-api/
├── capture-windows/
├── capture-linux/
│
├── inference-api/
├── inference-ort/
├── model-package/
│
├── body-observer/
├── face-observer/
├── hand-observer/
├── prop-observer/
│
├── calibration/
├── synchronization/
├── multiview-geometry/
├── fusion/
├── temporal/
├── contacts/
│
├── avatar-import/
├── avatar-profile/
├── retarget/
│
├── output-api/
├── output-vmc/
├── output-osc/
└── output-steamvr/

native/
├── onnxruntime-sys/
├── windows-ml/
├── vendor-camera/
└── steamvr-driver/

ml/
├── training/
├── synthetic/
├── evaluation/
├── export/
└── model-registry/

frontend/
contracts/
packaging/
Do not make every crate an interface crate. Split a crate only when there is:

A real platform boundary.

A separate state lifetime.

An external dependency boundary.

More than one maintained implementation.

A replay boundary.

A compile-time dependency that should not spread.

A measurable build or ownership advantage.

For example, capture-windows and capture-linux deserve a boundary. A separate left_ankle_confidence_utils crate does not.

Final decision
I would update the architecture this way:

￼
Primary runtime language:
Rust 2024

Primary runtime build:
Cargo workspace

Backend:
Headless Rust daemon

Frontend:
Independent TypeScript client

Training and synthetic generation:
Python

Inference:
ONNX Runtime C API behind a safe Rust adapter

GPU neutrality:
Runtime-selected execution providers

Platform support:
Native Windows and Linux CI

Native escape hatch:
Small C/C++ adapters only where required

Root command runner:
just

Bazel:
Deferred until measured repository-scale problems justify it
Rust is probably a better default than C++ for this project. Bazel is probably not a better default than Cargo for it—not yet.

The combination I would choose is Rust-first, Cargo-native, narrow FFI boundaries, and a deliberately boring root build interface. That gives you the modularity and safety benefits you want without turning build engineering into the project’s first major subsystem.

￼
Sources
￼
￼
￼
￼
￼

You said:
updated architecture MD please
￼
￼
￼

ChatGPT said:
￼Worked for 14m 18s
￼Download the updated Rust-first repository and API architecture

Revision 1.2 now specifies:

Rust 2024 for all first-party runtime and backend code.

No first-party C or C++ source. Native dependencies may exist internally, but they are accessed through stable C APIs or isolated processes.

Cargo workspace as the authoritative Rust build system.

Bazel deferred unless measured build, reproducibility, or remote-execution problems justify replacing Cargo.

just as the small cross-project command surface.

A strict process-level split between the TypeScript frontend and the headless Rust backend.

Tauri as an optional desktop shell, without allowing its Rust shell code to import backend or engine crates.

Tokio for control-plane networking and asynchronous administration.

Dedicated threads, bounded queues, preallocated buffers, and deadline-aware scheduling for the realtime tracking path.

Safe Rust throughout domain and tracking crates, with unsafe confined to small audited FFI, OS, SIMD, or GPU adapters.

ONNX Runtime behind a safe Rust inference interface using its stable C API.

OS- and GPU-neutral provider selection across Windows and Linux.

No Rust ABI or C++ ABI for plugins; extensions use versioned Protobuf IPC or an isolated worker process.

Rust-specific crate boundaries, ownership rules, linting, formatting, testing, dependency policy, CI, packaging, and replay architecture.

The existing rules around minimal production code, replacing rather than retaining obsolete paths, isolated experiments, and outcome-based tests. ￼
Pasted markdown


The existing documentation and repository-boundary philosophy, adapted to the Rust/Python/TypeScript project. ￼
Pasted markdown +1


Rust-only architecture revision
￼Download Revision 1.2 — Rust-first repository and API architecture

The maintained architecture now uses:

￼
Realtime engine and backend:      Rust 2024
Daemon and application API:       Rust
Camera and output adapters:       Rust
Inference orchestration:          Rust
CLI, replay, benchmark tools:     Rust
Plugin host and model worker:     Rust
Training and synthetic data:      Python
Replaceable frontend:             TypeScript
Rust build system:                Cargo workspace
Cross-project command surface:    just
Repository automation:            cargo xtask
Bazel:                             deferred
First-party C++:                   prohibited
Build-system decision
I changed the recommendation from CMake—and declined Bazel for the initial maintained path.

Cargo should own the Rust graph. Cargo workspaces are explicitly designed to manage multiple related Rust packages together, including coordinated dependency resolution and a shared lockfile/build directory. 
￼
Rust Documentation
+1

The overall build structure is:

￼
just
├── cargo       Rust runtime, daemon, CLI, adapters, tests
├── uv          Python ML, data generation, training, export
└── pnpm        TypeScript frontend and generated client
Bazel can build Rust through rules_rust, and the current rules include support for facilities such as Prost, bindgen, PyO3, and WebAssembly bindings. However, it would introduce BUILD files, Bazel toolchains, Cargo-to-Bazel dependency translation, and a second representation of the Rust dependency graph. 
￼
GitHub
+1

That conflicts with the engineering philosophy in your supplied document: use the smallest clear implementation, add a layer only when it currently reduces production complexity, and avoid dependencies or abstractions without a current reader. ￼
Pasted markdown


The revised document allows reconsidering Bazel only after a measured problem exists, such as:

Remote caching would materially reduce CI or release time.

Cross-language generated artifacts repeatedly drift.

The repository becomes large enough that path-aware Cargo, Python, and frontend jobs are inadequate.

A supported deployment target requires a hermetic cross-compilation graph Cargo cannot maintain cleanly.

Rust runtime structure
The old cpp/ tree has been replaced by a Cargo workspace:

￼
crates/
├── foundation/
├── domain/
├── engine/
├── application/
├── ports/
├── modules/
├── adapters/
├── ffi/
└── composition/
The important rule is that this is an ownership map, not permission to create fifty empty crates. A crate is added only when it provides a current dependency boundary, platform boundary, shipped artifact, or reusable implementation.

All conceptual C++ interfaces have been rewritten as Rust traits, including:

Node

CameraSource

InferenceBackend

ModelRunner

TaskModule

AvatarImporter

BodyFusion

TemporalPerformanceEstimator

AvatarRetargeter

OutputAdapter

Repository and application-service ports

Traits are still subject to the abstraction budget. A private implementation with one caller remains a concrete type rather than receiving a speculative trait.

Async versus realtime execution
The revised design uses Tokio for the control plane, not as an undifferentiated scheduler for the entire tracker.

Tokio is appropriate for HTTP, WebSockets, process supervision, timers, operations, and background I/O. It provides asynchronous I/O, scheduling, timers, and synchronization facilities for those workloads. 
￼
Tokio
+1

The latency-critical domains remain separate:

￼
Tokio control plane
├── HTTP API
├── WebSocket clients
├── operation progress
├── process supervision
└── background I/O

Dedicated realtime domains
├── camera capture
├── preprocessing
├── GPU submission
├── fusion
├── temporal output tick
├── retargeting
└── VMC / OSC output
Camera capture, GPU ownership, temporal output, and deadline-critical stages do not run as arbitrary tasks on Tokio’s general worker pool. They use dedicated threads or explicitly sized executors connected through bounded channels.

Public API implementation
The clean frontend/backend split remains unchanged.

The reference Rust transport stack is now:

￼
OpenAPI
    ↓
Axum HTTP handlers
    ↓
Application services
    ↓
Realtime engine ports

Protobuf
    ↓
Prost-generated Rust messages
    ↓
Axum binary WebSocket frames
Axum provides modular HTTP request handling and WebSocket support. Prost generates idiomatic Rust types from Protocol Buffer schemas. 
￼
GitHub
+3
￼
GitHub
+3
￼
Docs.rs
+3

The frontend still has no direct dependency on backend crates. The Tauri host is deliberately thin: it starts the daemon, manages the window and system integration, and communicates through the same public API available to any other client.

ONNX Runtime from Rust
ONNX Runtime remains the default cross-platform inference host. Its execution-provider system is specifically intended to map models onto different hardware acceleration libraries behind a common runtime interface. 
￼
ONNX Runtime

Rust support is currently presented as a community API, rather than one of ONNX Runtime’s primary official language APIs. 
￼
ONNX Runtime
+1

The revised architecture therefore makes this boundary explicit:

￼
Tracking module
    ↓
ModelRunner trait
    ↓
InferenceBackend trait
    ↓
avatartrack-inference-ort
    ↓
pinned ort Rust crate
    ↓
ONNX Runtime C API
    ↓
CUDA / TensorRT / OpenVINO / DirectML / CPU
The ort crate is confined to one adapter. It already exposes Rust interfaces for ONNX Runtime sessions and execution providers, but it does not become a domain-level dependency. 
￼
GitHub
+2
￼
Docs.rs
+2

Should that wrapper block a required provider or lifetime model, only the adapter is replaced with a small internal Rust FFI crate against ONNX Runtime’s stable C API.

What “no C++” means
The project now has:

No first-party .cpp, .cc, .cxx, .hpp, or CMake files.

No C++ domain model.

No C++ daemon.

No C++ plugin API.

No Rust-to-C++ object binding layer.

No C++ compilation target maintained by the project.

External native dependencies can still contain C or C++ internally. ONNX Runtime, GStreamer, operating-system media libraries, and GPU drivers are examples. The Rust code reaches them through narrow FFI or safe binding crates. Rust’s official documentation supports using C-compatible interfaces for foreign libraries, with unsafe behavior contained at the boundary. 
￼
Rust Documentation
+1

GStreamer has maintained Rust bindings that expose a safe Rust API over the underlying native library, making it a reasonable first broad camera/media adapter while still allowing direct Media Foundation or V4L2 adapters where measurements justify them. 
￼
GStreamer
+1

Rust safety and latency rules added
The document now requires:

Safe Rust by default.

unsafe only in small FFI or platform-adapter modules.

A SAFETY: invariant on every unsafe block.

Raw pointers and foreign handles never leaving their owning adapter.

No panic unwinding through FFI.

Typed errors within libraries.

No unwrap, expect, unchecked indexing, or unchecked conversions on external-data paths.

No broad Arc<Mutex<_>> object graph.

One owner for mutable realtime state.

Bounded message passing between stages.

Preallocated frame, packet, tensor-binding, and output storage where profiling justifies it.

No blocking I/O, database work, filesystem access, or log formatting on the final output tick.

No Rust ABI or trait objects across plugin boundaries.

Out-of-process Protobuf IPC for community plugins by default.

Rust ownership provides memory management without a garbage collector, which fits a latency-sensitive native pipeline, but it does not automatically solve scheduling or allocation jitter. Those still require the explicit realtime-domain and buffer-lifetime design described in the revision. 
￼
Rust Documentation
+1

The resulting decision is clean: Rust is the product runtime language; Python remains the ML research language; TypeScript remains the replaceable UI language; Cargo remains the Rust source of truth.

￼
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Rust_Revision_1_2.md
Document
￼
￼
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Revision_1_2_Rust.md
Document
￼
￼
Sources
￼
￼
￼
￼
￼
ChatGPT can make mistakes. Check important info.
￼
￼

￼
Pro
￼
￼
￼￼Choose FilesNo file chosen￼￼Choose FilesNo file chosen
Library
/
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Rust_Revision_1_2.md
￼
￼
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
46
47
48
49
50
51
52
53
54
55
56
57
58
59
60
61
62
63
64
65
66
67
68
69
70
71
72
73
74
75
76
77
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

ChatGPT said:
￼Worked for 122m 11s
Comprehensive design document
￼Download the editable DOCX

The document includes:

The complete product thesis and target user experience.

One-camera, two-camera, and scalable multi-camera operating modes.

Human-pose-based automatic camera calibration and synchronization.

Body, face, eye, lip, and hand-tracking architecture.

VRChat, SteamVR, OSC, VRM, and VMC integration strategies.

Avatar-aware retargeting for highly stylized proportions.

Confidence-aware fusion, contact locking, temporal prediction, interpolation, and failure containment.

Realistic inference-rate and hardware-scaling plans.

A synthetic-data, public-dataset, teacher–student, and self-supervised training strategy.

A staged implementation roadmap suitable for a free-time open-source project.

Quantitative and perceptual evaluation plans.

A detailed grievance-to-design-response matrix.

A comprehensive market sweep covering free webcam tracking, multi-camera applications, software estimation, IMUs, Lighthouse trackers, inside-out trackers, depth cameras, inertial suits, cloud mocap, professional optical systems, face tracking, and hand tracking.

Clickable references to official product documentation, pricing pages, research papers, datasets, support documentation, Steam pages, and clearly labeled anecdotal community evidence.

￼
Commodity_Camera_Avatar_Tracking_Design.docx
Document
￼
￼
￼
￼
￼
￼

You said:
￼
Pasted markdown.md
File
Here is an agent.md that I wrote for a differnent project, granted it's target is a bit different, but it shoudl still give you an idea what the type of structure and philosphy I like. Modify the API modular architecture document with info from this: 
￼

ChatGPT said:
The key tradeoff is reproducibility and repo-wide dependency enforcement versus forcing contributors to learn and maintain an additional packaging layer over the native language ecosystems.

￼
￼
Searched 5 websites
￼
￼
Searched 3 websites
￼
Inspected architecture notes for build-system guidance
I’m folding Rust into that decision too: a Rust-heavy runtime strengthens Bazel’s case somewhat, but GPU/inference ecosystem maturity matters more than language purity.

￼
￼
Searched 1 website

You said:
lowkey wanna just replace cpp with rust, no cpp, rust please!!! 
￼
￼
￼

ChatGPT said:
￼Worked for 10m 28s
Recommendation
Use Rust instead of C++ for the maintained runtime and backend. Do not use Bazel as the primary build system initially.

Use:

￼
Rust/Cargo workspace     Runtime, backend, calibration, fusion, retargeting
Python/uv                Training, evaluation, synthetic-data generation
TypeScript/pnpm          Frontend
Protobuf/OpenAPI         Public contracts
just                     Small root command surface
C/C++                    Narrow native compatibility shims only
Bazel is a good theoretical match for the final monorepo, but it is a poor practical match for the first several versions of this project—especially when native Windows support is mandatory and Rust becomes the primary runtime language.

Rust is a good fit
Nothing in this runtime inherently requires C++.

The expensive numerical work should occur inside:

ONNX Runtime execution providers.

Vendor GPU libraries.

Image-processing kernels.

Precompiled inference models.

Carefully bounded geometry kernels.

The maintained application code is primarily:

Camera scheduling.

Frame synchronization.

Session and calibration state.

Typed packet routing.

Confidence propagation.

Temporal state management.

Multiview fusion orchestration.

Retargeting.

API serving.

Recording and replay.

Diagnostics.

Output adapters.

Rust is particularly suitable for that layer.

I would use Rust for the entire headless production application, including the realtime graph engine. Python remains the research and training language. TypeScript remains the frontend language.

Recommended language ownership
￼
┌─────────────────────────────────────────────────────────┐
│ TypeScript frontend                                     │
│                                                         │
│ Setup, previews, diagnostics, avatar configuration       │
└────────────────────────┬────────────────────────────────┘
                         │
                 HTTP / WebSocket
                 OpenAPI / Protobuf
                         │
┌────────────────────────▼────────────────────────────────┐
│ Rust backend and realtime runtime                       │
│                                                         │
│ Sessions                                                │
│ Site calibration                                        │
│ Camera scheduling                                       │
│ Tracking graph                                          │
│ Geometry and fusion                                     │
│ Temporal reconstruction                                 │
│ Avatar retargeting                                      │
│ VMC / OSC / SteamVR outputs                             │
│ Recording, replay, metrics                              │
└────────────────────────┬────────────────────────────────┘
                         │
                   Narrow C ABI
                         │
┌────────────────────────▼────────────────────────────────┐
│ Native libraries and execution providers                │
│                                                         │
│ ONNX Runtime                                             │
│ Windows ML                                               │
│ CUDA / TensorRT                                          │
│ OpenVINO / MIGraphX                                      │
│ Media Foundation / V4L2 / GStreamer                     │
│ Vendor SDKs                                              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│ Python offline toolchain                                │
│                                                         │
│ Training                                                │
│ Synthetic data                                          │
│ Blender automation                                      │
│ Evaluation                                              │
│ Export to ONNX                                          │
│ Calibration fitting                                     │
└─────────────────────────────────────────────────────────┘
That is not “rewrite every native dependency in Rust.” It is Rust owns the product; foreign libraries sit behind small adapters.

Rust has explicit FFI support for C ABIs, and Cargo supports linking native static and dynamic libraries through build scripts. For difficult C++ interfaces, cxx provides a typed Rust–C++ bridge, while bindgen can generate low-level bindings to C and some C++ APIs. 
￼
Rust Programming Language
+3
￼
Rust Documentation
+3
￼
Rust Documentation
+3

ONNX Runtime does not force you to use C++
ONNX Runtime exposes a C API that supports session creation, model execution, provider registration, allocator configuration, shared thread pools, and custom operations. The execution-provider architecture is below the application language boundary: the same ONNX Runtime model can be partitioned across CPU, CUDA, TensorRT, OpenVINO, DirectML-class providers, and other supported providers. 
￼
ONNX Runtime
+1

The official documentation does not currently list Rust as a first-party binding with execution-provider API parity. Therefore, I would not let a third-party Rust wrapper define the system architecture. Instead:

￼
crates/inference-api
    Pure Rust task-level interface

crates/inference-ort
    Safe Rust implementation

crates/onnxruntime-sys
    Minimal generated/raw C binding

vendor/onnxruntime
    Versioned native runtime packages
The public Rust interface might look conceptually like:

Rust
￼
pub trait InferenceSession: Send {
    fn metadata(&self) -> &ModelMetadata;

    fn run(
        &mut self,
        request: InferenceRequest<'_>,
        outputs: &mut InferenceOutputs,
    ) -> Result<InferenceReport, InferenceError>;
}

pub trait InferenceBackend: Send + Sync {
    fn capabilities(&self) -> BackendCapabilities;

    fn compile(
        &self,
        model: &ModelPackage,
        profile: &InferenceProfile,
    ) -> Result<Box<dyn InferenceSession>, InferenceError>;
}
No ONNX Runtime pointer, CUDA stream, DirectML object, TensorRT context, or foreign tensor type escapes inference-ort.

The wrapper would own:

OrtEnv.

Sessions.

Session options.

Execution-provider registration.

Native memory handles.

Tensor lifetime.

Provider caches.

Thread-pool configuration.

Error translation.

Dynamic-library lifetime.

The rest of the application sees only canonical tensor descriptions and task outputs.

On Windows, Windows ML can dynamically obtain and register hardware-specific execution providers with ONNX Runtime. Rust can reach that through a small C/C++ or WinRT adapter without changing the rest of the runtime. 
￼
Microsoft Learn
+1

Do not require everything to be pure Rust
A hard “zero C++” rule would be counterproductive.

Use C or C++ where a dependency requires it:

￼
native/
├── onnxruntime_bridge/
├── windows_ml_bridge/
├── media_foundation_bridge/
├── vendor_camera_sdk/
├── steamvr_driver/
└── custom_gpu_ops/
Each native component should expose one of:

A small versioned C ABI.

A cxx bridge for tightly integrated first-party C++.

A separate process with Protobuf IPC for crash-prone or untrusted components.

Do not expose C++ STL types through the general plugin interface. Do not expose Rust trait objects through a binary plugin interface either. Rust’s native ABI is not a stable cross-version plugin contract; use extern "C", an IPC protocol, or a separately versioned executable boundary. 
￼
Rust Documentation
+1

A useful rule would be:

Rust by default. Native shim only when an external dependency requires one.

Keep asynchronous code out of the actual tracking hot path
Rust’s async ecosystem is useful for:

API requests.

Model downloads.

File access.

Frontend connections.

Session control.

Telemetry export.

Background compilation.

Device discovery.

It should not dictate the realtime tracking architecture.

Use dedicated worker threads for:

￼
camera capture
preprocessing
inference submission
multiview geometry
temporal reconstruction
avatar output
Use bounded channels between them.

The hot path should follow these constraints:

Preallocate frame and tensor pools.

Do not allocate per joint or per avatar bone.

Do not block on frontend or API activity.

Do not hold a global application mutex.

Do not place network I/O in the tracking loop.

Use source timestamps rather than arrival order.

Bound every queue.

Define an explicit drop policy.

Drop stale observations instead of building latency.

Warm models and providers before entering tracking mode.

Keep foreign-resource ownership inside RAII wrappers.

Catch panics at thread and FFI boundaries.

Never unwind through a non-unwinding C ABI boundary.

Rust’s FFI documentation specifically requires care around unwinding across foreign boundaries, so native adapters should convert failures to explicit status values before returning to Rust. 
￼
Rust Documentation
+1

Bazel: good capabilities, wrong starting point
Bazel would provide real benefits:

One build graph for Rust, Python, TypeScript, Protobuf, native libraries, and packaging.

Declared inputs and outputs.

Hermetic toolchains.

Cross-platform configuration.

Remote caching.

Remote execution.

Precise cross-language code generation dependencies.

Reproducible release artifacts.

Bazel’s remote cache can reuse reproducible action outputs across developers and CI machines, and its remote-execution model can provide consistent execution environments. 
￼
Bazel Documentation
+1

Those are legitimate advantages.

The problem is the cost of getting there.

The decisive issue: Rust-on-Windows support
Bazel itself supports Windows, including MSVC and Clang-based C++ toolchains. 
￼
Bazel Documentation

However, the current rules_rust documentation says its maintainers aim to support Linux and macOS and do not have sufficient maintainer expertise to properly support Windows; many Windows tests have been disabled. That is a poor foundation for a project whose normal contributors and users will heavily include Windows systems. 
￼
Bazel

That alone is enough for me to avoid Bazel as the initial source of truth for a Rust-first Windows/Linux application.

Every language adds another Bazel subsystem
A Bazel version would need at least:

￼
Bazel / Bzlmod
rules_rust
Crate Universe
rules_python
rules_js
rules_ts
protobuf rules
packaging rules
native toolchains
Windows platform definitions
Linux platform definitions
GPU-provider packaging rules
The components exist. rules_rust can import Cargo dependency graphs through Crate Universe, and the Rust rules include support for Protobuf, bindgen, Clippy, rustfmt, and rust-analyzer setup. 
￼
Bazel
+3
￼
Bazel
+3
￼
Bazel
+3

But maintaining those rule layers becomes a project of its own.

JavaScript support is not native “just run pnpm” behavior. rules_js deliberately recreates a pnpm-style dependency layout under Bazel’s output tree and documents trade-offs caused by Bazel’s filesystem model. 
￼
GitHub

Bazel is also completing its transition to Bzlmod and externally maintained language rules. Those are sensible long-term changes, but they mean the surrounding ecosystem has recently carried substantial transition work. 
￼
Bazel Documentation

For a small open-source project, this risks spending more time maintaining build infrastructure than improving tracking quality.

That conflicts with the principles in your existing project guidance: use the smallest clear implementation, add abstraction only for an existing reader or measurable complexity reduction, and keep root commands limited to operations used constantly on maintained paths. ￼
Pasted markdown +1


Recommended build arrangement
Use native tools as the source of truth for their own ecosystems.

￼
repo/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
│
├── crates/
├── apps/
│
├── frontend/
│   ├── package.json
│   ├── pnpm-lock.yaml
│   └── ...
│
├── ml/
│   ├── pyproject.toml
│   ├── uv.lock
│   └── ...
│
├── contracts/
│   ├── proto/
│   ├── openapi/
│   └── buf.yaml
│
├── native/
├── models/
├── packaging/
├── tools/
├── justfile
└── .github/workflows/
Cargo workspaces are explicitly designed to manage related packages together, sharing one lockfile and one output directory while allowing workspace-wide commands. 
￼
Rust Documentation
+1

The root command surface should remain small:

Makefile
￼
just bootstrap
just generate
just check
just test
just build
just run
just package
Internally:

￼
just check
    cargo fmt --check
    cargo clippy --workspace
    cargo test --workspace
    uv run ruff check
    uv run pytest
    pnpm lint
    pnpm test
    contract compatibility checks
Each ecosystem still owns its dependency graph:

￼
Rust        Cargo.lock
Python      uv.lock
Frontend    pnpm-lock.yaml
Contracts   pinned generators
Native      pinned binary manifests or source revisions
Models      signed model manifests and hashes
This is less hermetic than a carefully engineered Bazel build, but it is substantially easier for ordinary Rust, Python, and frontend contributors to understand.

Reproducibility can still be strong:

Pin Rust with rust-toolchain.toml.

Pin Python and dependencies in uv.lock.

Pin Node and pnpm versions.

Pin code generators.

Build releases in fixed Windows and Linux CI images.

Record native dependency checksums.

Publish model and runtime manifests.

Test clean builds in CI.

Produce releases only from clean tagged commits.

When Bazel would become worth adopting
Revisit Bazel when the project demonstrates actual pain in at least several of these areas:

Cross-language generation regularly becomes inconsistent.

Full clean builds become materially expensive.

CI repeatedly rebuilds identical native or generated artifacts.

The contributor team becomes large enough that environment drift costs real time.

Many binaries share a large native dependency graph.

Several platform and GPU permutations must be built from the same source graph.

Reproducible source builds become a release requirement.

Remote caching would save more maintainer time than the Bazel configuration consumes.

Packaging native provider combinations becomes unmanageable.

The project has someone willing to own Windows Bazel support.

Do not maintain Cargo and Bazel as two equally supported production build systems. That creates two dependency graphs, two feature-resolution systems, and two sets of linker behavior.

When Bazel wins, promote it by replacement.

That follows the same “winning experiment replaces the old path” rule already present in your engineering philosophy. ￼
Pasted markdown


Proposed Rust repository boundary
I would revise the runtime part of the architecture to approximately this:

￼
apps/
├── tracker-daemon/
├── tracker-cli/
└── replay-runner/

crates/
├── api-contracts/
├── domain/
├── coordinates/
├── timebase/
├── packets/
│
├── graph/
├── scheduler/
├── diagnostics/
├── recording/
├── replay/
│
├── capture-api/
├── capture-windows/
├── capture-linux/
│
├── inference-api/
├── inference-ort/
├── model-package/
│
├── body-observer/
├── face-observer/
├── hand-observer/
├── prop-observer/
│
├── calibration/
├── synchronization/
├── multiview-geometry/
├── fusion/
├── temporal/
├── contacts/
│
├── avatar-import/
├── avatar-profile/
├── retarget/
│
├── output-api/
├── output-vmc/
├── output-osc/
└── output-steamvr/

native/
├── onnxruntime-sys/
├── windows-ml/
├── vendor-camera/
└── steamvr-driver/

ml/
├── training/
├── synthetic/
├── evaluation/
├── export/
└── model-registry/

frontend/
contracts/
packaging/
Do not make every crate an interface crate. Split a crate only when there is:

A real platform boundary.

A separate state lifetime.

An external dependency boundary.

More than one maintained implementation.

A replay boundary.

A compile-time dependency that should not spread.

A measurable build or ownership advantage.

For example, capture-windows and capture-linux deserve a boundary. A separate left_ankle_confidence_utils crate does not.

Final decision
I would update the architecture this way:

￼
Primary runtime language:
Rust 2024

Primary runtime build:
Cargo workspace

Backend:
Headless Rust daemon

Frontend:
Independent TypeScript client

Training and synthetic generation:
Python

Inference:
ONNX Runtime C API behind a safe Rust adapter

GPU neutrality:
Runtime-selected execution providers

Platform support:
Native Windows and Linux CI

Native escape hatch:
Small C/C++ adapters only where required

Root command runner:
just

Bazel:
Deferred until measured repository-scale problems justify it
Rust is probably a better default than C++ for this project. Bazel is probably not a better default than Cargo for it—not yet.

The combination I would choose is Rust-first, Cargo-native, narrow FFI boundaries, and a deliberately boring root build interface. That gives you the modularity and safety benefits you want without turning build engineering into the project’s first major subsystem.

￼
Sources
￼
￼
￼
￼
￼

You said:
updated architecture MD please
￼
￼
￼

ChatGPT said:
￼Worked for 14m 18s
￼Download the updated Rust-first repository and API architecture

Revision 1.2 now specifies:

Rust 2024 for all first-party runtime and backend code.

No first-party C or C++ source. Native dependencies may exist internally, but they are accessed through stable C APIs or isolated processes.

Cargo workspace as the authoritative Rust build system.

Bazel deferred unless measured build, reproducibility, or remote-execution problems justify replacing Cargo.

just as the small cross-project command surface.

A strict process-level split between the TypeScript frontend and the headless Rust backend.

Tauri as an optional desktop shell, without allowing its Rust shell code to import backend or engine crates.

Tokio for control-plane networking and asynchronous administration.

Dedicated threads, bounded queues, preallocated buffers, and deadline-aware scheduling for the realtime tracking path.

Safe Rust throughout domain and tracking crates, with unsafe confined to small audited FFI, OS, SIMD, or GPU adapters.

ONNX Runtime behind a safe Rust inference interface using its stable C API.

OS- and GPU-neutral provider selection across Windows and Linux.

No Rust ABI or C++ ABI for plugins; extensions use versioned Protobuf IPC or an isolated worker process.

Rust-specific crate boundaries, ownership rules, linting, formatting, testing, dependency policy, CI, packaging, and replay architecture.

The existing rules around minimal production code, replacing rather than retaining obsolete paths, isolated experiments, and outcome-based tests. ￼
Pasted markdown


The existing documentation and repository-boundary philosophy, adapted to the Rust/Python/TypeScript project. ￼
Pasted markdown +1


Rust-only architecture revision
￼Download Revision 1.2 — Rust-first repository and API architecture

The maintained architecture now uses:

￼
Realtime engine and backend:      Rust 2024
Daemon and application API:       Rust
Camera and output adapters:       Rust
Inference orchestration:          Rust
CLI, replay, benchmark tools:     Rust
Plugin host and model worker:     Rust
Training and synthetic data:      Python
Replaceable frontend:             TypeScript
Rust build system:                Cargo workspace
Cross-project command surface:    just
Repository automation:            cargo xtask
Bazel:                             deferred
First-party C++:                   prohibited
Build-system decision
I changed the recommendation from CMake—and declined Bazel for the initial maintained path.

Cargo should own the Rust graph. Cargo workspaces are explicitly designed to manage multiple related Rust packages together, including coordinated dependency resolution and a shared lockfile/build directory. 
￼
Rust Documentation
+1

The overall build structure is:

￼
just
├── cargo       Rust runtime, daemon, CLI, adapters, tests
├── uv          Python ML, data generation, training, export
└── pnpm        TypeScript frontend and generated client
Bazel can build Rust through rules_rust, and the current rules include support for facilities such as Prost, bindgen, PyO3, and WebAssembly bindings. However, it would introduce BUILD files, Bazel toolchains, Cargo-to-Bazel dependency translation, and a second representation of the Rust dependency graph. 
￼
GitHub
+1

That conflicts with the engineering philosophy in your supplied document: use the smallest clear implementation, add a layer only when it currently reduces production complexity, and avoid dependencies or abstractions without a current reader. ￼
Pasted markdown


The revised document allows reconsidering Bazel only after a measured problem exists, such as:

Remote caching would materially reduce CI or release time.

Cross-language generated artifacts repeatedly drift.

The repository becomes large enough that path-aware Cargo, Python, and frontend jobs are inadequate.

A supported deployment target requires a hermetic cross-compilation graph Cargo cannot maintain cleanly.

Rust runtime structure
The old cpp/ tree has been replaced by a Cargo workspace:

￼
crates/
├── foundation/
├── domain/
├── engine/
├── application/
├── ports/
├── modules/
├── adapters/
├── ffi/
└── composition/
The important rule is that this is an ownership map, not permission to create fifty empty crates. A crate is added only when it provides a current dependency boundary, platform boundary, shipped artifact, or reusable implementation.

All conceptual C++ interfaces have been rewritten as Rust traits, including:

Node

CameraSource

InferenceBackend

ModelRunner

TaskModule

AvatarImporter

BodyFusion

TemporalPerformanceEstimator

AvatarRetargeter

OutputAdapter

Repository and application-service ports

Traits are still subject to the abstraction budget. A private implementation with one caller remains a concrete type rather than receiving a speculative trait.

Async versus realtime execution
The revised design uses Tokio for the control plane, not as an undifferentiated scheduler for the entire tracker.

Tokio is appropriate for HTTP, WebSockets, process supervision, timers, operations, and background I/O. It provides asynchronous I/O, scheduling, timers, and synchronization facilities for those workloads. 
￼
Tokio
+1

The latency-critical domains remain separate:

￼
Tokio control plane
├── HTTP API
├── WebSocket clients
├── operation progress
├── process supervision
└── background I/O

Dedicated realtime domains
├── camera capture
├── preprocessing
├── GPU submission
├── fusion
├── temporal output tick
├── retargeting
└── VMC / OSC output
Camera capture, GPU ownership, temporal output, and deadline-critical stages do not run as arbitrary tasks on Tokio’s general worker pool. They use dedicated threads or explicitly sized executors connected through bounded channels.

Public API implementation
The clean frontend/backend split remains unchanged.

The reference Rust transport stack is now:

￼
OpenAPI
    ↓
Axum HTTP handlers
    ↓
Application services
    ↓
Realtime engine ports

Protobuf
    ↓
Prost-generated Rust messages
    ↓
Axum binary WebSocket frames
Axum provides modular HTTP request handling and WebSocket support. Prost generates idiomatic Rust types from Protocol Buffer schemas. 
￼
GitHub
+3
￼
GitHub
+3
￼
Docs.rs
+3

The frontend still has no direct dependency on backend crates. The Tauri host is deliberately thin: it starts the daemon, manages the window and system integration, and communicates through the same public API available to any other client.

ONNX Runtime from Rust
ONNX Runtime remains the default cross-platform inference host. Its execution-provider system is specifically intended to map models onto different hardware acceleration libraries behind a common runtime interface. 
￼
ONNX Runtime

Rust support is currently presented as a community API, rather than one of ONNX Runtime’s primary official language APIs. 
￼
ONNX Runtime
+1

The revised architecture therefore makes this boundary explicit:

￼
Tracking module
    ↓
ModelRunner trait
    ↓
InferenceBackend trait
    ↓
avatartrack-inference-ort
    ↓
pinned ort Rust crate
    ↓
ONNX Runtime C API
    ↓
CUDA / TensorRT / OpenVINO / DirectML / CPU
The ort crate is confined to one adapter. It already exposes Rust interfaces for ONNX Runtime sessions and execution providers, but it does not become a domain-level dependency. 
￼
GitHub
+2
￼
Docs.rs
+2

Should that wrapper block a required provider or lifetime model, only the adapter is replaced with a small internal Rust FFI crate against ONNX Runtime’s stable C API.

What “no C++” means
The project now has:

No first-party .cpp, .cc, .cxx, .hpp, or CMake files.

No C++ domain model.

No C++ daemon.

No C++ plugin API.

No Rust-to-C++ object binding layer.

No C++ compilation target maintained by the project.

External native dependencies can still contain C or C++ internally. ONNX Runtime, GStreamer, operating-system media libraries, and GPU drivers are examples. The Rust code reaches them through narrow FFI or safe binding crates. Rust’s official documentation supports using C-compatible interfaces for foreign libraries, with unsafe behavior contained at the boundary. 
￼
Rust Documentation
+1

GStreamer has maintained Rust bindings that expose a safe Rust API over the underlying native library, making it a reasonable first broad camera/media adapter while still allowing direct Media Foundation or V4L2 adapters where measurements justify them. 
￼
GStreamer
+1

Rust safety and latency rules added
The document now requires:

Safe Rust by default.

unsafe only in small FFI or platform-adapter modules.

A SAFETY: invariant on every unsafe block.

Raw pointers and foreign handles never leaving their owning adapter.

No panic unwinding through FFI.

Typed errors within libraries.

No unwrap, expect, unchecked indexing, or unchecked conversions on external-data paths.

No broad Arc<Mutex<_>> object graph.

One owner for mutable realtime state.

Bounded message passing between stages.

Preallocated frame, packet, tensor-binding, and output storage where profiling justifies it.

No blocking I/O, database work, filesystem access, or log formatting on the final output tick.

No Rust ABI or trait objects across plugin boundaries.

Out-of-process Protobuf IPC for community plugins by default.

Rust ownership provides memory management without a garbage collector, which fits a latency-sensitive native pipeline, but it does not automatically solve scheduling or allocation jitter. Those still require the explicit realtime-domain and buffer-lifetime design described in the revision. 
￼
Rust Documentation
+1

The resulting decision is clean: Rust is the product runtime language; Python remains the ML research language; TypeScript remains the replaceable UI language; Cargo remains the Rust source of truth.

￼
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Rust_Revision_1_2.md
Document
￼
￼
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Revision_1_2_Rust.md
Document
￼
￼
Sources
￼
￼
￼
￼
￼
ChatGPT can make mistakes. Check important info.
￼
￼

￼
Pro
￼
￼
￼￼Choose FilesNo file chosen￼￼Choose FilesNo file chosen
Library
/
Commodity_Camera_Avatar_Tracking_Repository_API_Modular_Architecture_Rust_Revision_1_2.md
￼
￼
1
2
3
4
5
6
7
8
9
10
11
12
13
14
15
16
17
18
19
20
21
22
23
24
25
26
27
28
29
30
31
32
33
34
35
36
37
38
39
40
41
42
43
44
45
46
47
48
49
50
51
52
53
54
55
56
57
58
59
60
61
62
63
64
65
66
67
68
69
70
71
72
73
74
75
76
77
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
