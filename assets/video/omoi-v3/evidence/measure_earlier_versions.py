"""Measure the earlier films' final files the same way as v3, for LEARNINGS.md. Read-only.

Usage, from work/: python evidence/measure_earlier_versions.py
"""
from pathlib import Path
import json
import re
import subprocess

import numpy as np

HERE = Path(__file__).resolve().parent
FILM = HERE.parent.parent.parent
FILES = {'v1-production': FILM / 'production' / 'Omoi-Film' / 'Omoi — Back in reach.mp4',
         'v1-rev1': FILM / 'rev1' / 'Omoi-Film-Revision-1' / 'Omoi — A way back.mp4',
         'v2': FILM / 'pr7' / 'assets' / 'video' / 'omoi-v2' / 'omoi-v2.mp4',
         'v3': HERE.parent / 'omoi-v3' / 'omoi-v3.mp4'}


def run(*args):
    return subprocess.run(args, capture_output=True, text=True, check=True)


def luma_jump(path):
    info = json.loads(run('ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries', 'stream=width,height', '-of', 'json', str(path)).stdout)['streams'][0]
    size = info['width'] * info['height']
    proc = subprocess.Popen(['ffmpeg', '-v', 'error', '-i', str(path), '-map', '0:v:0', '-f', 'rawvideo', '-pix_fmt', 'gray', '-'], stdout=subprocess.PIPE)
    means = []
    while True:
        raw = proc.stdout.read(size)
        if len(raw) < size:
            break
        means.append(float(np.frombuffer(raw, np.uint8).mean()))
    proc.wait()
    return round(float(np.abs(np.diff(means)).max()), 2)


out = {}
for name, path in FILES.items():
    probe = json.loads(run('ffprobe', '-v', 'error', '-show_entries', 'stream=codec_type,sample_rate,duration,width,height', '-show_entries', 'format=duration,size', '-of', 'json', str(path)).stdout)
    audio = next(s for s in probe['streams'] if s['codec_type'] == 'audio')
    video = next(s for s in probe['streams'] if s['codec_type'] == 'video')
    tail = run('ffmpeg', '-hide_banner', '-nostats', '-i', str(path), '-map', '0:a:0', '-af', 'ebur128=peak=true', '-f', 'null', '-').stderr
    tail = tail[tail.rindex('Summary:'):]
    out[name] = {'bytes': int(probe['format']['size']), 'format_duration_s': float(probe['format']['duration']), 'video_duration_s': float(video['duration']),
                 'audio_duration_s': float(audio['duration']), 'audio_sample_rate': int(audio['sample_rate']),
                 'integrated_lufs': float(re.search(r'I:\s+(-?[\d.]+) LUFS', tail)[1]), 'lra_lu': float(re.search(r'LRA:\s+(-?[\d.]+) LU', tail)[1]),
                 'true_peak_dbtp': float(re.search(r'Peak:\s+(-?[\d.]+) dBFS', tail)[1]), 'max_frame_to_frame_luma_change': luma_jump(path)}
(HERE / 'earlier-versions.json').write_text(json.dumps(out, indent=2) + '\n', encoding='utf-8')
print(json.dumps(out, indent=1))
