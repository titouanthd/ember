#!/usr/bin/env python3
"""Generate Pagoda's sound effects.

The repo ships only generated WAVs — no downloaded or licensed audio.
Run this once after changing a sound, then commit the files in
`assets/sfx/`.

**Always generate at 44.1 kHz.** quad-snd (below macroquad) resamples
anything else with a nearest-neighbour filter, which audibly crackles.
`src/audio.rs` has a test that locks this in.

Pure standard library: no numpy, no external tools.

    python3 tools/generate-sounds.py
"""

import math
import os
import random
import struct
import wave

SAMPLE_RATE = 44_100
OUT_DIR = os.path.join(os.path.dirname(__file__), "..", "assets", "sfx")


# ----------------------------------------------------------------------
# Filters
# ----------------------------------------------------------------------

class Biquad:
    """RBJ biquad. `kind` is "lowpass", "highpass" or "bandpass"."""

    def __init__(self, kind: str, freq: float, q: float) -> None:
        w0 = 2.0 * math.pi * freq / SAMPLE_RATE
        cos_w0 = math.cos(w0)
        alpha = math.sin(w0) / (2.0 * q)

        if kind == "lowpass":
            b0 = (1.0 - cos_w0) / 2.0
            b1 = 1.0 - cos_w0
            b2 = b0
        elif kind == "highpass":
            b0 = (1.0 + cos_w0) / 2.0
            b1 = -(1.0 + cos_w0)
            b2 = b0
        elif kind == "bandpass":
            b0 = alpha
            b1 = 0.0
            b2 = -alpha
        else:
            raise ValueError(kind)

        a0 = 1.0 + alpha
        self.b0 = b0 / a0
        self.b1 = b1 / a0
        self.b2 = b2 / a0
        self.a1 = (-2.0 * cos_w0) / a0
        self.a2 = (1.0 - alpha) / a0
        self.x1 = self.x2 = self.y1 = self.y2 = 0.0

    def process(self, x: float) -> float:
        y = (
            self.b0 * x
            + self.b1 * self.x1
            + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2
        )
        self.x2, self.x1 = self.x1, x
        self.y2, self.y1 = self.y1, y
        return y


def filter_buffer(samples: list[float], kind: str, freq: float, q: float = 0.707) -> list[float]:
    f = Biquad(kind, freq, q)
    return [f.process(s) for s in samples]


# ----------------------------------------------------------------------
# Output chain
# ----------------------------------------------------------------------

def write_wav(
    name: str,
    samples: list[float],
    peak: float,
    lowpass_hz: float = 7500.0,
) -> None:
    """Finalize and write one clip.

    Removes DC, filters out harsh high end, fades the edges, normalizes
    to `peak` (-5 to -8 dBFS, leaving mixer headroom), and writes
    44.1 kHz / 16-bit / mono PCM.
    """
    # Remove any DC offset before filtering.
    mean = sum(samples) / len(samples)
    x = [s - mean for s in samples]

    # Keep the low end tight and the high end soft.
    x = filter_buffer(x, "highpass", 25.0)
    x = filter_buffer(x, "lowpass", lowpass_hz)

    # Edge fades: no clicks at the first or last sample.
    fade_in = int(0.004 * SAMPLE_RATE)
    fade_out = int(0.012 * SAMPLE_RATE)
    for i in range(min(fade_in, len(x))):
        x[i] *= i / fade_in
    for i in range(min(fade_out, len(x))):
        x[-1 - i] *= i / fade_out

    current_peak = max((abs(s) for s in x), default=0.0) or 1.0
    scale = peak / current_peak

    frames = bytearray()
    for s in x:
        v = max(-1.0, min(1.0, s * scale))
        frames += struct.pack("<h", int(round(v * 32767)))

    os.makedirs(OUT_DIR, exist_ok=True)
    path = os.path.join(OUT_DIR, name)
    with wave.open(path, "w") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(bytes(frames))

    seconds = len(x) / SAMPLE_RATE
    print(
        f"{name:18s} {seconds:5.2f}s {SAMPLE_RATE} Hz "
        f"peak={peak:.2f} {len(frames) / 1024:6.1f} KiB"
    )


# ----------------------------------------------------------------------
# Building blocks
# ----------------------------------------------------------------------

def bell(freq: float, duration: float, amp: float = 1.0) -> list[float]:
    """Soft struck-bell tone with gently rolled-off partials."""
    n = int(SAMPLE_RATE * duration)
    # (ratio, amplitude, decay time constant in seconds)
    partials = [
        (1.0, 1.0, 0.22),
        (2.0, 0.25, 0.12),
        (3.01, 0.10, 0.07),
        (4.17, 0.04, 0.035),
    ]
    out = []
    attack = 0.004
    release = 0.03
    for i in range(n):
        t = i / SAMPLE_RATE
        s = 0.0
        for ratio, partial_amp, tau in partials:
            s += partial_amp * math.exp(-t / tau) * math.sin(
                2.0 * math.pi * freq * ratio * t
            )
        if t < attack:
            s *= t / attack
        remaining = duration - t
        if remaining < release:
            s *= remaining / release
        out.append(amp * s)
    return out


def mix_at(base: list[float], clip: list[float], start_s: float) -> None:
    offset = int(start_s * SAMPLE_RATE)
    for i, value in enumerate(clip):
        if offset + i >= len(base):
            break
        base[offset + i] += value


# ----------------------------------------------------------------------
# Sounds
# ----------------------------------------------------------------------

