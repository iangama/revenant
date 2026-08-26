#!/usr/bin/env python3
"""Render the original M22 Block 5 ambience and system cues."""

import math
import random
import struct
import wave
from pathlib import Path

RATE = 48_000
ROOT = Path(__file__).resolve().parents[4]
OUTPUT = ROOT / "client/game/audio/m22"


def render(name: str, duration: float, sample) -> None:
    OUTPUT.mkdir(parents=True, exist_ok=True)
    frames = bytearray()
    for index in range(round(duration * RATE)):
        value = max(-1.0, min(1.0, sample(index / RATE, index)))
        frames.extend(struct.pack("<h", round(value * 32767)))
    with wave.open(str(OUTPUT / name), "wb") as target:
        target.setnchannels(1)
        target.setsampwidth(2)
        target.setframerate(RATE)
        target.writeframes(frames)


def ambience(t: float, _index: int) -> float:
    drift = 0.10 * math.sin(math.tau * 55 * t)
    machinery = 0.035 * math.sin(math.tau * 110 * t + 0.3 * math.sin(math.tau * 0.5 * t))
    relay = 0.025 * math.sin(math.tau * 220 * t) * (0.5 + 0.5 * math.sin(math.tau * 0.25 * t))
    return drift + machinery + relay


def system_ready(t: float, _index: int) -> float:
    envelope = math.sin(math.pi * min(t / 0.36, 1.0)) ** 2
    return envelope * (0.16 * math.sin(math.tau * 660 * t) + 0.08 * math.sin(math.tau * 990 * t))


noise = random.Random(2205)


def door_unlock(t: float, _index: int) -> float:
    body = 0.14 * math.sin(math.tau * (82 - 24 * t) * t) * math.exp(-4.0 * t)
    latch = (noise.random() * 2.0 - 1.0) * 0.10 * math.exp(-18.0 * t)
    relay = 0.06 * math.sin(math.tau * 240 * t) * math.exp(-7.0 * max(0.0, t - 0.12)) if t >= 0.12 else 0.0
    return body + latch + relay


render("relay_hub_ambience.wav", 4.0, ambience)
render("system_ready.wav", 0.36, system_ready)
render("relay_door_unlock.wav", 0.60, door_unlock)
