"""Measure the final omoi-v3.mp4 and write qa/validation.json and qa/copy-review.json.

Everything is measured on the muxed file except the score-stem and camera rows, which say so.
Usage: python source/validate.py
"""
from pathlib import Path
import hashlib
import json
import re
import subprocess
import sys

import numpy as np
import pyloudnorm
from scipy import signal
import soundfile as sf

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'source'))
sys.path.insert(0, str(ROOT / 'source' / 'audio'))
import render  # noqa: E402

MP4 = ROOT / 'omoi-v3.mp4'
SCORE = render.BUILD / 'score'
SR = 48000


def run(*args):
    return subprocess.run(args, capture_output=True, text=True, check=True)


def ebur128(path):
    tail = run('ffmpeg', '-hide_banner', '-nostats', '-i', str(path), '-map', '0:a:0', '-af', 'ebur128=peak=true', '-f', 'null', '-').stderr
    tail = tail[tail.rindex('Summary:'):]
    return {'integrated_lufs': float(re.search(r'I:\s+(-?[\d.]+) LUFS', tail)[1]), 'lra_lu': float(re.search(r'LRA:\s+(-?[\d.]+) LU', tail)[1]),
            'true_peak_dbtp': float(re.search(r'Peak:\s+(-?[\d.]+) dBFS', tail)[1])}


def decode_audio(path):
    raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:a:0', '-f', 'f32le', '-'], capture_output=True, check=True).stdout
    return np.frombuffer(raw, np.float32).reshape(-1, 2).astype(float)


def stream_hash(path, spec):
    raw = subprocess.run(['ffmpeg', '-v', 'error', '-i', str(path), '-map', spec, '-c', 'copy', '-f', 'data', '-'], capture_output=True, check=True).stdout
    return hashlib.sha256(raw).hexdigest()


def k_weight(x):
    pre = ([1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585])
    rlb = ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621])
    return signal.lfilter(*rlb, signal.lfilter(*pre, x, axis=0), axis=0)


def scene_levels(x):
    """Ungated K-weighted level per scene. Short scenes, so not a BS.1770 integrated value."""
    k = k_weight(x)
    out = {}
    for a, b in zip(render.CUTS, render.CUTS[1:]):
        seg = k[int(a * SR):int(b * SR)]
        out[f'{a:g}-{b:g}'] = round(float(-0.691 + 10 * np.log10((seg ** 2).sum(1).mean() + 1e-15)), 1)
    return out


def flux(x):
    f, t, z = signal.stft(x.mean(1), SR, nperseg=512, noverlap=512 - 120, boundary='zeros')
    keep = (f >= 150) & (f <= 8000)
    mag = np.log1p(np.abs(z[keep]) * 40)
    return t[1:], np.maximum(np.diff(mag, axis=1), 0).sum(0)


def cut_response(x):
    """The audio lab's check: how far the spectrum moves across each hit, against non-cut seconds."""
    f, ft, z = signal.stft(x.mean(1), SR, nperseg=4096, noverlap=2048)
    edges = np.geomspace(60, 16000, 25)
    logb = 10 * np.log10(np.stack([(np.abs(z[(f >= a) & (f < b)]) ** 2).sum(0) for a, b in zip(edges, edges[1:])]) + 1e-10)

    def shift(s, span=0.6):
        before = logb[:, (ft >= s - span) & (ft < s - 0.04)].mean(1)
        after = logb[:, (ft >= s + 0.04) & (ft < s + span)].mean(1)
        return float(np.sqrt(((after - before) ** 2).mean()))
    hits = render.CUTS[1:-1]
    others = [s for s in range(1, 40) if all(abs(s - c) > 0.25 for c in hits)]
    rows = {f'{c:g}': round(shift(c), 2) for c in hits}
    return {'spectral_shift_db_at_hits': rows, 'median_at_hits_db': round(float(np.median(list(rows.values()))), 2),
            'median_at_noncut_seconds_db': round(float(np.median([shift(s) for s in others])), 2), 'noncut_seconds': len(others)}


