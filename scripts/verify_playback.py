"""Verify the real listener and session pipeline with isolated synthetic telemetry."""
import argparse
import ctypes
from contextlib import closing
import http.client
import json
import os
from pathlib import Path
import queue
import socket
import sqlite3
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--out', type=Path, help='Optional JSON receipt path')
    args = parser.parse_args()
    binary = str(args.binary.resolve())
    resources = {}
    with tempfile.TemporaryDirectory(prefix='omoi-playback-') as directory:
        root = Path(directory)
        with closing(socket.socket()) as probe:
            probe.bind(('127.0.0.1', 0))
            port = probe.getsockname()[1]
        (root / 'config.toml').write_text(f'listener_port={port}\nbrowsers=[]\n', encoding='utf-8')
        env = os.environ | {'OMOI_HOME': str(root)}
        (root / 'empty').mkdir()
        for key in ('LOCALAPPDATA', 'APPDATA', 'HOME', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME'):
            env[key] = str(root / 'empty')

        def start():
            p = subprocess.Popen([binary, 'listen'], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                 creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
            lines = queue.Queue()
            threading.Thread(target=lambda: lines.put(p.stdout.readline()), daemon=True).start()
            try:
                line=lines.get(timeout=30)
                if not line.startswith('Playback listener:'):
                    p.wait(timeout=5)
                    detail=p.stderr.read()
                    raise AssertionError('Listener startup failed: '+detail)
            except BaseException:
                p.kill()
                p.wait()
                raise
            return p

        proc = start()
        try:
            token = (root / 'listener' / 'token').read_text()

            def post(event, extra=None, method='POST', path='/event'):
                conn = http.client.HTTPConnection('127.0.0.1', port, timeout=10)
                conn.request(method, path, json.dumps(event), {'Content-Type': 'application/json', 'X-Omoi-Token': token} | (extra or {}))
                result = conn.getresponse()
                status = result.status
                result.read()
                conn.close()
                return status

            first = dict(event_id='test-0', producer_id='test', seq=0, url='https://www.youtube.com/watch?v=abcdefghijk', at_ms=1700000000000,
                         state='playing', position_ms=0, cumulative_played_ms=0, title='Capture fixture', channel='Fixture', duration_s=600)
            assert post(first, {'X-Omoi-Token': ''}) == 403
            for origin in ('null', 'https://www.youtube.com', 'chrome-extension://fake'):
                assert post(first, {'Origin': origin}) == 403
            assert post(first, {'Host': 'evil.test'}) == 403
            assert post(first, method='OPTIONS') == 403
            assert post(first, method='GET', path='/api/history') == 403
            try:
                assert post(first | {'title': 'x' * 17000}) == 403
            except ConnectionResetError:
                pass  # Windows may reset a rejected socket with unread oversized input.
            assert post(first) == 200
            assert post(first) == 200
            proc.terminate()
            proc.wait(timeout=10)
            proc = start()
            assert (root / 'listener' / 'token').read_text() == token
            assert post(first | dict(event_id='test-1', seq=1, at_ms=1700000060000, position_ms=60000, cumulative_played_ms=60000, state='paused')) == 200
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                assert conn.execute('SELECT played_s FROM sessions').fetchall() == [(60,)]
                assert conn.execute('SELECT count(*) FROM raw_events').fetchone() == (0,)
                assert conn.execute('SELECT channel,duration_s FROM items').fetchone() == ('Fixture', 600)
            collision = subprocess.run([binary, 'listen'], env=env, capture_output=True, timeout=15)
            assert collision.returncode != 0
            if os.name == 'nt':
                from ctypes import wintypes
                from measure import Memory
                kernel=ctypes.WinDLL('kernel32',use_last_error=True)
                psapi=ctypes.WinDLL('psapi',use_last_error=True)
                kernel.GetProcessTimes.argtypes=[wintypes.HANDLE]+[ctypes.POINTER(wintypes.FILETIME)]*4
                psapi.GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(Memory),wintypes.DWORD]

                def cpu():
                    times=[wintypes.FILETIME() for _ in range(4)]
                    assert kernel.GetProcessTimes(int(proc._handle),*(ctypes.byref(t) for t in times))
                    return sum((t.dwHighDateTime<<32)|t.dwLowDateTime for t in times[2:])/10000

                before=cpu()
                time.sleep(3)
                memory=Memory()
                memory.cb=ctypes.sizeof(memory)
                assert psapi.GetProcessMemoryInfo(int(proc._handle),ctypes.byref(memory),memory.cb)
                resources={'working_set_bytes':memory.working,'peak_working_set_bytes':memory.peak_working,'idle_cpu_ms_over_3s':cpu()-before,'scope':'restarted listener, two-event fixture; excludes browser and ACL helper; not a sustained-playback budget test'}
        finally:
            proc.terminate()
            proc.wait(timeout=10)
    receipt={'result':'pass','checks':['listener auth','hostile requests','restart','duplicate replay','collection','metadata','port collision'],'resources':resources}
    if args.out:
        args.out.write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    print(json.dumps(receipt,indent=2))


if __name__ == '__main__':
    main()
