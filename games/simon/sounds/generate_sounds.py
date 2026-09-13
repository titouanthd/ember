#!/usr/bin/env python3
# generate_sounds.py — run once, commits the .wav files
# Requires Python 3 (no external deps).

import math
import struct
import wave

SAMPLE_RATE = 44100
DURATION = 0.4  # seconds
AMPLITUDE = 0.3  # 0..1, leaves headroom

SOUNDS = {
    "green":  415.0,
    "red":    310.0,
    "yellow": 252.0,
    "blue":   209.0,
}

def write_wav(path, freq):
    n_samples = int(SAMPLE_RATE * DURATION)
    with wave.open(path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)  # 16-bit
        w.setframerate(SAMPLE_RATE)
        frames = bytearray()
        for i in range(n_samples):
            t = i / SAMPLE_RATE
            # Apply a tiny fade-in/out to avoid clicks
            env = 1.0
            fade = 0.02 * SAMPLE_RATE  # 20ms
            if i < fade:
                env = i / fade
            elif i > n_samples - fade:
                env = (n_samples - i) / fade
            sample = AMPLITUDE * env * math.sin(2 * math.pi * freq * t)
            frames += struct.pack("<h", int(sample * 32767))
        w.writeframes(bytes(frames))

if __name__ == "__main__":
    for name, freq in SOUNDS.items():
        write_wav(f"{name}.wav", freq)
        print(f"Wrote {name}.wav ({freq} Hz)")