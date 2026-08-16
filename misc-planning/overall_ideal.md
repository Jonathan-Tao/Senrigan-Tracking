# Commodity-Camera Avatar Tracking

## Product, Technical, Training, and Market Design Specification

**Revision 1.1 — Person-agnostic, demonstration-ready architecture**  
**Concept status:** Research-backed open-source design  
**Market sweep current to:** August 16, 2026

> **Mission:** Make full-body VRChat and VTuber animation look professionally captured using one to several ordinary webcams—without wearable trackers, base stations, target boards, subscriptions, or a fragmented software stack.
>
> **North-star experience:** Select a quality mode and camera count, import an avatar, complete a short guided setup, and start. The same calibrated camera rig must accept any unseen person without retraining or becoming specialized to the person who performed the initial setup. It must remain useful on inexpensive, mixed-quality webcams.

## Document control

| Field | Specification |
|---|---|
| Primary audience | Independent developer, open-source contributors, VTubers, VRChat users, demo/kiosk operators |
| Product posture | Free, local-first, open-source application; Windows-first runtime with portable model/runtime layers |
| Core quality target | Perceptual avatar fidelity in bounded creator spaces, not laboratory-grade human measurement |
| Reference hardware | Ordinary 720p/1080p USB webcams, including mixed and low-cost models, plus a normal gaming PC |
| Target modes | Desktop VTubing; VRChat with HMD/controllers as trusted anchors; one, two, or N cameras |
| Active-person policy | One active performer at a time, but the performer may be any unseen human; simultaneous multi-person capture is not required for version 1 |
| Personalization policy | No per-person model fine-tuning, no permanent performer-specific weights, and no dependency on the person who calibrated the room |
| State policy | Persistent Site State + persistent Avatar Profile + ephemeral Performer State + short-lived Temporal State |
| Evidence policy | Official sources and primary research first; community reports are explicitly labeled anecdotal |

*Prices and availability in the market appendix are public list prices observed around the stated date and may vary by region, taxes, bundles and promotions. This document is a design proposal, not a claim that the final quality target has already been achieved.*

## 0.1 How to read this document

- Sections 1–10 define the product and technical design.

- Section 11 maps recurring complaints from current users to explicit countermeasures.

- Section 12 repeats the market sweep in detail across software, camera, depth, IMU, optical and professional systems.

- Section 13 lists the evidence base and source-quality method.

## 0.2 Bottom-line market conclusion

No current product combines ordinary webcams, one-to-many camera scaling, transferable human-motion site calibration, unseen-performer hot-swap, modern full-body reconstruction, dedicated face and hand tracking, confidence-aware fusion, avatar-specific retargeting, local real-time output, and free/open distribution in one polished VRChat/VTuber application.

> **Closest existing pieces—not one complete competitor**
>
> MocapForAll and Remocapp validate cheap multi-camera capture; FreeMoCap validates open commodity-camera infrastructure; PoseCap validates modern single-camera mesh recovery; Kineo validates human-motion self-calibration; XR Animator and Webcam Motion Capture validate low-friction avatar output. The proposed product is the integration and specialization gap between them. [S19–S28]

## 0.3 Document map

| **Part** | **Section**                              | **Purpose**                                                               |
|----------|------------------------------------------|---------------------------------------------------------------------------|
| 1        | Executive product thesis                 | What is being built, for whom and why the narrow objective matters.       |
| 2        | User demand and success definition       | What creators actually care about; measurable product targets.            |
| 3        | Product experience                       | One-camera, two-camera, and Studio modes; site setup, guest handoff, and avatar import. |
| 4        | Runtime system architecture              | Capture, state separation, self-calibration, performer adaptation, fusion, temporal solve, and output. |
| 5        | Body, face, hands and avatar retargeting | Dedicated modality paths and VRChat/VRM integration.                      |
| 6        | Performance and hardware scaling         | Inference-rate knobs, interpolation, latency and camera-count scheduling. |
| 7        | Training and data strategy               | Synthetic supervision, public data, distillation and legal constraints.   |
| 8        | Quality assurance and evaluation         | How to test “looks as good as mocap” without hiding failures.             |
| 9        | Implementation roadmap                   | A hobby-project path that produces useful checkpoints.                    |
| 10       | Risks, non-goals and honest limits       | Where vision cannot know the answer and how the system fails safely.      |
| 11       | Complaint-to-fix matrix                  | How recurring grievances are addressed.                                   |
| 12       | Comprehensive market sweep               | Current options across price and technology tiers.                        |
| 13       | Sources and research method              | Official, primary and anecdotal evidence.                                 |

# 1. Executive product thesis

A narrowly optimized avatar-animation system can beat broader tracking products on the only output this audience sees.

The product is not a general motion-capture instrument. It is an avatar performance synthesizer whose observations happen to come from cameras. Its success is judged by the motion delivered to a VRChat or VTuber model: stable, responsive, expressive, believable and free of catastrophic glitches.


| Do not optimize as the primary objective | Optimize directly |
|---|---|
| Lowest independent-frame joint error | Root and feet that remain visually anchored |
| A perfect human mesh on every frame | No flips, pops, or unexplained limb teleports |
| Treating hidden joints as certain | Calibrated uncertainty and graceful continuation |
| Generic retargeting to every avatar | Avatar-specific proportions, limits, contacts, and balance |
| Maximum inference FPS at any latency | Low latency with causal prediction and smooth correction |

A stable, anatomically plausible error of a few centimeters can look better on a stylized avatar than a millimeter-level estimate that jitters every frame.


## 1.1 Product thesis

Modern monocular human-reconstruction models can provide a strong 3D body hypothesis from ordinary RGB images, including hands and feet, but independent-frame use remains too slow and temporally unstable for a mass-market real-time application. [S01–S05]

A second camera changes the problem materially: complementary 2D evidence permits geometric triangulation and reduces occlusion, while the body model supplies anatomical priors. Three or more cameras further reduce shared blind spots. Recent uncalibrated multi-view research shows that camera geometry can be recovered from human motion rather than a checkerboard or marker board. [S09–S11]

For stylized avatars, the final solver is allowed to “cheat”: lock a planted foot, preserve fixed bone lengths, suppress one-frame joint flips, infer brief hidden motion and retarget to non-human proportions. These operations can lower frame-wise reconstruction fidelity while improving what viewers perceive.

The most credible route to ordinary hardware is teacher–student distillation. Use SAM-class and offline multi-view optimization to create strong targets; ship a compact camera/fusion/temporal model specialized for one active human at a time, fixed cameras, arbitrary performer identity, and humanoid avatar output. The runtime may estimate a temporary body code for the current person, but it must not update global weights or become dependent on that person. [S01–S07]

## 1.2 North-star user experience

> **The entire promise**
>
> Choose camera count and quality mode. Import or select an Avatar Profile. If the cameras have never been placed in this room, one person performs the site-calibration motion once. After that, any person can enter, optionally hold a short neutral/A-pose for maximum quality, and start tracking. Additional cameras raise quality automatically. The user never handles camera matrices, checkerboards, marker IDs, per-person training, or manual bone offsets.

## 1.3 What could plausibly become obsolete

| **Current method**                            | **Design goal**                       | **Honest assessment**                                                                                                                                                       |
|-----------------------------------------------|---------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Single-webcam MediaPipe-style tracking        | High confidence                       | A modern temporal 3D model with session-constant anatomy, unseen-person adaptation, and failure containment should be materially better.                                                                          |
| Software-only lower-body estimation           | High confidence                       | Actual visual observations of legs and hips are a stronger input than HMD/controller-only inference.                                                                        |
| Entry IMU systems for camera-visible creators | Plausible                             | Eliminates wearables, charging and yaw drift; loses when all cameras are occluded or the user leaves the capture volume.                                                    |
| Three-tracker consumer FBT                    | Plausible on perceived avatar quality | Dense whole-body evidence plus avatar-aware cleanup can look better than sparse points plus generic IK.                                                                     |
| High-end multi-tracker or optical mocap       | Aspirational in bounded motions       | Viewers may see little difference after retargeting for ordinary streaming/dance, but the professional system still wins raw measurement and difficult hidden interactions. |

> **Hard truth**
>
> The system cannot literally make every tracker obsolete. A fully hidden hand can have many physically valid poses that produce identical pixels. IMUs and optical trackers retain information the cameras do not have. The defensible claim is perceptual parity or superiority for one camera-visible performer in a bounded creator space—not universal physical equivalence.

## 1.4 Person-agnostic product invariant

“One active performer” must never mean “one supported performer.” The product is designed for demos, conventions, studios, households, and community spaces where people rotate through the same camera setup.

| Invariant | Requirement |
|---|---|
| Global model | The perception, fusion, temporal, face, hand, and retargeting weights are shared across all users. Runtime gradient updates are forbidden in the normal application. |
| Site calibration | Camera intrinsics/extrinsics, timing, floor, and capture volume persist until a camera moves. They must not encode the calibration person’s limb lengths, face, clothing, or identity. |
| Performer fit | Body proportions, neutral face scale, handedness cues, and current motion history are estimated online and stored only for the active session unless the user explicitly saves a convenience profile. |
| Avatar profile | Rig mapping, rest axes, joint limits, expression channels, tracker roles, and model proportions belong to the imported avatar, not to a human performer. |
| Handoff | When a new person enters, old body proportions, contacts, motion history, and face normalization are cleared. Site State and Avatar Profile remain valid. |
| Privacy | Persistent face recognition is unnecessary. Any appearance embedding used to keep the active track coherent is ephemeral and discarded when the track ends. |

The preferred UX has two levels:

1. **Instant guest mode:** the new person enters and tracking begins immediately with a generic body prior; proportions settle over the first few seconds.
2. **Quality guest mode:** the person holds neutral and A-pose positions for roughly 3–8 seconds, improving body-to-avatar scale, shoulder axes, hand crop placement, and floor/contact initialization. This is a quick fit, not camera recalibration and not personal model training.

# 2. User demand and success definition

The market repeatedly values stability, setup simplicity and expressiveness more than abstract pose benchmarks.

## 2.1 Primary users

| **Persona**                                 | **Likely setup**                                         | **Job to be done**                                                                      |
|---------------------------------------------|----------------------------------------------------------|-----------------------------------------------------------------------------------------|
| Desktop 3D VTuber                           | One or two webcams, no headset                           | Direct VRM/VMC body, face, eyes, lips and hands in one local app.                       |
| VRChat creator with HMD/controllers         | One or two room cameras plus existing tracked head/hands | Cameras solve chest, hips, elbows, knees and feet; HMD/controllers remain hard anchors. |
| Dancer / performance streamer               | Two to four cameras, faster shutter and wider space      | Stable feet, root, turns, crouches, spins and rapid motion without resets.              |
| Instrument performer                        | Two or more views, optional hand-focused camera          | Convincing torso, elbows, wrists and gross finger intent around a guitar/keyboard/prop. |
| Budget enthusiast / open-source contributor | Mixed webcams and GPUs                                   | Useful quality on commodity hardware, inspectable code, no recurring payment.           |
| Demo / kiosk operator                       | Pre-calibrated 2–4-camera station                       | Let many unseen people step in, select a model, perform a short fit, and receive stable output without operator intervention. |

## 2.2 What users consistently want

**No random explosions.** An occasional small positional error is acceptable; a knee flip, hip teleport or arm snap is not.

**Stable feet and root.** Feet must stay planted, the body must not “breathe” in depth, and standing still must look still.

**Reliable dancing and transitions.** The system must survive turns, crossing limbs, crouching, sitting, standing up and rapid gestures.

**Automatic fit to unusual avatars.** Chibi, tall, long-limbed, short-legged and highly stylized humanoids must not inherit broken knees or stretched hips.

**Face and hands without another hardware stack.** Creators want eye, mouth and finger expressiveness, but dislike adding a phone, hand sensor, gloves and multiple bridge applications.

**Low setup and low maintenance.** No target board, tape measurements, strap placement ritual, frequent recentering or per-session drift correction.

**Local, affordable and predictable.** No per-minute cloud credits; no upload requirement; an understandable quality/performance control.

*These priorities are synthesized from official limitation/support documentation, Steam reviews, GitHub issue discussions and community forums. Community reports are qualitative and anecdotal rather than prevalence statistics. [S20–S40, S77–S80]*

## 2.3 Definition of “crazy good”

