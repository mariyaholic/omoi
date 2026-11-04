"""Shared DSP for the Omoi v3 score: voices, reverb, limiter, loudness, AAC encode.

Copied from the Rev 2 audio lab (treatments/common.py); only the cut list, the paths and the
final encode differ. Everything is synthesised with numpy/scipy, except the FluidR3_GM piano
render (see LICENSES.md). All randomness is seeded.
"""
from pathlib import Path
import json
import re
import struct
import subprocess
import tempfile

import numpy as np
import pyloudnorm
import soundfile as sf
from scipy import signal
from scipy.ndimage import minimum_filter1d, uniform_filter1d

SR = 48000
DUR = 40
N = SR * DUR
HERE = Path(__file__).resolve().parent
SF2 = '/usr/share/sounds/sf2/FluidR3_GM.sf2'

# Picture hit points of the v3 cut list; render.py asserts it uses the same list.
CUTS = [0, 3, 7, 8.5, 11, 13, 15.5, 18, 21, 22.5, 26, 28.9, 31, 34.5, 40]
HITS = CUTS[1:]

TARGET_LUFS = -14.0
MAX_DBTP = -2.0


def db(x):
    return 10.0 ** (x / 20.0)


def hz(midi):
    return 440.0 * 2.0 ** ((np.asarray(midi, float) - 69.0) / 12.0)


def tgrid(dur):
    return np.arange(int(round(dur * SR))) / SR


def curve(points, t=None):
    """Piecewise-linear automation sampled at t (default: the whole film)."""
    xs, ys = zip(*points)
    return np.interp(np.arange(N) / SR if t is None else t, xs, ys)


class Bus:
    """A 40.000 s stereo buffer that voices are placed on, sample-accurately."""

    def __init__(self):
        self.x = np.zeros((N, 2))

    def add(self, sig, t=0.0, gain=1.0, pan=0.0):
        sig = np.asarray(sig, float)
        if sig.ndim == 1:
            angle = (pan + 1.0) * np.pi / 4.0
            sig = np.stack([sig * np.cos(angle), sig * np.sin(angle)], 1) * np.sqrt(2.0)
        a = int(round(t * SR))
        skip = max(0, -a)
        a = max(0, a)
        length = min(len(sig) - skip, N - a)
        if length > 0:
            self.x[a:a + length] += sig[skip:skip + length] * gain
        return self


# ---- envelopes -------------------------------------------------------------------------

def ar(n, attack, release):
    """Unit gate with raised-cosine attack and release (seconds); never clicks."""
    e = np.ones(n)
    a, r = min(n, int(attack * SR)), min(n, int(release * SR))
    if a:
        e[:a] *= 0.5 - 0.5 * np.cos(np.pi * np.arange(a) / a)
    if r:
        e[n - r:] *= 0.5 + 0.5 * np.cos(np.pi * np.arange(r) / r)
    return e


def perc(dur, attack=0.002, decay=0.2, release=0.01):
    t = tgrid(dur)
    return np.exp(-t / decay) * ar(len(t), attack, release)


# ---- oscillators and voices ------------------------------------------------------------

def _blep(p, dt):
    out = np.zeros_like(p)
    m = p < dt
    x = p[m] / dt[m]
    out[m] = x + x - x * x - 1.0
    m = p > 1.0 - dt
    x = (p[m] - 1.0) / dt[m]
    out[m] = x * x + x + x + 1.0
    return out


def square(freq, dur, duty=0.5, phase=0.0):
    """Band-limited (polyBLEP) pulse wave: the SSG / PSG voice."""
    t = tgrid(dur)
    dt = np.full(len(t), float(freq) / SR)
    p = (freq * t + phase) % 1.0
    naive = np.where(p < duty, 1.0, -1.0)
    return (naive + _blep(p, dt) - _blep((p - duty) % 1.0, dt)) * 0.5


