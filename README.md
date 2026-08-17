# Senrigan

Senrigan is an experimental, local **3D VTuber tracker from one ordinary webcam**.

The goal is simple to describe and hard to execute: import a VRM avatar, complete a short board-free setup, and get live full-body motion that is stable, grounded, and useful in existing VTuber applications. No trackers, base stations, second camera, cloud service, or subscription are required.

This is a hobby/open-source research-engineering project, not a startup plan. It is currently in the **design phase. There is no code yet**.

## The idea

Single-image pose models can produce plausible bodies, but ordinary webcam tracking often looks bad over time: idle jitter, foot skating, root pumping, mirrored limbs, occlusion explosions, drift, and smoothing lag. Senrigan focuses on the session-level information a generic image model does not use:

- A fixed camera supplies reusable room calibration.
- A short fit locks performer proportions.
- The estimator tracks causal state and observation age.
- Floor and contact constraints ground the body.
- Bounded prediction handles joints or performers that leave the frame.
- Geometry-aware retargeting adapts motion to stylized avatars.

Depth along the camera ray is still ambiguous. Reaches toward the lens, hidden limbs, fast turns, floor work, severe crop loss, and poor lighting will remain difficult. The project aims to fail smoothly and visibly rather than claim those cases are solved.

## Initial target

The first useful release is a single-GPU Windows application with:

- It captures a webcam and supports deterministic record/replay.
- It loads one clearly labelled whole-body model configuration.
- It previews a VRM with basic retargeting.
- It produces finite avatar output at 60 Hz.
- It sends VMC output to existing VTuber software.

The equal reference GPU classes are an **Intel Arc B580 desktop** and an **NVIDIA RTX 4060 desktop**. Senrigan is assumed to be the primary GPU workload. Other hardware and concurrent gaming are best-effort until measured. Multi-GPU scheduling is a later optional mode.

## Documents

| File | Owns |
|---|---|
| [`overall_ideal.md`](misc-planning/overall_ideal.md) | Authoritative direction, product behavior, workflows, success criteria, limitations, privacy, and release posture |
| [`engine_concept.md`](misc-planning/engine_concept.md) | Engine data flow, state, contracts, estimation, grounding, and retargeting |
| [`file_architecture.md`](misc-planning/file_architecture.md) | Stack, repository layout, implementation order, hardware gates, and testing |

## Stack intent

Rust runtime, ONNX Runtime through `ort`, Python for model work and evaluation, Tauri for the setup application, MCAP for record/replay, VRM as the first avatar format, and VMC as the first live output protocol.

Code is intended for GPLv3. Models remain separately licensed artifacts: every bundled or optional configuration must identify its checkpoint, source, terms, and restrictions. The project may use research-only models for local experiments without pretending that makes them redistributable.
