# Senrigan — Prior Art

**Sweep date:** 16 August 2026. Recheck sources at implementation time.
**Product scope:** [`overall_ideal.md`](overall_ideal.md).
**Names:** follow the glossary in that file.

Senrigan did not invent grounded monocular capture, contact reasoning, or geometry-aware retargeting. It integrates those ideas into a live one-camera VTuber tracker.

This file records research precedents, products in the intended replacement class, and products that stay out of scope. It does not promise that replacement has been measured.

---

## 1. Source method

Official product pages, manuals, protocol specifications, and primary papers carry the greatest weight. Store pages and project repositories support maturity and setup burden. Forum reports mark complaint patterns only. They are not rates.

Prices change by region and date. This file avoids list prices unless a date is attached. Recheck any commercial claim before a public comparison.

---

## 2. Research Senrigan builds on

These works are starting points. They are not shipped Senrigan modules.

### 2.1 World-grounded and causal motion

| Work | Role |
|---|---|
| [PhysCap](https://arxiv.org/abs/2008.08880) | Physical constraints on monocular capture |
| [WHAM](https://arxiv.org/abs/2312.07531) | World-grounded human motion from video |
| [GVHMR](https://zju3dv.github.io/gvhmr/) | World-grounded human mesh recovery |
| [OnlineHMR](https://arxiv.org/abs/2603.17355) | Causal online human mesh recovery |

Senrigan wants the same session job in a live VTuber loop. Floor, contact, and causal state belong in the estimator. They do not belong only in a per-frame spatial model.

### 2.2 Spatial recovery

| Work | Role |
|---|---|
| [SAM 3D Body](https://arxiv.org/abs/2602.15989) | Lead spatial-model family. Body, feet, and hands. No facial-expression output. [Code](https://github.com/facebookresearch/sam-3d-body) |
| [Fast SAM 3D Body](https://arxiv.org/abs/2603.15603) | Training-free speed-up of SAM 3D Body. First-class Phase 0 candidate. [Code](https://github.com/yangtiming/Fast-SAM-3D-Body) |
| [Multi-HMR](https://arxiv.org/abs/2402.14654) | Research spatial baseline, including the Anny checkpoint |
| [NLF](https://arxiv.org/abs/2407.07532) | Research spatial baseline |

License and bundle status live in the model ledger in [`overall_ideal.md`](overall_ideal.md). Fast SAM 3D Body code is reported as MIT. It still runs on SAM 3D Body weights. Those weights keep their own terms.

### 2.3 Human-based calibration

| Work | Role |
|---|---|
| [Kineo](https://arxiv.org/html/2510.24464v1) | Board-free geometry from consumer cameras and human motion |
| [MUC](https://arxiv.org/abs/2403.05055) | Uncalibrated multi-view reconstruction |
| [U-HMR](https://arxiv.org/abs/2403.12434) | Uncalibrated multi-view mesh recovery |

These support later multi-camera work. They are not a Phase 0 dependency.

### 2.4 Retargeting

| Work | Role |
|---|---|
| [ReConForM](https://arxiv.org/abs/2502.21207) | Contact-aware retargeting |
| [MeshRet](https://arxiv.org/abs/2410.20986) | Mesh-aware retargeting |
| [Ultrafast online retargeting](https://discovery.ucl.ac.uk/id/eprint/10219287/) | Online controllable retargeting |

The first useful release uses the rotation and IK baseline. A geometry-aware solver is Phase 3.

### 2.5 Face and hands

| Work | Role |
|---|---|
| [MediaPipe Face Landmarker](https://ai.google.dev/edge/mediapipe/solutions/vision/face_landmarker) | Phase 5 face baseline |
| [HaMeR](https://geopavlakos.github.io/hamer/) | Hand-mesh teacher |
| [WiLoR](https://rolpotamias.github.io/WiLoR/) | Hand-mesh teacher |
| [Project Babble](https://github.com/Project-Babble/ProjectBabble) | Community lower-face path |

Face and native-resolution hand crops are not first-useful-release work.

### 2.6 Runtime formats

| Work | Role |
|---|---|
| [ONNX Runtime execution providers](https://onnxruntime.ai/docs/execution-providers/) | Provider paths |
| [VRM](https://vrm.dev/en/) | First avatar format |
| [VMC Protocol](https://protocol.vmc.info/english) | First live output protocol |
| [MCAP](https://mcap.dev/) | Record and replay container |

---

## 3. Intended replacement class

Compare Senrigan to these products after the first useful release. Do not claim replacement before a public comparison exists. Public comparison rules live in [`file_architecture.md`](file_architecture.md).

| Product | What it is | What Senrigan adds after measurement |
|---|---|---|
| [XR Animator](https://github.com/ButzYung/SystemAnimatorOnline) | Free one-camera MediaPipe or TensorFlow.js tracker with VMC | A modern spatial model, locked proportions, site state, and status-labeled loss |
| [Webcam Motion Capture](https://webcammotioncapture.info/) | Paid one-camera VTuber tracker | The same job, local and open, with explicit failure status |
| [PoseCap](https://github.com/CorridorTech/PoseCap) | Early open single-camera mesh capture | A VTuber output path and a session estimator |
| Raw selected spatial model | Project ablation | Proof of what the estimator, contacts, and retargeter add |
| MediaPipe-class pose plus smoothing | Minimum reproducible baseline | The quality floor later changes must beat |

Recurring failures in this class:

| Failure | Senrigan response |
|---|---|
| Idle jitter | Analytical estimator and stillness gates |
| Foot skating | Contacts and planted-foot gates |
| Root pumping | Locked scale and root-support rules |
| Mirrored limbs | Innovation gates and flip counts |
| Occlusion explosions | Visibility versus origin, then hold |
| Smoothing lag | Causal prediction on the output clock |
| Crop scale error | Unclamped region of interest and denied root authority |
| Hidden failure | Typed status on every output tick |

---

## 4. Out of scope for the first useful release

Senrigan does not replace these products in the first useful release. A later phase may compare with a subset. That comparison is new work.

| Product | Why it stays out |
|---|---|
| [MocapForAll](https://store.steampowered.com/app/1759710/MocapForAll/), [Remocapp](https://www.remocapp.com/), [MocapForStreamer](https://xyeffectlab.booth.pm/items/4547715) | Two or more cameras. Paid or calibration-heavy workflows. Phase 8 may meet this class |
| [FreeMoCap](https://freemocap.org/) | Open multi-camera science tool. Board calibration. Different latency target |
| [SlimeVR](https://shop.slimevr.dev/), [HaritoraX](https://en.shiftall.net/products/haritorax2), [Sony mocopi](https://electronics.sony.com/more/mocopi/all-mocopi/p/qmss1-uscx) | Wearable inertial kits. They work when no camera sees the body |
| [VIVE Ultimate Tracker](https://shop-us.vive.com/products/vive-ultimate-tracker-3-1-kit), Tundra, [PICO Motion Tracker](https://www.picoxr.com/global/products/pico-motion-tracker) | Tracked points and 360-degree motion. Different input class |
| [Standable](https://store.steampowered.com/app/2370570/Standable_Full_Body_Estimation/) and VRChat built-in IK | No camera. Lower body is estimated from a headset |
| [Rokoko Vision](https://www.rokoko.com/products/vision), [DeepMotion](https://www.deepmotion.com/pricing), [Move AI](https://www.move.ai/pricing) | Cloud or offline video. Different privacy and latency model |
| [OptiTrack](https://www.optitrack.com/), Vicon, [Xsens](https://www.movella.com/products/motion-capture), [Rokoko suits](https://www.rokoko.com/products/smartsuit-pro) | Measurement instruments. Senrigan does not claim that class |

---

## 5. Interoperate. Do not replace

The first useful release sends a final retargeted pose to existing VTuber software. It does not replace the renderer.

Named Virtual Motion Capture (VMC) reference receivers:

1. [VSeeFace](https://www.vseeface.icu/)
2. [Warudo](https://docs.warudo.app/)

[VNyan](https://suvidriel.itch.io/vnyan) is an optional third receiver when a test machine has it.

Other adjacent tools include Project Babble and VRCFaceTracking. They are later face paths. They are not body-tracker replacements.

---

## 6. Gap statement

Fact: no reviewed product on this sweep date is a free local one-camera VTuber tracker that combines all of the following:

- a modern whole-body spatial model
- locked performer proportions
- board-free site state
- status-labeled prediction and hold
- Virtual Reality Model (VRM) import
- VMC output
- failure that stays visible

That is an integration gap. It is not a quality claim. It is not a claim that one camera beats a wearable kit or a two-camera geometric solver.

Recommendation: treat XR Animator and a MediaPipe-class baseline as the first public comparisons. Add Webcam Motion Capture when a licensed copy is available. Include failure clips. Name hardware, camera, avatar, and version in every comparison.