def fm(freq, dur, ratio=1.0, index=1.0, ratio2=None, index2=0.0, vibrato=0.0, vib_rate=5.5, vib_delay=0.15):
    """Serial FM stack in the OPN style. `index`/`index2` may be scalars or per-sample envelopes."""
    t = tgrid(dur)
    pitch = 0.0
    if vibrato:
        pitch = vibrato * np.sin(2 * np.pi * vib_rate * t) * np.clip((t - vib_delay) / 0.2, 0, 1)
    top = 0.0
    if ratio2 is not None:
        top = index2 * np.sin(2 * np.pi * freq * ratio2 * t)
    mod = index * np.sin(2 * np.pi * freq * ratio * t + top)
    return np.sin(2 * np.pi * freq * t + pitch + mod)


def bell(freq, dur, bright=1.0, decay=1.0):
    """Soft glass bell: inharmonic partials, the upper ones dying first. Peak is about 1."""
    t = tgrid(dur)
    out = np.zeros_like(t)
    for ratio, amp, life in [(1, 1, 1), (2.0, .34, .62), (3.01, .16 * bright, .38), (4.2, .07 * bright, .22), (5.43, .035 * bright, .14)]:
        if freq * ratio < SR * 0.45:
            out += amp * np.sin(2 * np.pi * freq * ratio * t + ratio) * np.exp(-t / (decay * life))
    return out * ar(len(t), 0.0015, 0.05) / 1.25


def additive_pad(midis, dur, cutoff, rng, voices=3, detune=5.0, rolloff=1.0):
    """Evolving pad. Harmonics follow `cutoff` (Hz, per-sample), so brightness is automatable
    without a time-varying filter and without aliasing. Unit-free; scale at the bus."""
    t = tgrid(dur)
    cutoff = np.broadcast_to(cutoff, t.shape)
    out = np.zeros((len(t), 2))
    for midi in midis:
        f0 = float(hz(midi))
        for v in range(voices):
            cents = detune * (v - (voices - 1) / 2) + rng.normal(0, 0.6)
            f = f0 * 2 ** (cents / 1200)
            drift = 0.25 * np.sin(2 * np.pi * (0.07 + 0.05 * rng.random()) * t + rng.random() * 6.28)
            pan = np.clip(0.75 * (2 * v / max(1, voices - 1) - 1) + rng.normal(0, 0.1), -1, 1)
            gl, gr = np.cos((pan + 1) * np.pi / 4), np.sin((pan + 1) * np.pi / 4)
            for k in range(1, 25):
                fk = f * k
                if fk > 16000:
                    break
                amp = k ** -rolloff * np.exp(-((fk / cutoff) ** 2))
                if amp.max() < 2e-3:
                    continue
                s = amp * np.sin(2 * np.pi * fk * t + k * drift + rng.random() * 6.28)
                out[:, 0] += s * gl
                out[:, 1] += s * gr
    return out / np.sqrt(len(midis) * voices)


def shaped_noise(dur, fc, width_oct, rng):
    """Noise with a log-gaussian spectral band centred on fc(t) (scalar or callable of seconds)."""
    n = int(round(dur * SR))
    f, frames, z = signal.stft(rng.normal(size=n + 4096), SR, nperseg=2048, noverlap=1536)
    centre = np.array([fc(x) for x in frames]) if callable(fc) else np.full(len(frames), float(fc))
    weight = np.exp(-0.5 * (np.log2(np.maximum(f[:, None], 1.0) / centre[None, :]) / width_oct) ** 2)
    x = signal.istft(z * weight, SR, nperseg=2048, noverlap=1536)[1][:n]
    return x / (np.std(x) + 1e-12)


def sos(kind, fc, x, order=2):
    return signal.sosfilt(signal.butter(order, fc, kind, fs=SR, output='sos'), x, axis=0)


def keypress(rng, weight=1.0):
    """Quiet-switch keystroke: a low wooden thock plus a short plastic tick and a release tick."""
    t = tgrid(0.09)
    thock = np.sin(2 * np.pi * (rng.uniform(165, 215) * t + 3 * (1 - np.exp(-t * 90)))) * np.exp(-t * 95)
    tick = sos('bandpass', [rng.uniform(1400, 2000), rng.uniform(4200, 5600)], rng.normal(size=len(t))) * np.exp(-t * 420)
    release = np.zeros_like(t)
    at = int(rng.uniform(0.038, 0.052) * SR)
    release[at:] = 0.25 * sos('bandpass', [1800, 5200], rng.normal(size=len(t) - at)) * np.exp(-t[:len(t) - at] * 650)
    return (weight * 0.55 * thock + 0.42 * tick + release) * ar(len(t), 0.0004, 0.006)