| **Quality dimension** | **Observable success condition**                                                                                             |
|-----------------------|------------------------------------------------------------------------------------------------------------------------------|
| Perceptual continuity | No visible single-frame flips or teleports; uncertain limbs follow a bounded, plausible path.                                |
| Contact quality       | Planted feet and seated hips remain visually locked; release occurs promptly when the user moves.                            |
| Root quality          | No uncontrolled depth pumping; correct gross forward/backward translation in 2+ camera mode.                                 |
| Responsiveness        | Normal gestures feel immediate; no floaty delayed body chasing the head/controllers.                                         |
| Expressiveness        | Torso twist, shoulder rhythm, hand orientation, face and gaze preserve performance intent.                                   |
| Avatar fit            | Calibration succeeds without manual joint offsets on mainstream humanoid rigs; extreme proportions degrade gracefully.       |
| Session reliability   | Long sessions do not accumulate yaw drift or require periodic resets; camera disagreement never causes an uncontrolled pose. |
| Setup burden          | First site setup within five minutes; repeat sessions within one minute if cameras did not move.                            |
| Performer interchange  | A person who did not calibrate the site reaches stable tracking within seconds, with no body-shape leakage from the prior person. |

## 2.4 Release acceptance targets

| **Gate**     | **Test**                      | **Target**                                                                                                                                                                                                           |
|--------------|-------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Prototype    | Offline two-view sequence     | Clearly lower depth error and fewer occlusion failures than best single view; deterministic replay and debug visualization.                                                                                          |
| Alpha        | 1 camera live                 | 30–60 Hz avatar output, heavy body observations ≥15 Hz, median end-to-end body latency under 120 ms, no uncontrolled pose explosions in a 10-minute scripted test.                                                   |
| Beta         | 2 cameras live                | Recommended 20–30 Hz fused observation rate, 60–90 Hz output, median latency under 90 ms, planted-foot slide under a visually defined threshold, less than one severe recovery event per hour in the standard suite. |
| 1.0          | 1–N cameras + avatar profiles | Five-minute first site setup, sub-minute repeat setup, stable 30-minute dance/sit/turn test, direct VRM/VMC plus VRChat virtual trackers, local logging and replay.                                                  |
| Identity gate | Cross-person handoff | Person A calibrates the site; at least ten unseen performers then use it without camera recalibration. Median time-to-good under 8 seconds in Quality Guest mode, with no prior-person proportion carryover. |
| Aspirational | Perceptual parity study       | Blind viewers cannot reliably prefer a consumer or professional reference after both are retargeted to the same stylized avatar for bounded test motions.                                                            |

# 3. Product experience

The UX hides calibration mathematics without pretending calibration does not exist.


| Mode | User promise | Main evidence | Compute posture |
|---|---|---|---|
| 1 camera — Good | Strong wearable-free baseline for standing, talking, light movement, and moderate dance | Monocular 3D prior + fixed session anatomy + temporal/contact cleanup | Lowest compute |
| 2 cameras — Recommended | Strong depth, turns, sitting, dance, and occlusion handling | Front-left + front-right geometry, automatic site calibration, learned fusion | Best value |
| 3–N cameras — Studio | Very high coverage and very few shared occlusions | Set-based fusion, dynamic best-view selection, optional hand/prop views | Highest ceiling |


## 3.1 Core workflow

The application exposes a simple flow while maintaining strict internal separation between the room, current human, and avatar.

### A. One-time site setup

1. **Connect cameras and select Quality mode.** The application enumerates devices, measures delivered frame rate, detects USB contention, and evaluates blur, exposure, framing, and missing lower-body coverage.
2. **Perform site-calibration motion.** Any sufficiently visible person can perform neutral stance, A/T pose, quarter-turns, a step or march, shallow crouch, and arm motion. The software estimates camera geometry, timing, floor, and usable capture volume.
3. **Review the placement score.** The app gives plain-language guidance such as “Camera 2 is too similar to Camera 1” or “move Camera 3 lower so both feet remain visible.”
4. **Save Site State.** It remains valid until a camera moves, changes zoom/crop, or is replaced. It is not tied to the calibration person.

### B. One-time avatar setup

1. Import VRM/FBX/GLB or an exported VRChat Avatar Profile.
2. Validate humanoid bone mapping, rest axes, joint limits, foot direction, expression channels, tracker roles, and scale.
3. Run an automatic retarget preview using canonical test motions: reach, crouch, sit, step, twist, and guitar hold.
4. Save the Avatar Profile independently of any human performer.

### C. Per-person start

1. A new person enters the capture volume.
2. **Instant Guest:** tracking starts immediately and improves as the Performer Adapter estimates proportions.
3. **Quality Guest:** the person holds neutral and A-pose for a few seconds. Optional quarter-turn/crouch motions improve shoulder, knee, floor, and hand calibration.
4. The avatar begins output. No camera solve, model retraining, or manual bone editing is repeated.
5. When the person leaves, the application clears all ephemeral human state and waits for the next guest.

## 3.2 Camera placement without “fancy calibration”

The application should accept approximate placement, but it must reject geometrically useless arrangements. Two cameras immediately adjacent to each other provide little new depth information; a camera that never sees the lower body cannot improve feet. “Plug-and-play” means automatic geometry estimation plus plain-language guidance—not that every physical arrangement is equally good.

| Mode | Default guidance | Reason |
|---|---|---|
| 1 camera | Front or slight three-quarter view; full body visible; camera near waist/chest height when possible | Best single-view visibility; feet and hands need sufficient pixels |
| 2 cameras | Front-left and front-right, generally 30°–90° apart, both seeing most of the body | Strong shared correspondences, useful depth baseline, complementary occlusion, face visibility |
| 3 cameras | Add a side or rear three-quarter view | Protects against turns, crossed arms, instrument occlusion, and body blocking |
| 4+ cameras | Spread around the usable volume; mixed heights may help | Creates a cheap markerless capture volume; the solver selects useful views per joint |

## 3.3 Calibration domains

| State domain | Examples | Persistence | May depend on the current human? |
|---|---|---|---|
| **Site State** | Camera intrinsics, distortion, relative pose, timing offsets, floor plane, capture bounds, static occlusion masks | Until a camera or room geometry changes | **No.** A human may provide motion correspondences, but the stored result is camera/room geometry only |
| **Avatar Profile** | Skeleton map, rest pose, bone axes, limits, foot geometry, expressions, virtual-tracker roles, optional prop/contact metadata | Until the avatar changes | **No.** It describes the model |
| **Performer State** | Session body proportions, neutral face normalization, handedness confidence, current track, height/scale estimate | Current person/session only | **Yes, transiently.** It is discarded on handoff by default |
| **Temporal State** | Pose history, velocity, acceleration, contacts, occlusion memory, motion mode | Milliseconds to seconds | **Yes, transiently.** It must be cleared when a new person is detected |

This separation should exist in code, storage schemas, and tests. A single serialized “calibration blob” containing camera geometry plus Person A’s body shape is explicitly prohibited.

## 3.4 Guided site calibration and quick performer fitting

### Site-calibration sequence — normally once per camera placement

| Pose/motion | What it solves | Identity-leakage rule |
|---|---|---|
| Neutral stance, 2–3 seconds | Vertical direction, stillness noise, rough scale, face/body framing | Do not persist the person’s body code |
| A- or T-pose | Long limb correspondences and camera rays | Use limb consistency, not fixed identity-specific lengths |
| Turn left and right | Cross-view correspondences, relative depth, camera rotation | Persist only camera geometry |
| March/step | Timing offsets, floor, translation scale, contact observations | Persist floor/timing; discard gait signature |
| Shallow crouch | Camera-depth excitation and floor/contact geometry | Do not store knee/hip proportions in Site State |
| Arm circles/hands near face | Shoulder/hand visibility and lens coverage | Persist camera quality maps only |

### Performer quick fit — optional on each new guest

| Action | Runtime estimate |
|---|---|
| Stand naturally | Height, shoulder/hip width, neutral root, face crop scale |
| Brief A-pose | Upper/lower arm ratios, shoulder axes, hand reach, torso proportions |
| One shallow crouch or step | Femur/tibia ratio, knee direction, floor-contact initialization |
| Neutral face then blink/smile | Expression normalization without requiring a face-worn tracker |

The quick fit is amortized inference or bounded optimization; it must not execute gradient descent on global model weights. If skipped, tracking starts with population priors and refines conservatively over the first seconds.

## 3.5 Guest/kiosk mode and person switching

The application runs a simple handoff state machine:

```text
EMPTY
  -> ACQUIRING       person enters and is sufficiently visible
  -> FITTING         estimate transient body/face normalization
  -> TRACKING        normal output
  -> LOST            person leaves or confidence collapses
  -> EMPTY           clear Performer State and Temporal State
```

A new-person decision may use discontinuities in position, body proportions, clothing features, and face/body track continuity, but any appearance embedding is short-lived and never used as persistent biometric identity. A manual **New Guest** button provides a deterministic reset for demos.

During a swap, the system must:

- preserve Site State and Avatar Profile;
- stop output or hold the avatar in a neutral safe pose;
- clear old contacts, velocities, face normalization, body code, and motion-class state;
- reacquire the new person before resuming;
- never morph Person B through Person A’s proportions over several seconds.

## 3.6 Avatar-profile workflow

- **Standalone VTuber:** import VRM or a supported humanoid FBX/GLB. Read hierarchy, rest pose, bone axes, limits, expressions, and optional spring-bone/collider metadata. [S14–S15]
- **Custom VRChat avatar:** a Unity companion exporter writes an Avatar Profile containing skeleton proportions, axes, tracker offsets, supported parameters, and optional contact/collider metadata—not the avatar asset itself.
- **Public VRChat avatar:** use generic virtual-tracker mode. The application cannot inspect an avatar file it does not possess, so VRChat’s own IK remains the final retargeter. [S12–S13]
- **Automatic validation:** run canonical reach, crouch, sit, turn, step, and guitar-hold poses. Detect reversed axes, unreachable tracker targets, permanently bent knees, foot rotation errors, and extreme proportions.
- **Storage rule:** model-specific scale and offsets persist in Avatar Profile; human-specific body dimensions do not.

## 3.7 User-facing controls

| Control | Behavior |
|---|---|
| Camera count | Automatic detection; each view can be body, face/hand detail, prop view, or disabled |
| Performance ↔ quality | Controls resolution, heavy-model cadence, crop frequency, hand refinement, temporal context, and number of expensive views |
| Motion style | Neutral, responsive, smooth, dance, seated, instrument; these tune priors and contact thresholds rather than adding canned animation |
| Guest mode | Instant or Quality Guest; optional manual New Guest reset |
| Latency budget | Low-latency VR mode versus smoother desktop/recording mode |
| Confidence display | Optional overlay showing observed, triangulated, inferred, contact-locked, and uncertain joints |
| Privacy | Local-only by default; no raw image retention; no persistent face identity; explicit opt-in for recording/debug or future data contribution |

# 4. Runtime system architecture

Confidence-aware geometry and temporal animation are more important than running a giant mesh model on every frame.


```text
Cameras 1..N                    HMD/controllers when present
    |                                      |
    v                                      v
Capture, timestamps, health, ring buffers, synchronization
    |
    +------------------- Persistent Site State -------------------+
    |        intrinsics, distortion, camera poses, timing, floor  |
    v                                                             |
Per-view shared encoder -> 2D landmarks, masks, visibility, features
    |                         |                         |
    |                         +-> native-res face/hand crops       |
    |                                                             |
    +-> selected heavy 3D observations                            |
    |                                                             |
    v                                                             |
Robust geometry + set-based learned fusion <----------------------+ 
    |
    +-> ephemeral Performer Adapter -> body proportions / neutral face scale
    |
    v
Canonical human pose + uncertainty + contacts + prop state
    |
    v
Causal temporal solver -> contact locks -> safe recovery
    |
    +---------------- Avatar Profile ----------------+
    |  rig mapping, rest axes, limits, proportions,  |
    |  expressions, tracker roles, optional colliders|
    v                                                |
Avatar-aware retargeter                              |
    |                                                |
    +-> VRM/VMC skeleton + face                      |
    +-> SteamVR/OSC virtual trackers + face channels |
    +-> recording/replay/debug                       |
```


## 4.1 Capture and synchronization layer

- Acquire each stream with hardware or monotonic host timestamps. Measure actual cadence instead of trusting nominal “30 FPS.”

- Use MJPEG or hardware decode where necessary to avoid saturating USB; warn when several cameras share a constrained controller.

- Normalize orientation, color range and exposure metadata; optionally recommend manual exposure/high shutter for dance to reduce motion blur.

- Maintain a short timestamped ring buffer. Estimate per-camera temporal offsets from body-motion correlation and refine slowly during a session. [S09]

- Detect dropped/repeated frames and mark them unavailable rather than passing stale observations as new evidence.

## 4.2 Per-camera observation layer

