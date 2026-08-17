# Senrigan — Implementation Landings

**Product:** [`overall_ideal.md`](overall_ideal.md).
**Engine:** [`engine_concept.md`](engine_concept.md).
**Phases, numbers, gates:** [`file_architecture.md`](file_architecture.md).

This file turns those documents into mergeable landings. Each landing stays in the first useful release. Nothing here is a throwaway demo.

The discarded YOLO-pose VMC slice is not a landing. Do not rebuild it. A 2D detector plus a stick figure is not Senrigan.

---

## 1. Target

Ship the first useful release: Phase 0 plus Phase 1 plus Phase 2.

That means a user can import a VRM, complete board-free setup, see finite status-labeled motion, and send the same pose through VMC. Site calibration, locked proportions, the analytical estimator, and the loss lifecycle are on. Extreme-avatar solving, learned temporal models, face, hands, multi-GPU, and a second camera are not.

Development host is this Linux laptop with an RTX 4060. Windows Media Foundation is still a first-useful-release capture path. Linux V4L2 is the path that unblocks work now.

---

## 2. Rules for every landing

1. Land only code that the first useful release still owns.
2. Keep the crate split: `types`, `engine`, `perception`, `avatar`, `app`.
3. One owner per state. The estimator owns root, pose, contact, and uncertainty.
4. The 60 Hz pose stream stays in Rust. The UI reads a snapshot and never blocks the output clock.
5. A fake spatial source is allowed. A second product made of YOLO or MediaPipe is not.
6. Record and replay exist before a quality claim.
7. Provider failure is visible. Do not add a silent CPU fallback.
8. Each landing has tests that fail if the landing is reverted.
9. Do not add a daemon, plugin ABI, HTTP control plane, or inference trait.

---

## 3. Two tracks

Work both tracks. Do not wait for SAM 3D Body export before landing the spine.

| Track | Owns | Unblocks |
|---|---|---|
| A. Spine | Contracts, capture, MCAP, fake source, estimator types, VRM, VMC, setup UI | A runnable app that is already Senrigan |
| B. Spatial | Candidate export, ORT CUDA on the 4060, adapter, provider report | Replacing the fake source without rewriting downstream |

Track B is Phase 0. Track A is Phase 1 using a deterministic fake source until Track B lands. Phase 2 follows both.

---

## 4. Landings

Merge in this order unless a note says two landings may proceed in parallel.

### A1 — Workspace and contracts

**Lands:** Cargo workspace, `justfile`, `.gitignore`, crate stubs, `types`.

**Must include:** frames `C`, `W`, `S`, `A`. Internal quaternions `xyzw`. OpenCV camera frame. Canonical joint table from `engine_concept.md`. `VisibilityEvidence` and `EstimateOrigin` as separate enums. Lifecycle enum. Manifest struct. Schema version. Reject NaN and invalid quaternions.

**Exit:** unit tests for transforms, time order, schema round trip, version reject, invalid numeric reject.

**Not in this landing:** capture, ONNX, UI.

### A2 — MCAP record and replay with a fake clock

**Lands:** MCAP writer and reader in `app`. Channels for `FramePacket` metadata, later observations, canonical frames, status.

**Must include:** a fake monotonic clock. A ten-second synthetic session that replays frame order and timestamps. Visible recording-failure path.

**Exit:** `cargo test` plus a checked-in tiny synthetic fixture under `testdata/`.

**Not in this landing:** a webcam. Do not record raw video in git.

### A3 — Linux capture into `FramePacket`

**Lands:** V4L2 capture in `app`. Bounded newest-frame queue. MJPG or YUYV to RGB. Timestamp quality (`Source` or `Receipt`).

**Must include:** write capture into MCAP. Replay without inference. Count dropped frames.

**Exit:** a local ten-minute capture replays order and mode. Queue overflow is counted.

**Parallel:** B1 may start.

### A4 — Fake spatial source and output clock

**Lands:** a deterministic `SpatialObservation` generator in `perception` that emits the canonical skeleton. Output loop at the owned output clock. Basic continuity and hold when the fake source pauses.

