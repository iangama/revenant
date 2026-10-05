#!/usr/bin/env python3
"""Original deterministic Meridian ambience/tonal bed; Python standard library.

No recordings or third-party samples. Integer-period oscillators make a seamless
16-second loop. Quiet filtered noise evokes air through glass; sparse harmonics
provide an untimed exploration music state within the existing Ambience bus.
"""
import math
from pathlib import Path
import random
import struct
import wave

root = Path(__file__).resolve().parents[1]
output = root / "client/game/audio/m33/meridian.wav"
output.parent.mkdir(parents=True, exist_ok=True)
rate, seconds = 22050, 16
rng = random.Random(33)
samples = []
wind = 0.0
for index in range(rate * seconds):
    t = index / rate
    wind = wind * 0.992 + rng.uniform(-1, 1) * 0.008
    envelope = math.sin(math.pi * t / seconds) ** 2
    tone = sum(math.sin(2 * math.pi * hz * t) for hz in (110, 165, 220)) / 3
    bell = math.sin(2 * math.pi * 330 * t) * (0.5 + 0.5 * math.cos(2 * math.pi * t / 8)) ** 8
    value = (0.14 * wind + 0.027 * tone + 0.014 * bell) * envelope
    samples.append(struct.pack("<h", round(value * 32767)))
with wave.open(str(output), "wb") as target:
    target.setnchannels(1)
    target.setsampwidth(2)
    target.setframerate(rate)
    target.writeframes(b"".join(samples))
print(f"Generated original Meridian loop: {output.stat().st_size} bytes")