# ---- space -----------------------------------------------------------------------------

def make_ir(rt60, rng, predelay=0.012, lp=6500, hp=140):
    """Decorrelated stereo impulse response; the highs decay faster than the mids. Unit energy."""
    n = int(rt60 * 1.4 * SR)
    t = np.arange(n) / SR
    channels = []
    for _ in range(2):
        mids = sos('bandpass', [hp, lp * 0.45], rng.normal(size=n)) * np.exp(-6.91 * t / rt60)
        highs = sos('bandpass', [lp * 0.45, lp], rng.normal(size=n)) * np.exp(-6.91 * t / (rt60 * 0.5))
        y = (mids + 0.6 * highs) * ar(n, 0.004, 0.05)
        channels.append(np.concatenate([np.zeros(int(predelay * SR)), y]))
    ir = np.stack(channels, 1)
    return ir / np.sqrt((ir ** 2).sum(0, keepdims=True))


def reverb(x, ir):
    """Wet signal only, truncated to the film."""
    return np.stack([signal.fftconvolve(x[:, c], ir[:, c])[:len(x)] for c in (0, 1)], 1)


def echo(x, delay, feedback, lp=4500, repeats=6):
    """Cross-feeding (ping-pong) tempo delay; wet signal only."""
    out = np.zeros_like(x)
    d = int(round(delay * SR))
    tap = x
    for i in range(1, repeats + 1):
        tap = sos('lowpass', lp, tap, 1) * feedback
        tap = tap[:, ::-1]
        out[d * i:] += tap[:len(x) - d * i]
    return out


# ---- fluidsynth (treatment A piano only) ------------------------------------------------

def _vlq(v):
    out = [v & 127]
    v >>= 7
    while v:
        out.insert(0, (v & 127) | 128)
        v >>= 7
    return bytes(out)


def write_midi(path, notes, program=0):
    """notes: (start_s, dur_s, midi, velocity). 960 ticks per second, so seconds map exactly."""
    events = [(0, 0, bytes([0xC0, program]))]
    for start, dur, midi, vel in notes:
        events.append((round(start * 960), 1, bytes([0x90, midi, vel])))
        events.append((round((start + dur) * 960), 0, bytes([0x80, midi, 0])))
    events.sort(key=lambda e: e[:2])
    track = b'\x00\xff\x51\x03\x07\xa1\x20'
    last = 0
    for tick, _, data in events:
        track += _vlq(tick - last) + data
        last = tick
    track += _vlq(max(0, (DUR + 2) * 960 - last)) + b'\xff\x2f\x00'
    Path(path).write_bytes(b'MThd' + struct.pack('>IHHH', 6, 0, 1, 480) + b'MTrk' + struct.pack('>I', len(track)) + track)


def fluid_render(notes, program=0, gain=0.7):
    """Render notes dry (no fluidsynth reverb or chorus) from FluidR3_GM. Returns (N, 2)."""
    with tempfile.TemporaryDirectory() as d:
        mid, wav = Path(d) / 'n.mid', Path(d) / 'n.wav'
        write_midi(mid, notes, program)
        subprocess.run(['fluidsynth', '-ni', '-R', '0', '-C', '0', '-g', str(gain), '-r', str(SR), '-O', 'float',
                        '-T', 'wav', '-F', str(wav), SF2, str(mid)], check=True, capture_output=True)
        x, sr = sf.read(wav, always_2d=True)
    assert sr == SR and np.abs(x).max() > 1e-3, 'fluidsynth render is empty or at the wrong rate'
    out = np.zeros((N, 2))
    out[:min(N, len(x))] = x[:N]
    return out


# ---- mastering ------------------------------------------------------------------------

def true_peak_db(x):
    """4x-oversampled peak, the same factor as ffmpeg's ebur128 peak=true."""
    return 20 * np.log10(np.abs(signal.resample_poly(x, 4, 1, axis=0)).max() + 1e-12)


