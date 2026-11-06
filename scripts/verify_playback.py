"""Verify the real listener and session pipeline with isolated synthetic telemetry."""
import argparse
import ctypes
from contextlib import closing
import http.client
import hashlib
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
    parser.add_argument('--duration',type=int,default=0,help='Real elapsed seconds of synthetic telemetry at five-second intervals')
    parser.add_argument('--stress-inbox',action='store_true',help='Exercise a near-full 20 MB synthetic inbox')
    args = parser.parse_args()
    if (args.duration or args.stress_inbox) and os.name!='nt':
        parser.error('Resource acceptance currently requires Windows process counters')
    if not 0<=args.duration<=7200:
        parser.error('Duration must be 0..7200 seconds')
    binary = str(args.binary.resolve())
    binary_sha256=hashlib.sha256(args.binary.read_bytes()).hexdigest()
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
                try:
                    conn.request(method, path, json.dumps(event), {'Content-Type': 'application/json', 'X-Omoi-Token': token} | (extra or {}))
                    result = conn.getresponse()
                    status = result.status
                    result.read()
                    return status
                finally:
                    conn.close()

            first = dict(event_id='test-0', producer_id='test', seq=0, url='https://www.youtube.com/watch?v=abcdefghijk', at_ms=1700000000000,
                         state='playing', position_ms=0, cumulative_played_ms=0, title='Capture fixture', channel='Fixture', duration_s=600)
            assert post(first, {'X-Omoi-Token': ''}) == 403
            for origin in ('null', 'https://www.youtube.com', 'chrome-extension://fake'):
                assert post(first, {'Origin': origin}) == 403
            assert post(first, {'Host': 'evil.test'}) == 403
            assert post(first, method='OPTIONS') == 403
            assert post(first, method='GET', path='/api/history') == 403
            invalid_status = post({})
            with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                conn.execute("CREATE TRIGGER reject_fixture BEFORE INSERT ON raw_events BEGIN SELECT RAISE(ABORT,'fixture storage failure'); END")
                conn.commit()
            storage_status = post(first)
            with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                conn.execute('DROP TRIGGER reject_fixture')
                conn.commit()
                assert conn.execute('SELECT count(*) FROM playback_receipts').fetchone() == (0,)
            assert (invalid_status, storage_status) == (400, 503), (invalid_status, storage_status)
            try:
                assert post(first | {'title': 'x' * 17000}) == 400
            except (ConnectionResetError,ConnectionAbortedError,BrokenPipeError,http.client.RemoteDisconnected):
                pass  # Rejecting before consuming an oversized body can abort/reset the socket.
            with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                assert conn.execute('SELECT count(*) FROM raw_events').fetchone()==(0,)
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
            # A regressing stop must not change the live identity before the
            # sessionizer rejects it. Exercise real HTTP ingestion, independent
            # collector processes, persisted state and the local-day boundary.
            midnight = 1700064000000  # 2023-11-16 00:00 at default UTC+08.
            for batched in (False, True):
                producer = f'twitch-regression-{int(batched)}'
                for seq, offset, counter, state in [(0, -30000, 0, 'playing'), (1, -10000, 20000, 'playing'), (2, -5000, 10000, 'stopped'), (3, 10000, 40000, 'playing')]:
                    sample = first | dict(event_id=f'{producer}-{seq}', producer_id=producer, seq=seq,
                                          url=f'https://www.twitch.tv/fixture_{int(batched)}', at_ms=midnight+offset,
                                          position_ms=counter, cumulative_played_ms=counter, state=state)
                    assert post(sample) == 200
                    if not batched:
                        completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
                        assert completed.returncode == 0, completed.stderr
                completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
                assert completed.returncode == 0, completed.stderr
                with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                    rows = conn.execute("SELECT i.external_id,s.played_s FROM sessions s JOIN items i ON i.id=s.item_id WHERE i.platform='twitch' AND i.external_id LIKE ?", (f'live:fixture_{int(batched)}:%',)).fetchall()
                    assert len(rows) == 1 and rows[0][1] == 40, ('regressing stop split Twitch session', batched, rows)
            # Exercise deletion after enqueue but before processing at midnight.
            first_live = first | dict(event_id='delete-live-0', producer_id='delete-live', seq=0,
                                      url='https://www.twitch.tv/delete_fixture', at_ms=midnight-1000)
            assert post(first_live) == 200
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            # A long pre-deletion backlog must keep its tombstoned continuity,
            # not expire after five minutes merely because suppressed samples
            # fail to update the durable control state.
            for seq in range(1, 9):
                assert post(first_live | dict(event_id=f'delete-live-{seq}', seq=seq, at_ms=midnight-1000+120000*seq, position_ms=120000*seq, cumulative_played_ms=120000*seq)) == 200
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                conn.execute('PRAGMA foreign_keys=ON')
                item = conn.execute("SELECT id,external_id FROM items WHERE platform='twitch' AND external_id LIKE 'live:delete_fixture:%'").fetchone()
                conn.execute("INSERT INTO deleted_items(platform,external_id,through) VALUES('twitch',?,?)", (item[1], int(time.time())))
                conn.execute('DELETE FROM items WHERE id=?', (item[0],))
                conn.commit()
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                assert conn.execute("SELECT count(*) FROM items WHERE external_id LIKE 'live:delete_fixture:%'").fetchone() == (0,)
                assert conn.execute("SELECT seq FROM playback_frontier WHERE producer='delete-live'").fetchone() == (8,)

            # Old raw bodies did not record their timezone. Refuse ambiguous
            # dates without modifying bytes, then use explicit owner migration.
            legacy = first | dict(event_id='legacy-0', producer_id='legacy', seq=0,
                                  url='https://www.twitch.tv/legacy_fixture', at_ms=midnight-2000,
                                  cumulative_played_ms=1000, state='stopped')
            assert post(legacy) == 200
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                identity = conn.execute("SELECT external_id FROM items WHERE external_id LIKE 'live:legacy_fixture:%'").fetchone()[0]
                for seq, offset, counter in ((1, -1000, 900), (2, 1000, 3000)):
                    event = legacy | dict(event_id=f'legacy-{seq}', seq=seq, at_ms=midnight+offset, cumulative_played_ms=counter, state='playing')
                    body = json.dumps(dict(event=event, key=dict(platform='twitch', external_id=identity)))
                    conn.execute("INSERT INTO raw_events(at,source,body) VALUES(?,'userscript',?)", (event['at_ms'], body))
                    conn.execute('INSERT INTO playback_receipts VALUES(?,?,?)', (event['event_id'], 'legacy', seq))
                conn.commit()
                before = conn.execute('SELECT id,body FROM raw_events ORDER BY id').fetchall()
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode != 0 and 'Legacy Twitch queue' in completed.stderr, completed.stderr
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                assert conn.execute('SELECT id,body FROM raw_events ORDER BY id').fetchall() == before
            completed = subprocess.run([binary, 'db', 'migrate-playback', '--utc-offset', '+08:00'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            preview = json.loads(completed.stdout)
            assert {row['event_id'] for row in preview['rows']} == {'legacy-1', 'legacy-2'}
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                assert conn.execute('SELECT id,body FROM raw_events ORDER BY id').fetchall() == before
            completed = subprocess.run([binary, 'db', 'migrate-playback', '--utc-offset', '+08:00', '--apply'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode != 0
            completed = subprocess.run([binary, 'db', 'migrate-playback', '--utc-offset', '+08:00', '--event-id', 'legacy-1', '--event-id', 'legacy-2', '--apply'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            assert json.loads(completed.stdout)['migrated_legacy_playback_events'] == 2
            completed = subprocess.run([binary, 'collect'], env=env, capture_output=True, text=True, timeout=30)
            assert completed.returncode == 0, completed.stderr
            with closing(sqlite3.connect(root / 'history.sqlite')) as conn:
                assert conn.execute("SELECT count(*) FROM items WHERE external_id LIKE 'live:legacy_fixture:%'").fetchone() == (2,)

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
                if args.stress_inbox:
                    large=first|dict(event_id='stress-0',producer_id='stress',title='x'*4000,channel='x'*1000,url=first['url']+'&padding='+'x'*7000)
                    assert post(large)==200
                    with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                        row=conn.execute('SELECT at,source,body FROM raw_events LIMIT 1').fetchone()
                        conn.executemany('INSERT INTO raw_events(at,source,body) VALUES(?,?,?)',[row]*1700)
                        conn.commit()
                    assert post(large|dict(event_id='stress-1',seq=1))==200
                    assert psapi.GetProcessMemoryInfo(int(proc._handle),ctypes.byref(memory),memory.cb)
                    with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                        size=conn.execute('SELECT sum(length(CAST(body AS BLOB))) FROM raw_events').fetchone()[0]
                        assert size<=20000000
                    resources['full_inbox']={'body_bytes':size,'working_set_bytes':memory.working,'lifetime_peak_bytes':memory.peak_working}
                    completed=subprocess.run([binary,'collect'],env=env,capture_output=True,text=True,timeout=60)
                    assert completed.returncode==0,completed.stderr
                if args.duration:
                    assert 1<=args.duration<=7200
                    start_time=time.monotonic()
                    start_cpu=cpu()
                    seq=0
                    wall_base=int(time.time()*1000)
                    print('Starting real-time telemetry load',flush=True)
                    while True:
                        elapsed=min(args.duration,time.monotonic()-start_time)
                        sample=first|dict(event_id=f'load-{seq}',producer_id='load',seq=seq,at_ms=wall_base+int(elapsed*1000),position_ms=int(elapsed*1000),cumulative_played_ms=int(elapsed*1000))
                        assert post(sample)==200
                        seq+=1
                        if elapsed>=args.duration:
                            break
                        if seq%60==0:
                            print(json.dumps({'elapsed_seconds':round(elapsed),'events':seq}),flush=True)
                        time.sleep(min(5,args.duration-elapsed))
                    assert psapi.GetProcessMemoryInfo(int(proc._handle),ctypes.byref(memory),memory.cb)
                    resources['sustained']={'elapsed_seconds':time.monotonic()-start_time,'events':seq,'listener_cpu_seconds':(cpu()-start_cpu)/1000,'working_set_bytes':memory.working,'lifetime_peak_bytes':memory.peak_working,'scope':'real-time synthetic HTTP playback telemetry; excludes browser/decoder and collector'}
                    proc.terminate()
                    proc.wait(timeout=10)
                    completed=subprocess.run([binary,'collect'],env=env,capture_output=True,text=True,timeout=60)
                    assert completed.returncode==0,completed.stderr
                    with closing(sqlite3.connect(root/'history.sqlite')) as conn:
                        played_ms=conn.execute("SELECT played_ms FROM playback_state WHERE producer='load'").fetchone()[0]
                        assert abs(played_ms-args.duration*1000)<1000,(played_ms,args.duration)
                        resources['sustained']['processed_played_ms']=played_ms
                        resources['catalogue_bytes_before_checkpoint']=sum(p.stat().st_size for p in root.glob('history.sqlite*'))
                        conn.execute('PRAGMA wal_checkpoint(TRUNCATE)')
                        page_size=conn.execute('PRAGMA page_size').fetchone()[0]
                        resources['page_bytes']=conn.execute('PRAGMA page_count').fetchone()[0]*page_size
                        resources['freelist_bytes']=conn.execute('PRAGMA freelist_count').fetchone()[0]*page_size
                    resources['processed_catalogue_bytes']=sum(p.stat().st_size for p in root.glob('history.sqlite*'))
        finally:
            proc.terminate()
            proc.wait(timeout=10)
    budgets={}
    if resources:
        budgets['listener_peak_at_most_10mb']=max(resources['peak_working_set_bytes'],resources.get('full_inbox',{}).get('lifetime_peak_bytes',0),resources.get('sustained',{}).get('lifetime_peak_bytes',0))<=10000000
    checks=['listener auth','hostile requests','restart','duplicate replay','collection','metadata','port collision','Twitch regressing stop across midnight, immediate and batched processing','pending Twitch deletion after midnight','explicit legacy queue timezone migration']
    if args.stress_inbox:
        checks.append('20 MB inbox eviction cap')
    if args.duration:
        checks.append('real-time event load and processed playback duration')
    receipt={'result':'pass' if all(budgets.values()) else 'failed','binary_sha256':binary_sha256,'functional_result':'pass','resource_budgets':budgets,'checks':checks,'resources':resources}
    if args.out:
        args.out.write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    print(json.dumps(receipt,indent=2))
    if not all(budgets.values()):
        raise SystemExit('Resource acceptance failed; see receipt')


if __name__ == '__main__':
    main()