Each camera produces an observation package rather than a final answer. The package includes body landmarks, image-space uncertainty, visibility, segmentation, optional 3D body latent, compact visual features and high-resolution face/hand crops. This preserves information that would be lost if the fusion model received only two completed skeletons.

| **Observation**           | **Cadence**                 | **Purpose**                                                                                                                                              |
|---------------------------|-----------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------|
| 2D body landmarks         | Fast, every available frame | Triangulation, camera solve, tracking continuity.                                                                                                        |
| Visibility / occlusion    | Fast, every frame           | Reject hidden or truncated joints.                                                                                                                       |
| Person mask / silhouette  | Moderate cadence            | Camera refinement, depth ordering, body extent.                                                                                                          |
| Compact image features    | Shared encoder              | Learned multiview fusion and ambiguous joints.                                                                                                           |
| Heavy 3D body observation | 12–30 Hz, selected views    | Strong pose/shape prior and initialization. SAM-class models are teachers or optional high-end observers, not necessarily the shipping path. [S01–S05] |
| Face / hand crops         | 30–60 Hz as budget allows   | Preserve small details that disappear in a downscaled full-body frame. [S17–S18, S60–S62]                                                              |

## 4.3 State partitioning and the Performer Adapter

The runtime must make it structurally difficult to overfit to whoever performed calibration.

### Persistent Site State

```text
SiteState = {
  camera_intrinsics[i], distortion[i], camera_to_world[i],
  time_offset[i], floor_plane, capture_bounds,
  static_visibility_map, camera_health_baseline
}
```

Site State can be refined only from high-confidence cross-view geometry. It is versioned, quality-scored, and invalidated if a camera moves. It contains no face embedding, clothing descriptor, gait signature, or human limb lengths.

### Persistent Avatar State

```text
AvatarProfile = {
  humanoid_bone_map, rest_transforms, bone_axes, joint_limits,
  proportions, foot_geometry, expression_map,
  tracker_roles, optional_colliders_and_prop_contacts
}
```

### Ephemeral Performer State

```text
PerformerState = {
  anonymous_track_id,
  body_code, height_scale, limb_ratios,
  neutral_face_normalization,
  handedness_confidence,
  current_visibility_and_quality
}
```

The **Performer Adapter** predicts this state from the current person’s first frames and then updates it slowly under strict confidence gates. It is an amortized network or bounded optimizer, not a fine-tuning loop. Shape estimates are held constant while the person moves rapidly or is occluded, preventing body “breathing.”

### Ephemeral Temporal State

```text
TemporalState = {
  pose_history, velocity, acceleration,
  contact_state, occlusion_memory, motion_mode,
  pending_corrections
}
```

On a person swap, Performer State and Temporal State are zeroed. Site State and Avatar Profile are retained. Unit tests should deliberately feed Person B after Person A and fail if Person A’s body code influences Person B after the reset boundary.

## 4.4 Automatic camera geometry

- Initialize person correspondences from synchronized 2D landmarks during the guided motion sequence.

- Solve relative rotations, translations, scale and approximate focal parameters with robust bundle adjustment / epipolar constraints.

- Use temporally consistent limb lengths estimated during the calibration motion, floor contacts, and moving joints as additional constraints. Absolute human identity is marginalized out; only geometry enters Site State.

- Estimate rolling temporal offset jointly or in alternating steps.

- Continuously refine at low rate only when the person is well observed; freeze geometry when confidence falls.

- If a camera physically moves, detect a persistent reprojection error and request a short re-calibration rather than silently corrupting the pose.

> **Research basis**
>
> Kineo and recent uncalibrated multi-view HMR methods demonstrate that human motion can estimate camera geometry and timing from consumer RGB views. This product constrains that problem to fixed cameras and one active person at a time, while explicitly requiring the resulting Site State to transfer to people who were not present during calibration. [S09–S11]

## 4.5 Geometric fusion

- Robustly triangulate each visible joint using confidence-weighted rays and a Huber/Tukey loss; reject high-residual camera observations.

- Fit the current session’s fixed-proportion canonical skeleton or body latent to triangulated joints, silhouettes, and per-view 3D priors. The proportions come from ephemeral Performer State, not persistent Site State.

- Preserve camera-specific residuals so the system can explain why an arm was ignored rather than silently averaging incompatible estimates.

- For one-camera mode, treat triangulation as unavailable and increase reliance on the monocular 3D and temporal priors.

## 4.6 Learned set-based fusion

A small set transformer or graph network consumes an unordered set of camera/joint tokens. It predicts per-joint view weights, a corrected fused pose, confidence and contact probabilities. Training with random camera counts and view dropout allows one architecture to accept one, two or many cameras. MUC and U-HMR provide useful structural precedent. [S10–S11]

```text
joint_token = {
  camera_id, timestamp, pixel_xy, confidence, visibility,
  camera_ray, optional_camera_relative_3d, velocity,
  compact_image_feature, current_performer_body_code
}

unordered joint tokens
    -> camera-aware set attention
    -> fused canonical-human state
    -> uncertainty + contacts + outlier scores
```

## 4.7 Causal temporal reconstruction

- Maintain a body state with joint rotations/positions, velocity, acceleration, contact, visibility and uncertainty.

- Predict to the desired output timestamp using past observations only. Traditional interpolation that waits for a future frame adds avoidable latency.

- When a new observation arrives, apply a confidence-weighted correction over several output ticks unless an immediate correction is physically necessary.

- Use motion-state context—standing, walking-in-place, dancing, crouching, seated, lying, instrument—to select appropriate contact and smoothness behavior. Motion-state memory is reset on a performer handoff.

- World-grounded HMR work such as WHAM, GVHMR and OnlineHMR is relevant, but the fixed-camera room makes this problem easier. [S05–S07]

## 4.8 Failure containment

| **Mechanism**          | **Prevents**                                                                                                |
|------------------------|-------------------------------------------------------------------------------------------------------------|
| Observation gate       | Reject joint innovations that exceed confidence-dependent position, rotation or acceleration limits.        |
| Bone lock              | Current performer bone lengths and joint axes remain fixed within the session; they are re-estimated after a guest handoff. |
| Joint limits           | Prevent anatomically impossible knee/elbow directions and avatar-specific rig violations.                   |
| Confidence hysteresis  | A camera does not oscillate between trusted/untrusted every frame.                                          |
| Occlusion hold/predict | Briefly continue the physical trajectory; gradually return toward a safe prior if uncertainty remains high. |
| Contact lock           | A planted foot or supported hand is held in world/avatar space until release evidence is strong.            |
| Safe recovery          | Blend back when evidence returns; never teleport to a new estimate.                                         |
| Session watchdog       | If Site State, person tracking, or all views fail, freeze/reduce motion gracefully, distinguish the failure type, and explain it on-screen. |

# 5. Body, face, hands, instruments, and avatar retargeting

A single application can share context, but one network should not be forced to solve every scale equally.

## 5.1 Body model

The internal body representation should be canonical and person-agnostic. A compact full-body model estimates pelvis/root, torso, limbs, feet, and coarse hand articulation from the full frame, while an ephemeral body code describes the current person’s proportions.

### Canonical body state

- Store joint rotations, world/root translation, contacts, and uncertainty in a normalized human skeleton or parametric latent.
- Separate **pose** from **shape**. Pose changes every frame; shape should change only during initial acquisition or slow, high-confidence refinement.
- Maintain a richer internal state than the sparse tracker targets eventually sent to VRChat. Dense internal joints improve contacts, occlusion recovery, shoulder behavior, and avatar retargeting.
- Use SAM 3D Body, 4DHumans, and temporal/world-grounded HMR systems as teacher/reference candidates. Runtime use must be benchmarked and likely distilled. [S01–S08]

### Performer Adapter

1. Normalize observations into Site State coordinates.
2. Aggregate 1–3 seconds of visible joints, silhouettes, and monocular body latents.
3. Predict a compact body code: overall scale, shoulder/hip width, torso length, upper/lower arm ratio, femur/tibia ratio, and foot scale.
4. Fit the code under robust reprojection and fixed-bone constraints.
5. Freeze or very slowly refine it while tracking.
6. Clear it on a new-person event.

The adapter must generalize to unseen body types. It may condition the runtime solver but never alter the global network weights. Version 1 should not require or create a persistent Human Profile; every entrant is re-estimated from live observations so a demo station remains genuinely person-agnostic.

## 5.2 Face, eyes and lips

Face motion is perceptually high-frequency and should remain responsive even when body inference is throttled. The system dynamically selects the clearest native-resolution face crop, runs a dedicated identity-agnostic landmark/expression model, stabilizes gaze and mouth independently, and maps normalized expression intent through the Avatar Profile. The first neutral frames may establish per-session eye openness, mouth rest, and face scale, but no face-worn tracker or persistent facial identity model is required. MediaPipe is a free baseline; ARKit remains a strong consumer reference; VRCFaceTracking provides a practical VRChat bridge. [S17, S57, S63, S81]

| **Signal**              | **Rate target** | **Design**                                                                                                                        |
|-------------------------|-----------------|-----------------------------------------------------------------------------------------------------------------------------------|
| Face pose / head        | 30–60 Hz        | Use image face pose unless an HMD supplies a better head anchor.                                                                  |
| Eyes / blink / gaze     | 30–60 Hz        | Confidence-aware gaze; avoid eye teleport under glasses or occlusion.                                                             |
| Mouth / visemes         | 30–60 Hz        | Blend visual mouth shapes with optional audio viseme timing; visual confidence remains primary.                                   |
| Expression coefficients | Model-dependent | Normalize expression against the current face, then map through per-avatar blendshape calibration; human normalization is ephemeral. |
| VRChat output           | Runtime bridge  | Use VRCFaceTracking/OSC-compatible channels where the avatar supports them; not every public avatar exposes identical parameters. |

## 5.3 Hands and fingers

- Never infer fingers solely from a downscaled full-body tensor. Crop original-resolution hands using the body model and process them with a dedicated hand model. [S18, S60–S62]

- Fuse hand crops from all views. A camera that sees the palm may be useless for the occluded fingertips; a side camera may resolve them.

- Use wrist/forearm orientation and hand-object context to disambiguate; maintain finger confidence separately from wrist confidence.

- In VR mode, fuse controller orientation and available finger/capacitive input as strong anchors.

- When fingers are too small or hidden, output a stable gesture prior rather than high-frequency invented articulation.

> **Instrument stress test**
>
> Guitar, keyboard, and prop performance is a first-class benchmark because torso, elbows, wrists, hand contacts, fingers, and a rigid object are simultaneously constrained and frequently occluded. Two body views plus an optional hand/prop-focused view should make the performance convincing. Exact fret- and string-level fingers remain limited by pixel resolution, but object-aware contact constraints can keep the gross performance visually correct.

## 5.4 Avatar-aware retargeting

- Solve **intent**, not a literal one-to-one skeleton copy. Preserve contact, balance, facing, reach, rhythm, and gesture readability across different proportions.
- Build an Avatar Profile from rest pose, humanoid mapping, bone axes, limb lengths, joint limits, foot dimensions, expression channels, and optional colliders.
- Compute a fresh human-to-avatar scale and morphology mapping for every active performer from Performer State + Avatar Profile. The avatar setup remains valid while the incoming human changes.
- Optimize target bone rotations, root/pelvis path, and virtual-tracker locations for the actual avatar. A chibi squat may need a different pelvis path than the human motion to look equivalent.
- Preserve world contacts even when the target body differs sharply. A short-legged avatar may bend more at the hips; a long-armed avatar may reduce shoulder translation; neither should cause floating feet or detached hands.
- Support direct VRM/VMC skeletal output for standalone VTuber software. [S14–S15]
- For VRChat, output timestamped virtual trackers through a SteamVR/OpenVR driver and/or supported OSC pathways; let VRChat IK handle public avatars. [S12–S13, S82]

A useful conceptual objective is:

```text
E_avatar =
    w_contact   * contact_error
  + w_balance   * balance_and_center_of_mass_error
  + w_direction * limb_direction_and_facing_error
  + w_reach     * hand_and_foot_target_error
  + w_pose      * human_intent_pose_error
  + w_limits    * avatar_joint_limit_penalty
  + w_smooth    * temporal_jerk_and_pop_penalty
```

Weights vary by avatar proportions and motion mode. The retargeter may intentionally deviate by centimeters from reconstructed human coordinates when that makes the avatar’s body language and contacts look more faithful.

## 5.5 Instrument and prop mode

Guitar is a particularly strong north-star test because a convincing result requires more than generic pose tracking.

### Additional observations