**Must include:** the same adapter boundary a real model will use. Status on every tick. Last-valid fallback.

**Exit:** replay of a dropped-observation fixture stays finite at the output clock.

**Not in this landing:** YOLO, MediaPipe, or any 2D-only product path.

### A5 — VRM import and rotation and IK baseline

**Lands:** `avatar` import for VRM 0 and VRM 1. Validate required bones, axes, rest. Normalized rotation transfer, root map, terminal IK.

**Must include:** at least the Seed-san sample or three normal-proportion fixtures used only locally. Import probes. Clear error on missing required bones.

**Exit:** three normal-proportion VRMs animate on recorded canonical motion without axis reversals or exploding scale.

**Not in this landing:** geometry-aware sparse solver.

### A6 — Preview and VMC from `RetargetResult` only

**Lands:** in-process preview of the final retargeted pose. VMC sender. Named receivers are VSeeFace and Warudo.

**Must include:** Rust owns the output-clock pose. Preview is a coalesced snapshot or a Rust renderer. VMC consumes only `RetargetResult`.

**Exit:** preview and one VMC receiver agree on root and required bones for a replay.

### A7 — Setup surface

**Lands:** the smallest UI that can select a camera, a model package, a VRM, start a session, consent to record, and show lifecycle. Use `iced`. Headless binary remains.

**Must include:** in-process Rust calls into the engine. The engine owns session state. The view holds a read-only snapshot.

**Exit:** a developer can run setup without Python for everything except model export.

**Windows packaging is later.** Linux is enough for this landing.

### B1 — Candidate inventory

**Lands:** `models/` ledger entries for SAM 3D Body, Fast SAM 3D Body, Multi-HMR, NLF. Exact source URL, hash when downloaded, code license, checkpoint license, `LOCAL_RESEARCH_ONLY` or `UNREVIEWED`.

**Exit:** no candidate is labeled bundle-clean without terms.

**Parallel with A1–A4.**

### B2 — Minimal export on the 4060

**Lands:** Python export scripts under `ml/`. One recorded still image. One ONNX graph that runs through `ort` with CUDA on this laptop.

**Exit:** finite tensors, named I/O, documented shapes. A machine-readable note if export fails. Failure does not block Track A.

**Not in this landing:** training, distillation, or a 2D fallback product.

### B3 — Adapter and swap

**Lands:** body-model adapter to the canonical skeleton. Parity check against the source framework within declared tolerances.

**Exit:** A4 replay with the real adapter in place of the fake source keeps the same schema. No MHR or SMPL type crosses the boundary.

### B4 — Feasibility decision

**Lands:** a written Phase 0 decision in `models/` or `benchmarks/`. One selected package for the 4060. Arc B580 marked pending until measured.

**Exit:** Phase 1 live inference uses that package. If no candidate exports, keep the fake source and write the failed gate. Do not invent a YOLO product.

### C1–C5 — Grounding (Phase 2)

Land only after A6 and after a spatial source exists (fake or real).

1. Instrumented calibration harness.
2. Board-free site solve and performer lock.
3. Coverage contract (unclamped ROI, visibility versus origin).
4. Analytical estimator as specified: constant-velocity predict, diagonal covariance, one bounded correction, hard projection.
5. Loss lifecycle and isolated reacquisition.

**Exit:** Phase 2 gates in `file_architecture.md`. This is the public first useful release, together with a Windows capture landing.

### W1 — Windows capture and package

**Lands:** Media Foundation capture. Installer or a zip that does not require Python. Both happen before the first useful release is called done.

**May follow A7.** Must exist before release.

---

## 5. What each merge looks like

A landing is done when:

- It is on its own branch and reviewable.
- Tests for that landing pass on Linux.
- It does not introduce a second architecture.
- README run steps cover only landings that exist.

Do not open a landing that implements Phase 3–8 while C5 is open.

---

## 6. First week

Start A1 and B1 together. Then A2. Then A3 on this laptop.

Do not start preview polish, face, or a public comparison until A6 exists.
