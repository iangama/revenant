# M22 runtime review evidence

These files are direct outputs from the original Godot runtime. They contain no generated concept art, external footage, third-party audio, trademarks, or proprietary game assets. Runtime geometry, materials, UI, effects, synthesis, and capture orchestration are authored in this repository.

Run `scripts/capture-m22.sh` from the repository root to recreate the package at the canonical 1280x720 viewport with Godot 4.7.1 GL Compatibility. The script uses a disposable settings directory, captures four review states, records the same deterministic validation at 30 FPS, and verifies `captures/SHA256SUMS`.

| File | Purpose | SHA-256 |
| --- | --- | --- |
| `captures/01-entry.png` | Explicit local identity, endpoint, connection, settings, quit, and keyboard path | `646454826a56b71715599864c51862adedcf498b630cbd5ed73f62f8dfc8c64e` |
| `captures/02-settings.png` | Four volume controls, mute, display, guidance, reduced flash, and keyboard actions | `7ad000191b6c3fa55cfd1438a676c38c0b2ae1570f58dce4c8302fa381dc2c18` |
| `captures/03-onboarding.png` | Revisitable movement guidance beside complete waiting-state HUD evidence | `ea0020988c29524b2b82f8768547a122dbdc375bb550d31c01640e6f7f442402` |
| `captures/04-runtime.png` | Warden guidance, authoritative-state text, enemy health, and bounded combat feedback | `677c8f4e0aae34059c4ccd45642b4f358a4d60f6daf7fa456bb5e4e65a28bcec` |
| `captures/05-m22-review.avi` | 46-frame, 1.53-second Motion JPEG/PCM audiovisual validation record at 30 FPS | `bbdaa09973b2cea72b6861170d332f1af09c78eaf1dd328ff5bccf14570f2c59` |

The complete evidence directory is 4,071,503 bytes including its checksum manifest. It is documentation-only and is not loaded or packaged by the client. Shipping M22 audio remains 741,692 bytes across 13 mono PCM sources, with 741,120 decoded bytes and 14 fixed runtime nodes.

The canonical capture was produced on 2026-08-27 with Mesa llvmpipe. Movie encoding increased the observed frame cost and is not used as the Block 7 gameplay measurement. The separate non-recording graphical measurement remains 18.403 ms mean, 37.446 ms maximum, -17.44 dBFS peak, -26.93 dBFS RMS, and seven simultaneous voices in that software-rendered environment.