- Detect or segment the guitar body and neck in each view.
- Estimate a rigid or articulated prop pose relative to Site State. A user may select a rough guitar template or perform a short “hold instrument” initialization; a visual marker is optional, never required.
- Track left-hand contact on the neck, right-hand contact/strumming region, forearm orientation, shoulder elevation, torso lean, and the instrument’s support against the body.
- Use audio onset or rhythm only as an optional weak cue for strumming timing; visual evidence remains primary.

### Contact graph

```text
performer torso/pelvis ---- supports ---- guitar body
left hand ---------------- contacts ---- neck / fret region
right forearm/hand -------- contacts ---- body / string plane
feet ---------------------- contact  ---- floor
```

This graph gives the solver extra constraints during occlusion. If the left wrist is hidden, the guitar neck and visible elbow sharply limit plausible hand locations. If individual fingertips are unresolved, the system should preserve stable palm/wrist placement and generate a conservative finger pose rather than random high-frequency motion.

### Success criterion

The viewer should believe the avatar is holding and playing the instrument with correct gross timing, reach, elbow path, wrist orientation, and body rhythm. Exact physical fret selection is a higher-resolution subproblem and should be reported separately rather than allowed to invalidate otherwise convincing full-body tracking.

## 5.6 Two operating modes

| **Mode**       | **Inputs**                                                                                       | **Output strategy**                                                                        |
|----------------|--------------------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------|
| Desktop VTuber | Cameras provide head, body, hands and face.                                                      | Direct skeleton + expressions to VRM/VMC; maximum model awareness.                         |
| VRChat in VR   | HMD/controllers are trusted head/hand anchors; cameras fill body and optional elbows/knees/feet. | Predict camera body state to the HMD render time; emit virtual trackers and face channels. |

# 6. Performance and hardware scaling

The avatar output rate and expensive perception rate are separate controls.

## 6.1 Realistic update rates

| **Stage**                 | **Rate**                            | **Reason**                                                                                               |
|---------------------------|-------------------------------------|----------------------------------------------------------------------------------------------------------|
| Camera capture            | 30–60 FPS                           | Use the highest reliable, low-blur cadence the device and USB path sustain.                              |
| Fast 2D body tracking     | 30–60 Hz                            | Cheap enough to maintain temporal evidence from every active camera.                                     |
| Heavy 3D body observation | 12–30 Hz typical; 30–45 Hz high-end | 15 Hz can work for seated/talking; 20–30 Hz is the practical default; fast dance benefits from 30–45 Hz. |
| Face / eye / mouth        | 30–60 Hz                            | Perceptually sensitive; prioritize over a redundant extra body mesh update.                              |
| Hand detail               | 15–45 Hz                            | Adaptive: run more often when hands are large/visible and gesturing.                                     |
| Fused temporal state      | 30–60 Hz                            | Update on every incoming observation and prediction tick.                                                |
| Avatar / tracker output   | 60–90 Hz                            | Causal prediction to current/render time; correction after observations arrive.                          |

> **Practical answer to “how many Hz?”**
>
> A trustworthy 25–30 Hz body-observation stream with a good causal model is enough for strong normal VTuber motion and much dancing. Fifteen to twenty Hz is usable for seated/talking modes. Forty-five Hz is valuable for fast dance. Sixty giant-model inferences per camera are not required; low latency, low motion blur, and predictable correction matter at least as much.

## 6.2 Why interpolation must be predictive

Conventional interpolation between pose A and a future pose B waits for B and therefore adds one inference interval of latency. The runtime instead predicts forward from pose, velocity, acceleration, contact and motion history; when B arrives it corrects smoothly. Recording mode may optionally use non-causal smoothing because latency is irrelevant.

## 6.3 Compute policy as cameras increase

| **Mode**    | **Scheduling strategy**                                                                                                                                                   |
|-------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| 1 camera    | One compact body encoder; optional periodic high-quality teacher on powerful GPUs; face and hand crops share features where possible.                                     |
| 2 cameras   | Batch the two views; run heavy observations at 15–25 Hz each or stagger 20 Hz per view to deliver a strong observation roughly every 25 ms; keep 2D paths full rate.      |
| 3–4 cameras | Run lightweight landmarks/features on all views; choose one or two heavy views dynamically; rotate heavy coverage as visibility changes.                                  |
| 5+ cameras  | Prioritize visibility diversity rather than processing every pixel. Downscale non-primary views, schedule hand/face cameras separately and expose an advanced budget cap. |

## 6.4 Provisional hardware tiers

| **Tier**                | **Behavior**                                                                    | **Provisional target**                                                                                         |
|-------------------------|---------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| Minimum / compatibility | 1×720p30; body 12–18 Hz; output 60 Hz; limited hand detail.                     | Older gaming GPU or modern integrated accelerator. Benchmark required; CPU-only not a primary target.          |
| Balanced                | 1–2×1080p30; body 20–30 Hz; face 30–60; adaptive hands.                         | RTX 3060/4060-class or equivalent is a reasonable design target, not a guaranteed requirement until profiling. |
| Performance             | 2–4 cameras; 30–45 Hz body observations; more hand refinement; 90 Hz VR output. | RTX 4070/3080-class or better; VR workload headroom remains the binding constraint.                            |
| Studio / developer      | 4+ views, high-resolution recording, optional offline teacher/cleanup.          | High-end GPU or second GPU; no promise that all views use the heavy model simultaneously.                      |

*Hardware labels are architecture targets. Real requirements must be established by repeatable benchmark scenes and include the cost of the VR game/render workload—not model inference in isolation.*

## 6.5 Poor-webcam mitigation—and the irreducible limits

| **Mechanism**            | **Effect**                                                                                                         |
|--------------------------|--------------------------------------------------------------------------------------------------------------------|
| Auto exposure/blur test  | Recommend brighter light or shorter exposure; detect when nominal 60 FPS is merely dark/noisy.                     |
| Lens model estimation    | Correct common distortion and crop inconsistencies during calibration.                                             |
| Per-camera normalization | Handle color, sharpness and white-balance mismatch before feature fusion.                                          |
| Adaptive resolution      | Use full resolution only for needed body/face/hand regions.                                                        |
| Confidence weighting     | A poor camera can still help one visible ankle without contaminating every joint.                                  |
| Hard limit               | No algorithm reconstructs fingers absent from the pixels or a foot hidden from all views with guaranteed accuracy. |

# 7. Training and data strategy

Synthetic data is not merely a fallback; it is the only inexpensive source of perfect multi-camera labels.


```text
Licensed/public motion + licensed VTuber clips + synthetic motions
                           |
                           v
Blender synthetic capture worlds
  - unseen human identities and body shapes
  - randomized avatars and retargeting targets
  - 1..8 cheap-camera simulations
  - face, hands, guitar/props, contacts, occlusion
  - exact cameras, mesh, joints, masks, depth, timing
                           |
                           v
Privileged offline teacher
  - SAM/HMR body teachers
  - face and hand teachers
  - exact multi-view geometry
  - temporal/contact optimization
  - avatar-aware retargeting
                           |
                           v
Compact person-agnostic runtime student
  - shared per-view encoder
  - ephemeral performer adapter, no gradient updates
  - set-based multi-view fusion
  - causal temporal/contact model
  - direct avatar or virtual-tracker output
                           |
                           v
Target: 20–30 Hz trustworthy observations and 60–90 Hz stable output
on ordinary gaming hardware, with quality scaling by camera count and GPU.
```


## 7.1 Training objective hierarchy

| **Loss family**      | **Purpose**                                                                                                      |
|----------------------|------------------------------------------------------------------------------------------------------------------|
| Observation fidelity | 2D reprojection, 3D joint/rotation error, mesh/silhouette consistency, camera consistency.                       |
| Temporal quality     | Velocity, acceleration and jerk regularization; no high-frequency body-shape change; causal prediction accuracy. |
| Contact quality      | Foot and hand contact classification; stationary-contact velocity loss; floor penetration/hover penalties.       |
| Anatomy and avatar   | Session-constant bone length; joint limits; retargeted contact and balance; model-specific rest-pose consistency. |
| Subject generalization | Correct pose/shape on held-out identities, clothing, body types, skin tones, and face/hand appearances. |
| Hot-swap behavior | Fast acquisition of a new person, zero carryover from the previous body code, and stable state reset. |
| Failure behavior     | Uncertainty calibration, outlier rejection, occlusion continuation and smooth reacquisition.                     |
| Perceptual motion    | Sequence-level discriminator/critic trained on good motion, plus human blind-rating data later.                  |

## 7.2 Synthetic data generator

- Build synthetic **sites** first: fixed floor, walls, desks, chairs, microphones, lights, clutter, and one to eight cameras with random azimuth, elevation, distance, focal length, distortion, crop, and partial framing.
- Within the same fixed Site State, cycle through many different synthetic performers. Randomize broad stature and body proportions, clothing, hair, skin tone, face shape, glasses, facial hair, hand shape, and movement style. Include explicit person-exit/person-entry transitions so state reset is supervised.
- Randomize Avatar Profiles independently from humans: realistic, tall, short, chibi, long-arm, short-leg, large-head, large-foot, and unusual rest-axis rigs. Generate the desired retargeted output, not only the human skeleton.
- Include desks, chairs, microphones, controllers, guitars, keyboards, and foreground occluders. Export prop geometry, rigid pose, hand/forearm contact regions, and support relationships.
- Simulate cheap-webcam artifacts: motion blur, rolling shutter, low light, sensor noise, compression, oversharpening, color mismatch, autofocus changes, dropped/repeated frames, mixed frame rates, time offsets, and USB-induced stalls.
- Export exact camera parameters, mesh, skeleton, 2D landmarks, visibility, depth, masks, optical flow, contacts, floor, prop transforms, face coefficients, hand pose, anonymous performer-boundary labels, and target avatar animation.
- Include degenerate camera placements and broken cameras so the placement/quality model learns to reject them rather than forcing fusion.
- Render realistic humans for perception transfer and stylized avatars for retarget/perceptual critics. The runtime sees real humans, but avatar output training must cover the non-human proportions the product exists to support.

## 7.3 Person-agnostic training and anti-overfitting rules

The model must adapt to a person without learning that person.

### Architecture rules

- **Global weights only:** perception, fusion, temporal, face, hand, and retargeting networks are frozen during normal runtime.
- **Amortized body code:** a small adapter predicts current proportions from a short observation window. The code is input conditioning, not a weight update.
- **Canonical motion latent:** pose/motion features are normalized by estimated body proportions so a tall and short person performing the same action map to similar motion intent.
- **Separate face identity from expression:** face features should be decomposed into stable shape/appearance and changing expression; only normalized expression drives the avatar.
- **View and identity dropout:** training randomly removes cameras, hides limbs, changes clothing, and swaps identities while preserving Site State.
- **Hard reset token:** the temporal model receives an explicit new-performer boundary and is trained to clear old velocity/contact/body context immediately.

### Data split rules

- Split humans, face assets, clothing combinations, motion performers, and body-shape seeds across train/validation/test.
- Never let rendered views of the same synthetic identity appear in both training and the “unseen person” test split.
- Hold out entire camera models/artifact presets, rooms, and avatar proportion families to test generalization rather than memorization.
- Include a **calibrator/performance split**: Site State is solved using Person A, but the supervised tracking loss is evaluated on Persons B–N.

### Losses and regularizers

| Loss | Purpose |
|---|---|
| Body-code consistency | The same visible person should produce stable proportions across poses and views |
| Cross-person motion consistency | Equivalent normalized motions should map to equivalent canonical pose regardless of body shape |
| Identity leakage penalty | Motion latent should not make it easy to classify performer identity beyond what is required for proportions |
| New-person reset loss | Output after a handoff must not depend on the previous subject’s pose, contacts, or body code |
| Fast adaptation loss | Reach acceptable proportions and retargeting within a short prefix of unseen-person frames |
| Calibration-transfer loss | A Site State estimated with one synthetic person must reconstruct other people without geometry changes |

The goal is not to erase legitimate anatomical differences. It is to isolate them in an explicit ephemeral body code instead of letting the whole tracker become person-specific.

## 7.4 Public data that is useful

| **Dataset**                                 | **Strength**                                  | **Use**                                                | **License caution**                                                      |
|---------------------------------------------|-----------------------------------------------|--------------------------------------------------------|--------------------------------------------------------------------------|
| AMASS                                       | Large motion archive                          | Body motion prior, locomotion and transitions.         | Component licenses vary; do not assume redistributable. [S64]          |
| AIST++                                      | Multi-view dance                              | Dance timing, turns and choreography.                  | Review media/annotation terms; cite and separate derived assets. [S65] |
| Motion-X                                    | Whole-body motion including hands/face        | Broad motion prior and expressive coordination.        | Academic/research restrictions may apply. [S66]                        |
| BEDLAM / SynBody / Syn4D                    | Synthetic human imagery and multi-view labels | Domain randomization, camera and geometry pretraining. | Dataset-specific terms; some are research-only. [S67–S69]              |
| ARCTIC / OakInk2 / GRAB / BEHAVE / InterCap | Hands/body/object interaction                 | Props, bimanual actions and contact reasoning.         | Mostly research datasets; use only under compatible terms. [S70–S74]   |