def piano_onsets():
    """From the score's piano stem, before mixing: where the 5 ms level first rises 12 dB above
    the 25 ms before the hit. ms after the hit; a level rise, because a flux peak can land on a
    later partial of the same note."""
    x, _ = sf.read(SCORE / 'stems' / 'piano.wav', always_2d=True)
    m = x.mean(1)
    rms = lambda a: 20 * np.log10(np.sqrt((m[a:a + 240] ** 2).mean()) + 1e-9)
    out = {}
    for c in render.CUTS[1:-1]:
        s = int(c * SR)
        floor = 20 * np.log10(np.sqrt((m[s - 1440:s - 240] ** 2).mean()) + 1e-9)
        found = next((ms for ms in range(-5, 51) if rms(s + ms * 48) > floor + 12), None)
        out[f'{c:g}'] = found
    return out


def luma_stats():
    proc = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(MP4), '-map', '0:v:0', '-f', 'rawvideo', '-pix_fmt', 'gray', '-'], stdout=subprocess.PIPE)
    means = []
    while True:
        raw = proc.stdout.read(render.W * render.H)
        if len(raw) < render.W * render.H:
            break
        means.append(float(np.frombuffer(raw, np.uint8).mean()))
    assert proc.wait() == 0
    d = np.abs(np.diff(means))
    top = int(np.argmax(d))
    return {'decoded_frames': len(means), 'max_frame_to_frame_change_0_255': round(float(d[top]), 2), 'at_seconds': round((top + 1) / render.FPS, 3),
            'frames_over_20_levels': int((d > 20).sum()), 'peak_mean_luma': round(max(means), 1)}, means


def hard_cuts():
    log = run('ffmpeg', '-hide_banner', '-nostats', '-i', str(MP4), '-map', '0:v:0', '-vf', "select='gt(scene,0.08)',metadata=print:file=-", '-f', 'null', '-').stdout
    return [round(float(v), 3) for v in re.findall(r'pts_time:([\d.]+)', log)]