def tp_limit(x, ceil_db, window=0.006, hold=0.025, release=0.14):
    """Linked look-ahead true-peak limiter. Returns (limited, gain curve)."""
    ceil = db(ceil_db)
    over = np.abs(signal.resample_poly(x, 4, 1, axis=0))[:len(x) * 4].reshape(len(x), 4, 2)
    need = np.minimum(1.0, ceil / np.maximum(over.max((1, 2)), 1e-12))
    w, h = int(window * SR) | 1, int(hold * SR)
    # Each output gain averages w samples of `held`. Every one of those must already see this peak,
    # so the minimum spans w/2 ahead and w/2 + hold behind; that is what makes the ceiling hard.
    size = w + h
    held = minimum_filter1d(need, size=size, origin=w // 2 + h - size // 2, mode='nearest')
    smooth = uniform_filter1d(held, size=w, mode='nearest')
    # Release can only lower the gain further, so it cannot break the ceiling.
    gain = smooth.copy()
    coef = 1.0 / (release * SR)
    g = 1.0
    for i, s in enumerate(smooth.tolist()):
        g = min(s, g + (1.0 - g) * coef)
        gain[i] = g
    return x * gain[:, None], gain


def edge_fade(x, head=0.004, tail=0.03):
    """Guarantees the first and last samples are exactly zero."""
    return x * ar(len(x), head, tail)[:, None]


def measure(path):
    """Integrated, LRA, true peak via ffmpeg ebur128, as the brief specifies."""
    r = subprocess.run(['ffmpeg', '-hide_banner', '-nostats', '-i', str(path), '-map', '0:a:0', '-af', 'ebur128=peak=true',
                        '-f', 'null', '-'], capture_output=True, text=True, check=True).stderr
    tail = r[r.rindex('Summary:'):]
    return {'lufs': float(re.search(r'I:\s+(-?[\d.]+) LUFS', tail)[1]),
            'lra': float(re.search(r'LRA:\s+(-?[\d.]+) LU', tail)[1]),
            'dbtp': float(re.search(r'Peak:\s+(-?[\d.]+) dBFS', tail)[1])}


def encode(wav, out):
    """48 kHz stereo AAC. -ar is explicit: no filter in this chain may change the sample rate."""
    subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-bitexact', '-i', str(wav), '-map', '0:a:0', '-c:a', 'aac', '-b:a', '192k',
                    '-ar', str(SR), '-ac', '2', '-t', str(DUR), '-movflags', '+faststart', str(out)], check=True)


def finish(stems, out_dir, limit_db=-2.6):
    """Master the stem sum to target, write stems + mix, encode, and measure the encoded file.

    Loudness and true peak are closed-loop on the AAC, not on the WAV. The caller muxes that AAC
    unchanged (-c:a copy) and measures the muxed file again."""
    out_dir = Path(out_dir)
    (out_dir / 'stems').mkdir(parents=True, exist_ok=True)
    out = out_dir / 'score.m4a'
    wav = out_dir / 'mix.wav'
    raw = sum(stems.values())
    trim = TARGET_LUFS - pyloudnorm.Meter(SR).integrated_loudness(raw)
    for _ in range(6):
        mix, gain = tp_limit(raw * db(trim), limit_db)
        mix = edge_fade(mix)
        sf.write(wav, mix, SR, subtype='PCM_24')
        encode(wav, out)
        m = measure(out)
        print(f'score: trim {trim:+.2f} dB ceil {limit_db:.2f} -> {m}', flush=True)
        if abs(m['lufs'] - TARGET_LUFS) <= 0.1 and m['dbtp'] <= MAX_DBTP - 0.1:
            break
        trim += TARGET_LUFS - m['lufs']
        limit_db -= max(0.0, m['dbtp'] - (MAX_DBTP - 0.15))
    else:
        raise SystemExit('score: did not converge on loudness and true peak')
    for name, stem in stems.items():
        sf.write(out_dir / 'stems' / f'{name}.wav', stem * db(trim), SR, subtype='FLOAT')
    gr = -20 * np.log10(gain)
    m.update(trim_db=round(trim, 2), limit_ceiling_db=round(limit_db, 2), max_gain_reduction_db=round(float(gr.max()), 2),
             pct_time_limiting=round(float((gr > 0.05).mean() * 100), 2), file=out.name)
    (out_dir / 'metrics.json').write_text(json.dumps(m, indent=2) + '\n')
    return m