## 7.5 How existing VTuber output videos can help

- They are weak supervision for the original human body because retargeting and animation have already altered the motion.

- They are useful for learning the distribution of visually convincing avatar gestures, idle motion, head/body coordination, dance style and smoothing.

- Extract approximate avatar skeletons, clean them into motion clips, retarget those clips onto many synthetic humans and avatar rigs, then render new camera views. The newly rendered sequence has exact ground truth even if the original extraction was approximate. This turns final-output clips into a motion-distribution source rather than pretending they reveal the original performer’s exact skeleton.

- Use only clips with explicit permission or a license that permits training. Public visibility is not permission. [S75–S76]

## 7.6 Teacher–student plan

| **Stage**                              | **Action**                                                                                                                         |
|----------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| Teacher observation                    | Run SAM-class/HMR models, hand/face teachers and exact synthetic labels; optionally fit an expensive offline multiview body.       |
| Geometry teacher                       | Use known or solved cameras, robust triangulation, temporal optimization and contact constraints to produce a clean body sequence. |
| Student perception                     | Distill image features and pose distributions into a compact shared encoder.                                                       |
| Student fusion                         | Train with random camera count, view dropout, timing jitter and calibration error.                                                 |
| Student temporal/retarget              | Predict current/render-time avatar state and imitate the teacher’s cleaned output rather than raw mesh estimates.                  |
| Performer Adapter                        | Distill rapid body/face normalization across held-out identities; train explicit reset and calibration-transfer behavior.          |
| Fine-tune on unlabeled real multi-view | Use reprojection, cross-view agreement, cycle consistency and temporal losses without requiring mocap labels.                      |

## 7.7 Community data flywheel—only after the product is useful

- Debug capture is off by default. Local pose-only logs are preferred over raw video.

- Opt-in contributors may submit synchronized camera clips plus tracker/IMU reference data with explicit consent and clear deletion terms.

- Provide scripted 5–10 minute capture routines covering turns, crouches, chairs, dance and hands near face.

- Separate evaluation contributors from training contributors to avoid benchmark leakage.

- Never depend on community uploads to make the first useful prototype; that would stall the hobby project.

## 7.8 Licensing and legal posture

> **Dataset rule**
>
> A free project does not make research-only datasets or creator videos automatically legal to train on or redistribute. Maintain a machine-readable data ledger containing source, license, permitted use, attribution, derived artifacts, and whether resulting weights may be distributed. When uncertain, exclude the source or obtain permission.

- Prefer self-generated synthetic assets, permissively licensed motion, public-domain/CC-compatible media and opt-in recordings.

- Keep third-party raw datasets out of the repository; provide acquisition scripts only where terms permit.

- Review every foundation-model license before distributing derivative weights, conversion code or bundled checkpoints.

- This design is not legal advice; the U.S. Copyright Office’s AI-training report illustrates that the legal landscape remains fact-specific. [S75]

# 8. Quality assurance and evaluation

A good demo is insufficient; the project needs adversarial replay, quantitative failure metrics and perceptual comparison.

## 8.1 Standard test suite

| **Scenario**      | **Motion**                                                | **Measure**                                                 |
|-------------------|-----------------------------------------------------------|-------------------------------------------------------------|
| Stillness         | Neutral stand for 60 seconds                              | Root/foot jitter, shape stability, false motion.            |
| Occlusion         | Hands behind torso, crossed arms/legs, one camera blocked | Confidence behavior, no snapping, recovery continuity.      |
| Transitions       | Stand↔sit, crouch, kneel, floor rise                      | Contact state, pelvis path, knee direction.                 |
| Dance             | Spins, fast steps, kicks, arm crossings                   | Latency, blur robustness, view switching, foot release.     |
| Turnaround        | 180° and 360° turns                                       | Camera fusion and rear-view coverage.                       |
| Instrument        | Guitar/keyboard/controller/prop                           | Elbow/wrist/hand contact and occlusion.                     |
| Avatar extremes   | Chibi, tall, long arms, short legs, large feet            | Retargeting and VRChat IK target generation.                |
| Degraded hardware | Low light, 720p, mixed FPS, dropped frames                | Graceful quality reduction and diagnostics.                 |
| Long session      | 30–120 minutes                                            | No drift, memory leak, calibration creep or rising latency. |
| Calibration transfer | Person A calibrates the site; Persons B–J perform | Site geometry remains valid; no person-specific bias or camera re-solve. |
| Guest hot-swap | Ten people enter/leave back-to-back | Fast reset, no old contacts/shape, deterministic time-to-good. |
| Human diversity | Held-out heights, proportions, clothing, skin tones, hair, glasses | Stable acquisition and quality without per-user training. |

## 8.2 Metrics

| **Metric**                       | **Definition**                                                                                     |
|----------------------------------|----------------------------------------------------------------------------------------------------|
| Median / P95 end-to-end latency  | Timestamp from exposure/capture to emitted avatar state; report VR and desktop separately.         |
| Pose observation cadence         | Actual successful updates, not requested inference rate.                                           |
| Catastrophic event rate          | Joint flip, root teleport, severe tracker loss or visible reset per hour.                          |
| Contact slip                     | Foot/hand velocity while contact probability and ground truth/annotation say planted.              |
| Root drift / pumping             | Uncommanded world-space movement during stillness.                                                 |
| Occlusion recovery discontinuity | Position/rotation jump when a joint reappears.                                                     |
| Confidence calibration           | Whether predicted uncertainty matches actual error/failure likelihood.                             |
| Perceptual rating                | Pairwise preference and “professional/consumer” identification after identical avatar retargeting. |
| Setup success                    | Time to first valid pose; percentage of users completing without technical help.                   |
| Time-to-good for unseen person   | Time from entering the volume to stable proportions, root, contacts, face, and avatar output.       |
| Prior-person leakage             | Difference in Person B output caused by which person used the system immediately before them. Target: statistically negligible after reset. |
| Calibration transfer error       | Change in reconstruction quality when Site State is solved by a different person.                    |

## 8.3 Reference systems for comparison

- Free webcam baseline: XR Animator / MediaPipe-style pose. [S16, S19]

- Current consumer camera software: Webcam Motion Capture, MocapForAll, FreeMoCap, Remocapp and PoseCap where available. [S20–S27]

- Wearable consumer baseline: SlimeVR/Haritora/mocopi and a three-point optical/inside-out setup. [S30–S42]

- High-end reference: borrowed or rented six-plus Lighthouse trackers, inertial suit or professional optical sequence. [S37–S51]

- All outputs retargeted to the same avatars and rendered with identical cameras to prevent presentation bias.

## 8.4 Perceptual study design

1.  Record the same motion simultaneously with the proposed camera system and reference hardware.

2.  Retarget both through a controlled, equivalent avatar pipeline; do not give one system a superior cleanup pass.

3.  Randomize side and label; collect preference, naturalness, responsiveness and visible-error annotations.

4.  Stratify by motion class and avatar proportions. A system can reach parity for standing/talking yet fail dance or floor work.

5.  Include unseen performers and cases where the calibration person is not the performer.

6.  Publish failure clips and confidence traces, not only the best take.

# 9. Implementation roadmap

The project should prove the risky ideas in the cheapest order.

| Milestone | Exit condition |
|---|---|
| Phase 0 — Reproducible benchmark harness | Record/replay synchronized videos; render overlays; measure latency, contacts, person boundaries, and severe events before building a polished UI |
| Phase 1 — One-camera offline baseline | Evaluate current HMR models, session-lock body proportions, add temporal/contact cleanup, export VRM/VMC, and test on at least five people not used for any tuning |
| Phase 2 — One-camera live proof | Run 15–30 Hz body observations, 60 Hz output, dedicated face path, hand crops, and VRChat virtual tracker prototype |
| Phase 3 — Two-camera geometry with explicit developer calibration | Use known calibration during development; triangulate/fuse and demonstrate a clear improvement over either camera alone on dance, turns, sitting, and guitar |
| Phase 4 — Transferable human-motion site calibration | Replace target board with guided motion; estimate camera pose, timing, floor, and quality; prove Person A calibration works for Persons B–N |
| Phase 5 — Guest hot-swap and Performer Adapter | Instant/Quality Guest modes, explicit state reset, rapid unseen-person body fit, and demo/kiosk workflow |
| Phase 6 — Learned N-view fusion | Train a set-based confidence model on synthetic 1–8 camera data with view dropout, person swaps, and geometry as a sanity constraint |
| Phase 7 — Avatar-aware retargeting and failure containment | Unity profile exporter, extreme avatar tests, contact solver, safe recovery, model-size adaptation, and diagnostics |
| Phase 8 — Face, hand, and instrument quality pass | Dynamic view selection, hand crops, face normalization, VRCFaceTracking bridge, guitar/prop mode |
| Phase 9 — Distillation and packaging | Compact runtime student, ONNX/TensorRT/DirectML backends, benchmark-driven presets, installer, crash-safe local logging |
| Phase 10 — Community beta | Opt-in hardware reference captures, public test suite, transparent quality dashboard, model card, and license audit |

## 9.1 The first decisive experiment

> **Do this before training a fusion network**
>
> Record two ordinary webcams with meaningful angular separation. Use known calibration or a ChArUco board only for this developer experiment. Run strong 2D landmarks and a monocular 3D prior, triangulate/fuse offline, and compare against each single view on feet, pelvis depth, crossed limbs, turns, and guitar handling. Then repeat with a different person from the one used during setup. If two-view fusion is not dramatically more stable—or the second person causes the system to collapse—diagnose that before building the product.

## 9.2 Recommended hobby-project scope

- Windows first; NVIDIA acceleration first if necessary, but isolate inference behind an interface so DirectML/other backends can follow.

- One **active** performer at a time, fixed cameras, humanoid avatars, and person-agnostic hot-swap. Simultaneous multi-person capture and persistent identity recognition are out of scope.

- Body first, face baseline second, high-fidelity fingers later. Do not let exact hand tracking block the body proof.

- Use existing pretrained models and geometry before creating a custom image backbone.

- Publish recorded benchmark clips and a deterministic replay tool so contributors can improve models without owning the full hardware setup.

## 9.3 Suggested module boundaries

| Module | Responsibility |
|---|---|
| capture | Camera discovery, decoding, timestamps, buffers, device health |
| site | Site State estimation, versioning, invalidation, geometry/timing/floor solve, placement score |
| performer | Anonymous active-track management, quick body/face fit, handoff/reset, no persistent identity |
| perception | Body, face, hand, prop, and segmentation backends with a common observation schema |
| fusion | Robust triangulation, set model, uncertainty, camera quality, and geometric residuals |
| temporal | Causal prediction, contacts, motion states, safe recovery, and render-time prediction |
| avatar | Profile import/export, morphology-aware retargeting, VRM/VMC, VRChat drivers, expression mapping |
| ui | Wizard, guest mode, quality maps, overlays, presets, diagnostics, and privacy controls |
| evaluation | Replay, metrics, reference alignment, person-transfer tests, test scenes, and reports |

# 10. Risks, non-goals, and honest limits

The project can be ambitious without pretending the information problem disappears.

## 10.1 Highest technical risks

| **Risk**                           | **Why it matters**                                                                                        | **Mitigation**                                                                                     |
|------------------------------------|-----------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------|
| Runtime cost while VR is rendering | VRChat already consumes GPU. A model that is “real-time” alone may be unusable alongside the application. | Distill, batch, stagger views, adaptive detail paths, benchmark in-game.                           |
| Auto-calibration fragility         | Low motion, similar camera positions or poor visibility can create ambiguous geometry.                    | Guided poses, quality score, robust solve, cache calibration, ask for a specific corrective move.  |
| Synthetic-to-real gap              | Perfect renders do not reproduce webcam noise, clothing and clutter.                                      | Use pretrained real-image teachers, domain randomization and unlabeled real multiview consistency. |
| Hands around props                 | Fingers can be too small or hidden.                                                                       | Dedicated crops/views, stable gesture priors, confidence and optional hand camera.                 |
| VRChat interface ceiling           | External software cannot directly control every bone of arbitrary public avatars.                         | Emit all supported tracker roles; fuse HMD/controllers; provide custom-avatar profile/export path. |
| Dataset licensing                  | Many popular research datasets are non-commercial/research-only.                                          | Data ledger, permissive synthetic pipeline, no unlicensed creator scraping.                        |
| Person-specific leakage            | A body code, face normalization, or calibration subject can silently bias all later guests.                | Strict state partition, no runtime fine-tuning, handoff reset, calibrator/performance holdout tests. |
| Guest acquisition instability      | A new person may be misdetected or inherit stale contacts/velocity.                                         | Explicit state machine, neutral hold during reset, fast-fit confidence gate, manual New Guest button. |
| Expectation inflation              | “As good as \$20k mocap” can be interpreted as measurement equivalence.                                   | Publish bounded claims, blind tests and failure cases.                                             |

