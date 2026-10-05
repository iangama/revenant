#!/usr/bin/env python3
"""Original M35 weapon signals; deterministic synthesis, no external samples."""
import math
from pathlib import Path
import random
import struct
import wave

root = Path(__file__).resolve().parents[1] / 'client/game/audio/m35'
root.mkdir(parents=True, exist_ok=True)
rate = 22050
for name in ['scatter_caster', 'rail_driver']:
    rng = random.Random(35)
    samples = []
    duration = 0.26 if name == 'scatter_caster' else 0.32
    for index in range(round(duration * rate)):
        t = index / rate
        envelope = min(1, t / 0.008) * math.exp(-t * 17) * min(1, (duration-t) / 0.03)
        if name == 'scatter_caster':
            signal = 0.55 * rng.uniform(-1, 1) + 0.45 * math.sin(2 * math.pi * (160*t-120*t*t))
        else:
            signal = math.sin(2 * math.pi * (1100*t-1200*t*t)) + 0.18 * math.sin(2 * math.pi * 180*t)
        samples.append(round(0.28 * envelope * signal * 32767))
    with wave.open(str(root / f'{name}.wav'), 'wb') as target:
        target.setnchannels(1)
        target.setsampwidth(2)
        target.setframerate(rate)
        target.writeframes(struct.pack('<' + 'h'*len(samples), *samples))
    print(name, f'peak={max(abs(v) for v in samples)/32767:.3f}', f'duration={duration:.2f}s')