def main():
    probe = json.loads(run('ffprobe', '-v', 'error', '-show_entries', 'stream=codec_type,codec_name,profile,width,height,r_frame_rate,nb_frames,duration,sample_rate,channels,bit_rate,pix_fmt,color_space,color_primaries,color_transfer,color_range',
                           '-show_entries', 'format=duration,size,bit_rate:format_tags', '-of', 'json', str(MP4)).stdout)
    video = next(s for s in probe['streams'] if s['codec_type'] == 'video')
    audio = next(s for s in probe['streams'] if s['codec_type'] == 'audio')
    errors = subprocess.run(['ffmpeg', '-v', 'error', '-i', str(MP4), '-f', 'null', '-'], capture_output=True, text=True)
    x = decode_audio(MP4)
    wav, wav_rate = sf.read(SCORE / 'mix.wav', always_2d=True)
    n = min(len(x), len(wav))
    corr = signal.fftconvolve(x[:n].mean(1), wav[:n].mean(1)[::-1])
    mid = x.mean(1)
    meter = pyloudnorm.Meter(SR)
    limiter = json.loads((SCORE / 'metrics.json').read_text())
    luma, means = luma_stats()
    camera = json.loads((ROOT / 'qa' / 'camera.json').read_text())
    prov = render.PROVENANCE
    capture_ok = all(hashlib.sha256((ROOT / 'capture' / s['file']).read_bytes()).hexdigest() == s['sha256'] for s in prov['shots'])
    worst = max(camera['shots'].items(), key=lambda kv: kv[1]['scale_max'])
    captions = sorted({s[2] for s in render.SCENES})
    authored = captions + ['OMOI', 'OMOI / UNSAVED PREVIEW', 'APP CAPTURE', 'SYNTHETIC DATA', 'APP CAPTURE / SYNTHETIC DATA', 'Local watch history for the PC', 'Windows · development preview']
    result = {
        'file': MP4.name, 'bytes': MP4.stat().st_size, 'sha256': hashlib.sha256(MP4.read_bytes()).hexdigest(),
        'video': {k: video.get(k) for k in ('codec_name', 'profile', 'width', 'height', 'r_frame_rate', 'nb_frames', 'duration', 'pix_fmt', 'color_space', 'color_primaries', 'color_transfer', 'color_range', 'bit_rate')},
        'audio': {k: audio.get(k) for k in ('codec_name', 'sample_rate', 'channels', 'duration', 'bit_rate', 'nb_frames')},
        'format': {k: probe['format'].get(k) for k in ('duration', 'size', 'bit_rate')}, 'format_tags': probe['format'].get('tags', {}),
        'decode_return': errors.returncode, 'decode_errors': errors.stderr,
        'loudness_final_file_ffmpeg_ebur128': ebur128(MP4),
        'decoded_audio_samples': len(x), 'expected_audio_samples': 40 * SR,
        'pyloudnorm_on_decoded_audio_lufs': round(float(meter.integrated_loudness(x)), 2),
        'stereo_minus_mono_lu': round(float(meter.integrated_loudness(x) - meter.integrated_loudness(np.stack([mid, mid], 1))), 2),
        'first_5ms_peak_db': round(float(20 * np.log10(np.abs(x[:240]).max() + 1e-9)), 1), 'last_5ms_peak_db': round(float(20 * np.log10(np.abs(x[-240:]).max() + 1e-9)), 1),
        'last_500ms_level_db': round(float(20 * np.log10(np.std(x[-SR // 2:]) + 1e-9)), 1),
        'av_offset_samples_vs_mix_wav': int(np.argmax(corr) - (n - 1)), 'mix_wav_rate': wav_rate,
        'aac_in_mp4_identical_to_encoded_score': stream_hash(MP4, '0:a:0') == stream_hash(SCORE / 'score.m4a', '0:a:0'),
        'limiter': {k: limiter[k] for k in ('trim_db', 'limit_ceiling_db', 'max_gain_reduction_db', 'pct_time_limiting')},
        'scene_levels_lufs_scale': scene_levels(x),
        'cut_response': cut_response(x),
        'piano_stem_onset_ms_after_hit': piano_onsets(),
        'hard_cuts_detected_s': hard_cuts(),
        'mean_luma': luma,
        'mean_luma_at_seconds': {str(t): round(means[int(t * render.FPS)], 1) for t in (28, 28.6, 28.9, 29.2, 30.6, 31, 31.3, 34, 34.5, 34.8, 35)},
        'upscale': {'worst_shot': worst[0], 'worst_scale': worst[1]['scale_max'], 'any_upscale': worst[1]['scale_max'] > 1.0, 'per_capture': camera['shots']},
        'capture': {'shots': len(prov['shots']), 'sha256_match_provenance': capture_ok, 'catalogue_items': prov['catalogue_items'], 'browser_openings': prov['browser_openings'],
                    'tags_in_catalogue_after_capture': prov['catalogue_after_capture']['tags'], 'editor_saved': prov['editor_saved'], 'external_requests': prov['external_requests']},
    }
    (ROOT / 'qa' / 'validation.json').write_text(json.dumps(result, indent=2, default=float) + '\n', encoding='utf-8')
    (ROOT / 'qa' / 'copy-review.json').write_text(json.dumps({
        'captions': captions, 'authored_text': authored, 'periods_in_authored_text': any('.' in a for a in authored),
        'preserved_labels': ['UNSAVED PREVIEW', 'APP CAPTURE', 'SYNTHETIC DATA'], 'ending_line_source': 'README.md first line: "Local, low-overhead watch history for the PC"',
        'evidence_caveat': 'Original UI states tab time is an approximation, not measured playback; held in frame 15.5-18 s'}, indent=2, ensure_ascii=False) + '\n', encoding='utf-8')
    ls = result['loudness_final_file_ffmpeg_ebur128']
    print(MP4.name, result['bytes'], 'bytes;', video['width'], 'x', video['height'], video['r_frame_rate'], video['nb_frames'], 'frames;', audio['sample_rate'], 'Hz;',
          'v', video['duration'], 'a', audio['duration'], ';', ls, '; worst scale', worst[0], worst[1]['scale_max'], '; decode rc', errors.returncode)


if __name__ == '__main__':
    main()