## 10.2 Non-goals for version 1

- Biomechanics, medical measurement, robotics ground truth or certified centimeter/millimeter accuracy.

- Guaranteed exact pose when a limb is hidden from every camera.

- Simultaneous multi-person capture, large traveling volumes, moving cameras, or persistent biometric identity recognition.

- Photorealistic body shape scanning or cloth simulation.

- Perfect finger reconstruction from a distant, blurry full-body image.

- Supporting every avatar format and every GPU backend at launch.

## 10.3 Claim language that remains defensible

| **Status**               | **Language**                                                                                                 |
|--------------------------|--------------------------------------------------------------------------------------------------------------|
| Good                     | “High-quality, wearable-free avatar tracking from ordinary webcams.”                                         |
| Good after evidence      | “Two-camera mode matches or beats common consumer FBT in perceived avatar quality for our tested motions.”   |
| Good after transfer tests | “The same calibrated camera setup works for unseen people without retraining or camera recalibration.”       |
| Aspirational study claim | “Viewers could not reliably distinguish the output from the professional reference in the bounded test set.” |
| Avoid                    | “More accurate than OptiTrack,” “works from any random camera placement,” or “never glitches.”               |

> **Core judgment**
>
> This is a credible, differentiated open-source project. The highest-probability win is not universal mocap replacement; it is making $200–$700 wearable FBT hard to justify for camera-visible VTubers and VRChat users by producing equal-or-better-looking avatar motion with one or two webcams. Multi-camera Studio mode can push much closer to professional-looking output, provided temporal, contact, retargeting, runtime, and cross-person generalization are engineered rigorously.

**END OF CORE DESIGN SPECIFICATION**

The following sections map market grievances to design responses and provide the detailed market sweep requested.

# 11. Complaint-to-fix matrix

Every recurring grievance becomes a requirement, mitigation and test—not a marketing bullet.

| **Recurring grievance**                                                   | **Where seen**                                | **Design response**                                                                                                                                                                    | **Verification**                                                     | **Residual risk**                                                    |
|---------------------------------------------------------------------------|-----------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------|----------------------------------------------------------------------|
| The tracker only works well for the person who calibrated or trained it. | Personalized demos and brittle research prototypes | Separate Site State, Avatar Profile, Performer State, and Temporal State; prohibit runtime weight updates; train with identity holdouts and subject swaps; explicit New Guest reset. | Person A calibrates; Persons B–J enter in random order; compare quality and body-code carryover. | High if architecture and datasets enforce it; otherwise a major hidden failure. |
| Monocular depth is wrong; hips pump forward/back and feet float.          | Webcam / MediaPipe / single-view HMR          | Session-level fixed anatomy; floor solve; world-root model; in 2+ camera mode use robust triangulation and silhouette constraints.                                                          | Stillness + forward/back step; measure root pumping and foot height. | High in 2+ views; partial in 1 view.                                 |
| Tracking jitters even when standing still.                                | Webcam, depth and markerless systems          | Uncertainty-aware temporal filter, shape lock, camera outlier rejection, contact lock and separate output-rate prediction.                                                             | 60-second stillness test with root/joint velocity thresholds.        | High.                                                                |
| A limb suddenly flips or teleports when occluded/reacquired.              | Webcam / inside-out / optical loss            | Innovation gating, joint limits, confidence hysteresis, motion prediction and smooth reacquisition; no direct use of low-confidence point estimates.                                   | Cross arms/legs and block each camera in turn; count severe events.  | High if implemented rigorously.                                      |
| Feet skate; dancing looks robotic or legs stay stiff.                     | Sparse IK, IMU, webcam                        | Explicit contact probabilities, floor/world constraints, motion-state prior, avatar-aware knee/pelvis solve and dance-heavy training.                                                  | Dance suite and planted-foot slip metric; blind naturalness rating.  | High for visible motions.                                            |
| Repeated calibration and recentering breaks immersion.                    | IMU and many camera systems                   | Guided site calibration once, cache camera geometry and Avatar Profile, estimate each guest’s proportions transiently, refine safely, and avoid accumulated yaw integration.                                                                       | Two-hour session; camera unchanged between restarts.                 | High.                                                                |
| IMUs drift or need yaw resets.                                            | SlimeVR / Haritora / mocopi / inertial suits  | Vision supplies an absolute room reference every observed frame; camera geometry does not integrate gyro error. Optional IMUs may be fused only as complementary evidence. [S31–S34] | Long-session heading stability and turn/recenter test.               | High while visible.                                                  |
| Straps slip, trackers are hot, need charging, or take time to wear.       | IMU and optical trackers                      | No body-worn hardware in camera-only mode; optional HMD/controllers already worn for VR.                                                                                               | Setup-time and session-maintenance survey.                           | Fully addressed.                                                     |
| Base stations, dongles, reflections and line-of-sight are annoying.       | Lighthouse tracking                           | No base stations/dongles; ordinary cameras with per-view confidence. Multiple views reduce body line-of-sight loss.                                                                    | Home-room setup test and deliberate single-view block.               | Mostly; cameras still need light/visibility.                         |
| Inside-out trackers jump or fail in difficult lighting/featureless rooms. | VIVE Ultimate and similar                     | Body-centric camera evidence, redundant views, outlier rejection, no tracker-mounted SLAM map. App warns about blur/exposure. [S40]                                                  | Lighting sweep and sudden-disagreement test.                         | Mostly; severe darkness remains a limit.                             |
| Marker-board calibration is too technical.                                | MocapForAll / FreeMoCap / general multiview   | Use the moving human as calibration target; plain-language pose wizard; quality score and automatic timing solve. [S09, S23, S25]                                                    | First-time novice setup completion rate.                             | High, research risk remains.                                         |
| Multi-camera setups require exact measurements and synchronized cameras.  | Traditional multi-view mocap                  | Estimate camera pose and time offset from motion; short buffers and timestamp alignment; accept approximate placement.                                                                 | Mixed webcam models/frame rates; report solve confidence.            | High if motion has enough excitation.                                |
| Avatar proportions cause bent knees, floating feet or strange reach.      | VRChat IK and generic retargeting             | Avatar Profile, rest-axis validation, joint limits, model-specific tracker target optimization and crouch/reach calibration.                                                           | Extreme avatar test matrix.                                          | High for custom/imported avatars; partial for public avatars.        |
| Sitting, lying down and floor poses fail.                                 | Software estimates and monocular systems      | Train explicit motion states, chair/floor contacts and transitions; use side/rear views and state-specific priors.                                                                     | Sit/stand/kneel/prone suite.                                         | Moderate–high with 2+ views.                                         |
| Hands disappear near the torso or props.                                  | Webcam and optical hand tracking              | Original-resolution crops, multi-view hand fusion, wrist/body context, hand-object contact and stable fallback gestures.                                                               | Hands-near-face and guitar/keyboard suite.                           | Moderate; exact hidden fingers remain impossible.                    |
| Webcam face is less expressive than iPhone tracking.                      | Webcam VTuber software                        | Dedicated high-rate face model, dynamic best-view selection, per-avatar expression calibration, optional audio visemes and VRCFaceTracking bridge.                                     | Blind face-expression comparison; glasses/lighting suite.            | Moderate–high with sufficient face pixels; TrueDepth may still lead. |
| I need several apps and bridges for body, face and hands.                 | Current VTuber stacks                         | One capture/calibration/fusion application with VRM/VMC, SteamVR tracker and face bridges; modular backends behind one UI.                                                             | Fresh-machine install and end-to-end setup time.                     | High.                                                                |
| Cloud mocap has credits, uploads and delays.                              | DeepMotion/Rokoko Vision/other video services | Local inference, no mandatory account, no per-minute credits; offline recording cleanup optional.                                                                                      | Network-disconnected operation.                                      | Fully addressed if model licenses permit redistribution.             |
| Extra cameras multiply GPU use linearly.                                  | Naïve multiview HMR                           | Heavy model on selected views; all-view lightweight landmarks/features; batching, staggered updates and adaptive hand/face crops.                                                      | 1–6 camera scaling benchmark with VR workload.                       | High design priority.                                                |
| Cheap cameras have blur, mismatched colors and dropped frames.            | Commodity camera capture                      | Device health test, exposure advice, per-view normalization, timestamp ring buffers, stale-frame rejection and confidence weighting.                                                   | Mixed-camera stress suite.                                           | Partial; no software can restore absent detail.                      |
| When tracking fails, it fails without explanation.                        | Many consumer systems                         | Visible confidence map, camera quality scores, deterministic replay, event log and explicit “observed vs inferred” state.                                                              | User-diagnosis study and issue-report completeness.                  | High.                                                                |

*Community evidence is directional. Individual Reddit/Steam reports are not prevalence estimates; official support and architecture documents receive more evidentiary weight. [S20–S45, S77–S80]*

# 12. Comprehensive market sweep

Current options across price, technology and quality—from free software estimation to professional optical stages.

## 12.1 Market map by price and technology

| **Category**                     | **Typical public cost**                   | **Inputs**                   | **Why it sells**                                    | **Recurring limitations**                                                    |
|----------------------------------|-------------------------------------------|------------------------------|-----------------------------------------------------|------------------------------------------------------------------------------|
| Software-only estimation         | ~\$0–\$20                                 | HMD/controller or one webcam | No wearables; lowest cost.                          | Inferred lower body/depth, stiff or unstable legs, limited occlusion.        |
| Single-webcam avatar tracking    | \$0–low subscription                      | One RGB camera               | Fast setup; face/body in one frame.                 | Monocular depth, full-body framing, feet/root and occlusion.                 |
| Commodity multi-webcam           | ~\$60–\$300 hardware + \$0–\$120 software | 2+ RGB cameras               | Real geometry and occlusion coverage at low cost.   | Calibration targets, room/USB setup, product polish and stability.           |
| Consumer IMU                     | ~\$219–\$1,180                            | 5–12 wearable sensors        | 360° orientation, no camera visibility needed.      | Yaw drift, inferred positions, straps, charging, calibration.                |
| Lighthouse / inside-out trackers | ~\$689–\$1,500+                           | 3–10 body trackers           | True 6DoF points and good fast motion.              | Cost, wearables, battery, line of sight/environment, sparse IK.              |
| RGB-D / depth                    | ~\$419–\$600+                             | One depth camera + software  | Metric depth and easy scale/floor.                  | Front-view occlusion, range/FOV, IR/driver ecosystem, discontinued hardware. |
| Professional inertial suit       | ~\$1,500–\$5,000+                         | Full wearable suit           | Portable, many segments, 360°.                      | Cost, drift/global position, suit fit, calibration and cleanup.              |
| Professional optical             | Five figures to much more                 | Many cameras + markers       | Highest raw measurement quality and capture volume. | Cost, space, calibration, line of sight, technical operation.                |

## 12.2 Closest direct camera/software competitors