def make_drop() -> None:
    """Short warm wooden 'tok' — neutral, physical, no falling bomb."""
    duration = 0.18
    n = int(SAMPLE_RATE * duration)
    rng = random.Random(11)
    out = []
    phase = 0.0
    lp = 0.0
    for i in range(n):
        t = i / SAMPLE_RATE
        # A 20 ms, 18 Hz settle — enough weight, no audible glide.
        freq = 220.0 + 18.0 * math.exp(-t / 0.020)
        phase += 2.0 * math.pi * freq / SAMPLE_RATE

        # Woodblock-ish modes: fundamental plus two quick upper partials.
        body = (
            math.sin(phase)
            + 0.45 * math.sin(2.0 * phase) * math.exp(-t / 0.025)
            + 0.22 * math.sin(3.9 * phase) * math.exp(-t / 0.012)
        )
        body *= math.exp(-t / 0.048)
        # A little low body so the block feels heavy.
        body += 0.30 * math.sin(2.0 * math.pi * 92.0 * t) * math.exp(-t / 0.035)

        # One soft, low-passed contact tick (no bright click).
        alpha = 1.0 - math.exp(-2.0 * math.pi * 1200.0 / SAMPLE_RATE)
        lp += alpha * (rng.uniform(-1.0, 1.0) - lp)
        click = lp * math.exp(-t / 0.004) * 0.08

        out.append(body + click)
    write_wav("drop.wav", out, peak=0.55, lowpass_hz=6000.0)


def make_slice() -> None:
    """Short soft 'shk' — an airy cut, not a hissy swoosh."""
    duration = 0.16
    n = int(SAMPLE_RATE * duration)
    rng = random.Random(22)
    lead = int(0.008 * SAMPLE_RATE)
    band_a = Biquad("bandpass", 1800.0, 0.80)
    band_b = Biquad("bandpass", 1800.0, 0.80)

    out = []
    for i in range(n):
        y = band_b.process(band_a.process(rng.uniform(-1.0, 1.0)))

        if i < lead:
            env = 0.0
        else:
            t = (i - lead) / SAMPLE_RATE
            env = (1.0 - math.exp(-t / 0.010)) * math.exp(-t / 0.045)
        remaining = duration - i / SAMPLE_RATE
        if remaining < 0.015:
            env *= remaining / 0.015
        out.append(y * env)
    write_wav("slice.wav", out, peak=0.40, lowpass_hz=5500.0)


PERFECT_FREQS = [
    523.25,  # C5
    587.33,  # D5
    659.25,  # E5
    783.99,  # G5
    880.00,  # A5
    1046.50,  # C6
]


def make_perfects() -> None:
    for i, freq in enumerate(PERFECT_FREQS, start=1):
        write_wav(
            f"perfect_{i}.wav",
            bell(freq, 0.50),
            peak=0.50,
            lowpass_hz=8000.0,
        )


def make_game_over() -> None:
    """Two descending notes with a soft low rumble underneath."""
    duration = 0.95
    n = int(SAMPLE_RATE * duration)
    rng = random.Random(33)
    out = []
    lp = 0.0
    notes = [
        # (frequency, start, length, decay tau)
        (196.00, 0.00, 0.45, 0.22),  # G3
        (146.83, 0.28, 0.67, 0.28),  # D3
    ]
    for i in range(n):
        t = i / SAMPLE_RATE
        s = 0.0
        for freq, start, length, tau in notes:
            local = t - start
            if 0.0 <= local < length:
                env = (1.0 - math.exp(-local / 0.010)) * math.exp(-local / tau)
                if local > length - 0.06:
                    env *= (length - local) / 0.06
                s += env * (
                    math.sin(2.0 * math.pi * freq * local)
                    + 0.25 * math.sin(4.0 * math.pi * freq * local)
                )

        # Soft filtered rumble, not hiss.
        alpha = 1.0 - math.exp(-2.0 * math.pi * 220.0 / SAMPLE_RATE)
        lp += alpha * (rng.uniform(-1.0, 1.0) - lp)
        s += lp * math.exp(-t / 0.35) * 0.25
        s += 0.20 * math.sin(2.0 * math.pi * 49.0 * t) * math.exp(-t / 0.40)

        out.append(s)
    write_wav("game_over.wav", out, peak=0.55, lowpass_hz=4000.0)


def make_start() -> None:
    """Quick rising fifth; subtle enough for every restart."""
    duration = 0.22
    n = int(SAMPLE_RATE * duration)
    out = [0.0] * n
    mix_at(out, bell(523.25, 0.10, amp=0.7), 0.00)
    mix_at(out, bell(783.99, 0.14, amp=0.8), 0.06)
    write_wav("start.wav", out, peak=0.40, lowpass_hz=7000.0)


def make_new_best() -> None:
    """Bright arpeggio, delayed so it lands after the game-over thud."""
    duration = 1.15
    n = int(SAMPLE_RATE * duration)
    out = [0.0] * n
    arpeggio = [523.25, 659.25, 783.99, 1046.50]
    for i, freq in enumerate(arpeggio):
        start = 0.35 + i * 0.09
        length = 0.55 if i == len(arpeggio) - 1 else 0.32
        mix_at(out, bell(freq, length, amp=0.75), start)
    write_wav("new_best.wav", out, peak=0.50, lowpass_hz=7500.0)


if __name__ == "__main__":
    make_drop()
    make_slice()
    make_perfects()
    make_game_over()
    make_start()
    make_new_best()
