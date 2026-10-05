#!/usr/bin/env python3
"""Original Prism Warden signals, synthesized without recordings or samples.

Short glass-like tones use different pitch contours and pulse counts for a
lane, center, perimeter, phase shift and recovery. Shared bounded audio pools
play these at the Effects level; every signal also has a semantic caption.
"""
import math
from pathlib import Path
import struct
import wave

root = Path(__file__).resolve().parents[1] / 'client/game/audio/m34'
root.mkdir(parents=True, exist_ok=True)
rate = 22050
patterns = {
    'lane': [(0.00, 440), (0.16, 660)],
    'center': [(0.00, 550), (0.18, 440), (0.36, 330)],
    'perimeter': [(0.00, 330), (0.18, 440), (0.36, 550)],
    'shift': [(0.00, 220), (0.16, 330), (0.32, 440), (0.48, 660)],
    'recovery': [(0.00, 660), (0.12, 880)],
}
for name, notes in patterns.items():
    samples = []
    for index in range(round((notes[-1][0] + 0.30) * rate)):
        t = index / rate
        value = 0.0
        for start, hz in notes:
            age = t - start
            if 0 <= age < 0.30:
                envelope = min(1.0, age / 0.015) * math.exp(-age * 14) * min(1.0, (0.30-age) / 0.035)
                value += 0.13 * envelope * (math.sin(2*math.pi*hz*age) + 0.25*math.sin(2*math.pi*hz*2.01*age))
        samples.append(round(value * 32767))
    with wave.open(str(root / f'prism_{name}.wav'), 'wb') as target:
        target.setnchannels(1)
        target.setsampwidth(2)
        target.setframerate(rate)
        target.writeframes(struct.pack('<' + 'h'*len(samples), *samples))
    print(name, f'peak={max(abs(v) for v in samples)/32767:.3f}', f'duration={len(samples)/rate:.2f}s')