| **Product**                          | **Price**                                                       | **Cameras**                  | **What it does**                                                        | **Strength**                                                   | **Main gap**                                                                                                              | **Refs**      |
|--------------------------------------|-----------------------------------------------------------------|------------------------------|-------------------------------------------------------------------------|----------------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------|---------------|
| XR Animator / System Animator Online | Free/open                                                       | 1 webcam                     | MediaPipe/TensorFlow.js body, face/hand/avatar animation.               | Lowest-friction baseline; broadly available.                   | Monocular depth and landmark-to-avatar limits; not modern multiview HMR.                                                  | S16, S19      |
| Webcam Motion Capture                | Low monthly subscription (publicly advertised around \$1.99/mo) | 1 webcam                     | Commercial turnkey full-body/fingers/face to VTuber tools.              | Very accessible one-camera product.                            | Single-view depth/occlusion; proprietary solver; full-body framing.                                                       | S20           |
| PoseCap                              | Free/open, early development                                    | 1 webcam                     | Real-time SMPL-X-oriented capture.                                      | Modern monocular direction; useful code/reference.             | Project notes root/world translation limitations and early-stage hardware expectations.                                   | S21           |
| MocapForAll                          | \$119.99 public Steam list price                                | 2+ webcams                   | Realtime multi-camera to VMC/SteamVR; ordinary cameras.                 | Closest established consumer multi-webcam concept.             | Explicit marker calibration, room/camera setup, paid app; community reports mention jitter/scale/calibration issues.      | S22, S23, S77 |
| FreeMoCap                            | Free/open                                                       | Multiple webcams             | Open multi-camera 3D capture infrastructure.                            | Strong community/scientific foundation and reproducibility.    | ChArUco calibration, synchronization/setup burden, not a polished low-latency VRChat one-stop product.                    | S24, S25      |
| Remocapp                             | Subscription tiers; check current regional pricing              | 1–8 webcams                  | Realtime multicamera, common DCC/game integrations, VMC/SteamVR claims. | Very close camera-count scalability.                           | Proprietary/subscription, explicit setup/calibration, limited independent public validation compared with older products. | S26, S27      |
| MocapForStreamer                     | Roughly tens of dollars; storefront pricing varies              | 2 webcams                    | Niche VTuber-focused multi-webcam software.                             | Directly aligned audience.                                     | Small ecosystem, calibration/setup, limited public evidence of robust modern fusion.                                      | S28           |
| Kineo                                | Research/open code                                              | Sparse consumer RGB cameras  | Automatic calibration, unsynchronized cameras, metric reconstruction.   | Strong validation of the proposed human-only calibration idea. | Research pipeline rather than polished avatar app; setup/runtime maturity unknown for general users.                      | S09           |
| MUC / U-HMR                          | Research                                                        | Arbitrary uncalibrated views | Learned uncalibrated multi-view reconstruction.                         | Architecture precedent for per-view/per-joint fusion.          | Not packaged consumer real-time products; may depend on offline compute/training assumptions.                             | S10, S11      |

## 12.3 Software-only and single-camera consumer options

| **Option**                        | **Price**              | **Input**                        | **Why users choose it**                              | **Main complaints/limits**                                                                                  | **Refs** |
|-----------------------------------|------------------------|----------------------------------|------------------------------------------------------|-------------------------------------------------------------------------------------------------------------|----------|
| Standable: Full Body Estimation   | ~\$19.99               | HMD/controllers only             | Extremely low cost; no cameras or worn leg trackers. | Lower body is estimated, not observed; prone/sit/leg edge cases and stiffness are inherent.                 | S29      |
| VRChat built-in IK / estimation   | Included               | HMD/controllers                  | Zero setup; universal.                               | Sparse anchors cannot determine exact hips/feet; avatar proportions influence result.                       | S12      |
| XR Animator                       | Free                   | Webcam                           | Open, immediate avatar animation.                    | MediaPipe-style depth/occlusion/jitter limits.                                                              | S19      |
| Webcam Motion Capture             | Low subscription       | Webcam                           | Turnkey body, hand and face feature set.             | Monocular ambiguity and proprietary dependence.                                                             | S20      |
| PoseCap                           | Free/open              | Webcam                           | Modern full-body mesh direction.                     | Early preview; compute and root/world translation limitations.                                              | S21      |
| Warudo / VSeeFace tracking stacks | Free/paid frontend mix | Webcam + optional phone/Leap/VMC | Good avatar presentation and integrations.           | Tracking quality depends on separate underlying body/face/hand inputs; not a complete high-end body solver. | S55, S56 |

## 12.4 Consumer wearable and tracked-point options

| **System**                       | **Public cost**                                       | **Technology**                           | **Strength**                                                  | **Recurring grievance**                                                                                                                            | **Refs**     |
|----------------------------------|-------------------------------------------------------|------------------------------------------|---------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------|--------------|
| SlimeVR                          | Official sets roughly \$219+                          | IMU network, virtual skeleton            | Affordable, open ecosystem, 360°, no base stations.           | Orientation drift/reset behavior, inferred position, Wi-Fi/strap/calibration/body-proportion issues; community reports emphasize drift and feet.   | S30–S32, S78 |
| HaritoraX 2                      | \$299 public list                                     | Wearable IMUs                            | Long battery, consumer packaging, no camera volume.           | Drift and placement/calibration; optical Add-on R exists specifically to correct drift.                                                            | S33–S34      |
| Sony mocopi                      | \$449.99 class; Pro kit higher                        | 6 or 12 wearable sensors                 | Portable, creator-oriented ecosystem, face/body integrations. | Sparse/inferred motion, calibration, feet/global position and price; Pro kit substantially raises cost.                                            | S35–S36      |
| PICO Motion Tracker              | Low-cost pair, regional pricing                       | Ankle trackers + PICO headset estimation | Very low cost and easy PICO integration.                      | Ecosystem lock-in, sparse tracked points, headset/model dependence, sitting/lying/dance edge cases.                                                | S42          |
| VIVE Tracker 3.0 + Base Stations | 3 trackers + 2 bases ≈\$900 before straps/accessories | Lighthouse 6DoF points                   | Excellent positional tracking, mature SteamVR support.        | High setup cost, worn trackers, charging/dongles, line of sight/reflections, sparse IK.                                                            | S37–S38      |
| Tundra Tracker + Base Stations   | ~\$125 per tracker plus bases                         | Lighthouse 6DoF points                   | Smaller/lighter tracker alternative.                          | Same base-station and sparse-point costs; availability/bundles vary.                                                                               | S41          |
| VIVE Ultimate Tracker 3+1        | \$689 public kit class                                | Inside-out 6DoF trackers                 | No base stations; real positions.                             | Battery/wearables and environment sensitivity; official troubleshooting cites lighting/glare/room-feature conditions; community jump/loss reports. | S39–S40, S79 |

## 12.5 Depth-camera options

| **Option**           | **Cost**                             | **Technology**                          | **Strength**                                                       | **Limit**                                                                                 | **Refs** |
|----------------------|--------------------------------------|-----------------------------------------|--------------------------------------------------------------------|-------------------------------------------------------------------------------------------|----------|
| Intel RealSense D455 | ~\$419 public list class             | Stereo RGB-D, 0.6–6 m ideal range class | Real metric depth, scale and floor; commodity developer ecosystem. | Single-view occlusion remains, body SDK required, range/FOV/IR and driver integration.    | S43      |
| Orbbec Femto Bolt    | ~\$459 public list class             | Time-of-flight RGB-D                    | Azure-Kinect-like direction and skeleton ecosystem.                | Still front-view optical; lighting/IR, range, driver/licensing and extra hardware.        | S44      |
| Nuitrack             | ~\$119+ license class depending tier | Cross-camera skeleton SDK               | Reduces engineering required for depth/body tracking.              | Additional license cost; quality tied to supported depth hardware and frontal visibility. | S45      |
| Azure Kinect DK      | Discontinued by Microsoft            | RGB-D + body tracking ecosystem         | Historically strong developer reference.                           | End-of-life/availability and support uncertainty; not a good new consumer dependency.     | S46      |

## 12.6 Professional inertial and optical options

| **System**              | **Cost class**                          | **Technology**                       | **Strength**                                                              | **Recurring grievance**                                                           | **Refs**                 |
|-------------------------|-----------------------------------------|--------------------------------------|---------------------------------------------------------------------------|-----------------------------------------------------------------------------------|--------------------------|
| Rokoko Smartsuit Pro II | ~\$2,295 suit class; bundles higher     | Full inertial suit                   | Portable, many segments, creator/DCC ecosystem.                           | Cost, suit wear/fit, calibration, magnetic/position drift and foot cleanup.       | S47                      |
| Perception Neuron       | ~\$1,500–\$4,000+ by kit                | Inertial suit                        | Detailed body capture without camera line of sight.                       | Calibration, magnetic environment, global position/foot slide, suit setup.        | S49                      |
| Xsens MVN               | Several thousand dollars plus software  | Professional inertial                | Industry-grade workflow and robust orientation.                           | High cost, licensing, drift/global position and suit operation.                   | S48                      |
| OptiTrack               | Five figures for meaningful body volume | Many IR cameras + reflective markers | High raw precision, robust professional pipeline, ground-truth reference. | Cost, room, calibration, markers, line of sight, technical operation and cleanup. | S50–S51                  |
| Vicon / Qualisys class  | Quote-based, typically five figures+    | Professional optical                 | High-end capture volume and support.                                      | Same infrastructure and cost class; far beyond consumer VTuber budgets.           | S50 (category reference) |

## 12.7 Cloud and offline video mocap

| **Option**            | **Pricing model**                          | **Input**                         | **Strength**                                             | **Why it does not fill this gap**                                                                          | **Refs**                                    |
|-----------------------|--------------------------------------------|-----------------------------------|----------------------------------------------------------|------------------------------------------------------------------------------------------------------------|---------------------------------------------|
| Rokoko Vision         | Free/paid usage tiers; current limits vary | Uploaded single/dual-camera video | Easy offline animation extraction and Rokoko ecosystem.  | Not the same as continuous local VRChat tracking; upload, processing time, credit/plan limits and cleanup. | S53                                         |
| DeepMotion Animate 3D | Credit/subscription plans                  | Uploaded video                    | Accessible video-to-animation service with export tools. | Cloud dependence, per-minute economics, latency and foot/contact cleanup.                                  | S54                                         |
| Move AI               | Subscription/enterprise offerings          | Phone/camera capture              | High-quality markerless production focus.                | Price, capture workflow and offline orientation; not a free local avatar tracker.                          | S52                                         |
| RADiCAL / Plask class | Cloud subscription/credits                 | Uploaded video                    | Low hardware barrier.                                    | Cloud privacy, credits, processing and not continuous VR output.                                           | Category; vendor pricing changes frequently |

## 12.8 Face and hand market adjacent to body FBT

| **Option**                       | **Cost/input**                | **Technology**                 | **Strength**                                                 | **Main gap**                                                                 | **Refs** |
|----------------------------------|-------------------------------|--------------------------------|--------------------------------------------------------------|------------------------------------------------------------------------------|----------|
| iPhone / ARKit face              | Phone hardware                | TrueDepth face blendshapes     | Strong consumer face reference, smooth and widely supported. | Separate device, mounting/charging/network and Apple hardware cost.          | S57      |
| MediaPipe/OpenSeeFace            | Free webcam                   | RGB face landmarks/blendshapes | No extra hardware, local and widely integrated.              | Lighting/glasses/occlusion and usually less nuanced than TrueDepth.          | S17, S56 |
| Project Babble / VRCFaceTracking | Free/open                     | Webcam + bridge                | Community path to lower-face and VRChat expression inputs.   | Setup and avatar parameter compatibility; limited by camera pixels.          | S63, S81 |
| VIVE Full Face Tracker           | Accessory cost                | Dedicated VR face/eye hardware | Purpose-built VR expression capture.                         | Extra cost/hardware, device compatibility and availability.                  | S58      |
| Ultraleap LMC2                   | Dedicated sensor cost         | IR hand tracking               | High-rate hand-specific tracking and mature SDK.             | Limited capture volume/FOV, self-occlusion, extra hardware and availability. | S59      |
| HaMeR / WiLoR / Fast-HaMeR       | Research/open implementations | RGB hand crops                 | Strong modern hand-mesh foundation and distillation teacher. | Compute, license/research maturity, and fundamental hidden-finger ambiguity. | S60–S62  |

## 12.9 What the market sweep says people actually need

**A stable avatar matters more than a theoretically dense skeleton.** Current cheap camera products prove accessibility but not dependable feet/root; wearable products prove stability is worth paying for.

**Users will tolerate small errors, not maintenance.** Drift resets, strap placement, target-board calibration and multi-app bridges create recurring friction.

**Two cameras are an under-served consumer sweet spot.** They add real geometric evidence without the price or room complexity of a professional volume, yet existing products remain calibration-heavy or general-purpose.

**The product must include retargeting and failure behavior.** Raw pose quality alone does not solve stylized avatar proportions, VRChat IK, contacts or recovery.

**Face and hands are part of “full body” in practice.** Creators judge life and expressiveness through gaze, mouth and hands; a body-only solver still leaves a fragmented stack.

**Local and free is a differentiator.** Cloud video tools and paid camera apps are useful, but they do not deliver unlimited, private, continuous VRChat tracking.

## 12.10 Gap statement

> **Unoccupied product position**
>
> Free/open, local, one-to-N commodity-camera avatar tracking with transferable site calibration; instant or near-instant unseen-performer initialization; modern body reconstruction; geometric + learned fusion; dedicated face/hands; 60–90 Hz predictive output; Avatar Profile import; direct VRM/VMC and VRChat virtual trackers; explicit confidence and graceful failure. No reviewed product currently occupies this complete position.

