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


def shaped_tone(t: float, duration: float, frequencies: tuple[float, ...], gain: float = 0.16) -> float:
    envelope = math.sin(math.pi * min(t / duration, 1.0)) ** 2
    return envelope * sum(gain * math.sin(math.tau * frequency * t) / len(frequencies) for frequency in frequencies)


operator_noise = random.Random(2206)
drone_noise = random.Random(2207)
warden_noise = random.Random(2208)


def operator_servo(t: float, _index: int) -> float:
    return shaped_tone(t, 0.16, (145, 290), 0.12) + (operator_noise.random() * 2 - 1) * 0.025 * math.exp(-18 * t)


def pulse_rifle(t: float, _index: int) -> float:
    return shaped_tone(t, 0.28, (180, 540, 900), 0.20)


def arc_sidearm(t: float, _index: int) -> float:
    return shaped_tone(t, 0.20, (260, 780, 1170), 0.18)


def confirmed_impact(t: float, _index: int) -> float:
    return shaped_tone(t, 0.14, (95, 380), 0.15) * math.exp(-5 * t)


def relay_drone(t: float, _index: int) -> float:
    return shaped_tone(t, 0.30, (210, 315), 0.14) + (drone_noise.random() * 2 - 1) * 0.025 * math.exp(-9 * t)


def warden(t: float, _index: int) -> float:
    return shaped_tone(t, 0.45, (58, 116, 174), 0.18) + (warden_noise.random() * 2 - 1) * 0.018 * math.exp(-6 * t)


def enemy_defeat(t: float, _index: int) -> float:
    return 0.15 * math.sin(math.tau * (240 - 150 * t) * t) * math.exp(-5 * t)


def player_damage(t: float, _index: int) -> float:
    return shaped_tone(t, 0.20, (72, 144), 0.20) * math.exp(-3 * t)


def cooldown_tick(t: float, _index: int) -> float:
    return shaped_tone(t, 0.08, (520,), 0.10)


def completion(t: float, _index: int) -> float:
    return shaped_tone(t, 0.60, (220, 330, 440), 0.16)


render("operator_servo.wav", 0.16, operator_servo)
render("pulse_rifle_confirmed.wav", 0.28, pulse_rifle)
render("arc_sidearm_confirmed.wav", 0.20, arc_sidearm)
render("confirmed_impact.wav", 0.14, confirmed_impact)
render("relay_drone_cue.wav", 0.30, relay_drone)
render("warden_cue.wav", 0.45, warden)
render("enemy_defeat.wav", 0.35, enemy_defeat)
render("player_damage.wav", 0.20, player_damage)
render("cooldown_tick.wav", 0.08, cooldown_tick)
render("completion.wav", 0.60, completion)
