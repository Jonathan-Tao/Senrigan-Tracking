# Senrigan

Senrigan is a local one-camera VTuber tracker. It is in design. There is no code yet.

A user imports a Virtual Reality Model (VRM), completes a short board-free setup, and sends live full-body motion through Virtual Motion Capture (VMC). No trackers, base stations, second camera, cloud service, or subscription are required. The project is a hobby and research-engineering effort. It is not a startup plan.

Single-image pose models can produce plausible bodies. Ordinary webcam tracking often looks bad over time: idle jitter, foot skating, root pumping, mirrored limbs, occlusion explosions, drift, and smoothing lag. Senrigan adds the session facts a generic image model does not use:

- A fixed camera supplies reusable site calibration.
- A short fit locks performer proportions.
- The estimator tracks causal state and observation age.
- Floor and contact constraints ground the body.
- Bounded prediction handles joints or performers that leave the frame.
- Geometry-aware retargeting adapts motion to stylized avatars. That layer is later than the first useful release.

Depth along the camera ray is still ambiguous. Reaches toward the lens, hidden limbs, fast turns, floor work, severe crop loss, and poor lighting will remain difficult. The project aims to fail smoothly and visibly rather than claim those cases are solved.

## First useful release

The first useful release is a Windows application that uses one graphics processing unit (GPU) and:

- Captures a webcam and supports deterministic record and replay.
- Loads one clearly labeled spatial model configuration.
- Imports a VRM and applies the rotation and inverse-kinematics (IK) baseline.
- Runs site calibration, locked performer proportions, the analytical estimator, and the loss lifecycle.
- Previews the final pose and sends the same pose through VMC.
- Publishes finite, status-labeled avatar output on the output clock.

The live baseline without grounding is an internal checkpoint. It is not a public release.

Reference GPU classes are an Intel Arc B580 desktop and an NVIDIA RTX 4060 desktop. One GPU is enough to develop. Both classes must pass before the first useful release. Senrigan is assumed to be the primary GPU workload. Other hardware and concurrent gaming are best-effort until measured. Multi-GPU scheduling is a later optional mode.

## Documents

| File | Owns |
|---|---|
| [`overall_ideal.md`](misc-planning/overall_ideal.md) | Glossary, product behavior, workflows, success criteria, limitations, privacy, and release posture |
| [`engine_concept.md`](misc-planning/engine_concept.md) | Engine data flow, state, contracts, estimation, grounding, and retargeting |
| [`file_architecture.md`](misc-planning/file_architecture.md) | Stack, repository layout, owned numbers, phases, hardware gates, and testing |
| [`prior_art.md`](misc-planning/prior_art.md) | Research precedents, replacement class, and out-of-scope products |
| [`AGENTS.md`](AGENTS.md) | Prose rules for maintained documents |
| [`LICENSE`](LICENSE) | GNU General Public License version 3 terms for Senrigan work |

## Stack intent

Rust runtime, Open Neural Network Exchange (ONNX) Runtime through `ort`, Python for model work and evaluation, Tauri for the setup application, the MCAP recording container for record and replay, VRM as the first avatar format, and VMC as the first live output protocol.

## License

Copyright (C) 2026 Senrigan contributors.

Unless a file states different terms, the original Senrigan source code and repository documentation are free software under the [GNU General Public License version 3 only](LICENSE), identified by the SPDX expression `GPL-3.0-only`.

This license does not relicense model weights, datasets, avatar files, or third-party code and assets. Those artifacts keep their own terms. Every bundled or optional model configuration must identify its checkpoint, source, terms, and restrictions. A research-only artifact can be used for a permitted local experiment. It cannot enter a public bundle unless its terms permit distribution and the intended use.