# 13. Sources and research method

Official product pages and primary research carry the greatest weight; community reports identify failure patterns, not prevalence.

## 13.1 Source-quality hierarchy

| **Tier** | **Sources**                                                                                                        | **Use**                                                                                     |
|----------|--------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------------------------|
| Tier 1   | Official product pages, technical manuals, support documents, protocol specifications and primary research papers. | Used for current features, prices, architecture and documented limitations.                 |
| Tier 2   | Steam product/review pages, GitHub repositories/issues and reproducible community documentation.                   | Used to assess product maturity, setup burden and recurring operational problems.           |
| Tier 3   | Reddit/forums and individual reports.                                                                              | Used only as anecdotal evidence of complaint patterns; never treated as a statistical rate. |

## 13.2 Research notes

- Market pricing is volatile and regional. The document uses public list prices or defensible ranges and records the access date rather than implying permanent pricing.

- Professional systems are compared on their actual strengths. The design does not claim that an RGB camera system beats them in raw measurement.

- Community complaint themes are triangulated against official architecture/support documents where possible—for example IMU drift documentation, VIVE environment guidance and camera calibration manuals.

- Technical recommendations rely on primary research and official documentation rather than secondary summaries.

- URLs below were selected for direct verification and future rechecking. Accessed August 2026 unless the vendor page is dynamically updated.

## 13.3 Bibliography and market evidence

**[S01] Meta AI.** [SAM 3D Body — official code and checkpoints](https://github.com/facebookresearch/sam-3d-body).

**[S02] Research paper.** [SAM 3D Body: Robust 3D Human Body Reconstruction from Images](https://arxiv.org/abs/2602.15989).

**[S03] Research paper.** [Fast SAM 3D Body](https://arxiv.org/abs/2603.15603).

**[S04] Research paper.** [SAM-Body4D: Temporally Consistent Human Reconstruction](https://arxiv.org/abs/2512.08406).

**[S05] Research paper.** [OnlineHMR: Causal Online Human Mesh Recovery](https://arxiv.org/abs/2603.17355).

**[S06] Research paper.** [WHAM: Reconstructing World-grounded Humans with Accurate 3D Motion](https://arxiv.org/abs/2312.07531).

**[S07] Research project.** [GVHMR: World-Grounded Human Motion Recovery](https://zju3dv.github.io/gvhmr/).

**[S08] Research project.** [4DHumans / HMR 2.0](https://shubham-goel.github.io/4dhumans/).

**[S09] Research paper/project.** [Kineo: calibration-free, unsynchronized consumer-camera human capture](https://arxiv.org/html/2510.24464v1).

**[S10] Research paper.** [MUC: uncalibrated multi-view human reconstruction](https://arxiv.org/abs/2403.05055).

**[S11] Research paper.** [U-HMR: uncalibrated multi-view human mesh recovery](https://arxiv.org/abs/2403.12434).

**[S12] VRChat.** [Full-Body Tracking documentation](https://docs.vrchat.com/docs/full-body-tracking).

**[S13] VRChat.** [OSC Trackers documentation](https://docs.vrchat.com/docs/osc-trackers).

**[S14] VMC.** [Virtual Motion Capture Protocol](https://protocol.vmc.info/english.html).

**[S15] VRM Consortium.** [VRM 1.0 Humanoid specification](https://vrm.dev/en/vrm1/humanoid/).

**[S16] Google AI Edge.** [MediaPipe Pose Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/pose_landmarker).

**[S17] Google AI Edge.** [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker).

**[S18] Google AI Edge.** [MediaPipe Hand Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/hand_landmarker).

**[S19] Open source.** [XR Animator / System Animator Online](https://github.com/ButzYung/SystemAnimatorOnline).

**[S20] Consumer software.** [Webcam Motion Capture](https://webcammotioncapture.info/).

**[S21] Open source.** [PoseCap — real-time single-webcam SMPL-X capture](https://github.com/CorridorTech/PoseCap).

**[S22] Steam.** [MocapForAll product page and reviews](https://store.steampowered.com/app/1759710/MocapForAll/).

**[S23] MocapForAll.** [User manual and calibration workflow](https://akiya-research-institute.github.io/MocapForAll-Manual/en/).

**[S24] Open source.** [FreeMoCap](https://freemocap.org/).

**[S25] FreeMoCap.** [Camera calibration architecture](https://docs.freemocap.org/freemocap/docs/architecture/backend-calibration/).

**[S26] Consumer software.** [Remocapp multi-camera motion capture](https://www.remocapp.com/).

**[S27] Consumer software.** [Remocapp pricing](https://remocapp.com/pricing).

**[S28] Consumer software.** [MocapForStreamer](https://xyeffectlab.booth.pm/items/4547715).

**[S29] Steam.** [Standable: Full Body Estimation](https://store.steampowered.com/app/2370570/Standable_Full_Body_Estimation/).

**[S30] SlimeVR.** [Official store](https://shop.slimevr.dev/).

**[S31] SlimeVR.** [How SlimeVR estimates full-body position](https://docs.slimevr.dev/slimevr101.html).

**[S32] SlimeVR.** [IMU comparison and yaw-drift guidance](https://docs.slimevr.dev/diy/imu-comparison.html).

**[S33] Shiftall.** [HaritoraX 2](https://en.shiftall.net/products/haritorax2).

**[S34] Shiftall.** [HaritoraX Add-on R optical drift correction](https://en.shiftall.net/products/haritorax-addon-r).

**[S35] Sony.** [mocopi](https://electronics.sony.com/more/mocopi/all-mocopi/p/qmss1-uscx).

**[S36] Sony.** [mocopi Pro Kit](https://xyn.sony.net/en/products/mocopi-pro/).

**[S37] HTC VIVE.** [VIVE Tracker 3.0](https://www.vive.com/us/accessory/tracker3/).

**[S38] HTC VIVE.** [SteamVR Base Station 2.0](https://www.vive.com/us/accessory/base-station2/).

**[S39] HTC VIVE.** [VIVE Ultimate Tracker 3+1 kit](https://shop-us.vive.com/products/vive-ultimate-tracker-3-1-kit).

**[S40] HTC VIVE Support.** [Ultimate Tracker stability and environment guidance](https://www.vive.com/us/support/ultimate-tracker/category_howto/tracking-is-not-stable-or-accurate.html).

**[S41] Tundra Labs.** [Tundra Tracker](https://tundra-labs.com/products/tundra-tracker-1).

**[S42] PICO.** [PICO Motion Tracker](https://www.picoxr.com/global/products/pico-motion-tracker).

**[S43] RealSense.** [D455 depth camera](https://www.realsenseai.com/products/depth-camera-d455/).

**[S44] Orbbec.** [Femto Bolt RGB-D camera](https://www.orbbec.com/products/tof-camera/femto-bolt/).

**[S45] Nuitrack.** [Skeleton tracking licensing and pricing](https://nuitrack.com/#pricing).

**[S46] Microsoft.** [Azure Kinect DK end-of-life announcement](https://techcommunity.microsoft.com/blog/azure-ai-services-blog/microsoft-azure-kinect-developer-kit-technology-transfers-to-partner-ecosystem/3899122).

**[S47] Rokoko.** [Smartsuit Pro II](https://www.rokoko.com/products/smartsuit-pro).

**[S48] Movella / Xsens.** [MVN motion capture](https://www.movella.com/products/motion-capture).

**[S49] Noitom.** [Perception Neuron motion capture](https://www.noitom.com/perception-neuron-series).

**[S50] OptiTrack.** [Professional optical motion capture systems](https://www.optitrack.com/).

**[S51] OptiTrack.** [Motive software](https://www.optitrack.com/software/motive/).

**[S52] Move AI.** [Markerless capture products and pricing](https://www.move.ai/pricing).

**[S53] Rokoko.** [Rokoko Vision video mocap](https://www.rokoko.com/products/vision).

**[S54] DeepMotion.** [Animate 3D pricing](https://www.deepmotion.com/pricing).

**[S55] Warudo.** [3D VTuber software documentation](https://docs.warudo.app/).

**[S56] VSeeFace.** [Webcam face tracking and VMC integration](https://www.vseeface.icu/).

**[S57] Apple Developer.** [ARKit face blend-shape locations](https://developer.apple.com/documentation/arkit/arfaceanchor/blendshapelocation).

**[S58] HTC VIVE.** [VIVE Full Face Tracker](https://www.vive.com/us/accessory/vive-full-face-tracker/).

**[S59] Ultraleap.** [Leap Motion Controller 2](https://www.ultraleap.com/product/leap-motion-controller-2/).

**[S60] Research project.** [HaMeR hand mesh recovery](https://geopavlakos.github.io/hamer/).

**[S61] Research project.** [WiLoR hand mesh recovery](https://rolpotamias.github.io/WiLoR/).

**[S62] Research project.** [Fast-HaMeR](https://fast-hamer.github.io/).

**[S63] Open source.** [Project Babble webcam face tracking](https://github.com/Project-Babble/ProjectBabble).

**[S64] Dataset.** [AMASS motion archive](https://amass.is.tue.mpg.de/).

**[S65] Dataset.** [AIST++ dance motion dataset](https://google.github.io/aistplusplus_dataset/).

**[S66] Dataset.** [Motion-X whole-body motion dataset](https://motion-x-dataset.github.io/).

**[S67] Dataset.** [BEDLAM synthetic human dataset](https://bedlam.is.tue.mpg.de/).

**[S68] Dataset.** [SynBody synthetic human dataset](https://synbody.github.io/).

**[S69] Research paper/dataset.** [Syn4D synthetic multi-view 4D data](https://arxiv.org/abs/2605.05207).

**[S70] Dataset.** [ARCTIC hands/object interaction](https://arctic.is.tue.mpg.de/).

**[S71] Research paper/dataset.** [OakInk2 bimanual hand-object interaction](https://openaccess.thecvf.com/content/CVPR2024/html/Zhan_OakInk2_A_Dataset_of_Bimanual_Hands-Object_Manipulation_in_Complex_Task_Completion_CVPR_2024_paper.html).

**[S72] Dataset.** [GRAB full-body grasping](https://grab.is.tue.mpg.de/).

**[S73] Dataset.** [BEHAVE human-object interaction](https://virtualhumans.mpi-inf.mpg.de/behave/).

**[S74] Dataset.** [InterCap human-object interaction](https://intercap.is.tue.mpg.de/).

**[S75] U.S. Copyright Office.** [Copyright and Artificial Intelligence, Part 3: Generative AI Training](https://www.copyright.gov/ai/Copyright-and-Artificial-Intelligence-Part-3-Generative-AI-Training-Report-Pre-Publication-Version.pdf).

**[S76] YouTube Help.** [Creative Commons licensing on YouTube](https://support.google.com/youtube/answer/2797468).

**[S77] Steam Community.** [MocapForAll community discussions and reviews](https://steamcommunity.com/app/1759710/).

**[S78] Reddit community sweep.** [SlimeVR drift discussions (anecdotal)](https://www.reddit.com/r/SlimeVR/search/?q=drift&restrict_sr=1&sort=relevance&t=all).

**[S79] Reddit community sweep.** [VIVE Ultimate tracking discussions (anecdotal)](https://www.reddit.com/r/virtualreality/search/?q=vive%20ultimate%20tracker%20tracking&restrict_sr=1&sort=relevance&t=all).

**[S80] Reddit community sweep.** [VRChat full-body calibration and proportion discussions (anecdotal)](https://www.reddit.com/r/VRchat/search/?q=full%20body%20tracking%20calibration&restrict_sr=1&sort=relevance&t=all).

**[S81] Open source.** [VRCFaceTracking documentation](https://docs.vrcft.io/).

**[S82] SteamVR.** [OpenVR driver documentation](https://github.com/ValveSoftware/openvr/wiki/Driver-Documentation).

**[S83] Research paper.** [SparsePoser: sparse tracker full-body pose reconstruction](https://arxiv.org/abs/2311.02191).

**DESIGN VERDICT**

> Build the one-camera pipeline to establish a useful floor. Treat two cameras as the recommended product, not an optional afterthought. Preserve geometry, uncertainty and image features through fusion. Spend disproportionate engineering effort on contacts, temporal recovery and avatar-specific retargeting. Use synthetic data to supervise the multiview problem, public models as teachers, and a compact student at runtime.
>
> **That combination has a credible chance to make current cheap webcam tracking look obsolete and to challenge \$200–\$700 consumer FBT on perceived VRChat/VTuber quality—while remaining honest that fully hidden motion and laboratory measurement are different problems.**
