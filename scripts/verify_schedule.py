"""Install/run/remove a uniquely named synthetic Windows task; never collect real profiles."""
import argparse
import json
import os
from pathlib import Path
import sqlite3
from contextlib import closing
import subprocess
import tempfile
import time
import uuid


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', required=True, type=Path)
    parser.add_argument('--out', type=Path, help='Optional JSON receipt path')
    args = parser.parse_args()
    assert os.name == 'nt', 'Windows is required'
    folder = 'OmoiVerify-' + uuid.uuid4().hex
    with tempfile.TemporaryDirectory(prefix='omoi-schedule-') as directory:
        root = Path(directory)
        (root / 'config.toml').write_text('browsers=[]\n', encoding='utf-8')
        env = os.environ | {'OMOI_HOME': str(root)}

        def run(mode):
            r = subprocess.run([str(args.binary.resolve()), 'schedule', '--task-folder', folder, mode],
                               env=env, capture_output=True, text=True, timeout=45, creationflags=subprocess.CREATE_NO_WINDOW)
            assert r.returncode == 0, f'{mode}: {r.stderr}'
            return r.stdout

        try:
            assert 'not installed' in run('status')
            xml = run('install')
            assert 'omoiw.exe' in xml and 'PT1H' in xml and '<Hidden>true</Hidden>' in xml
            assert str(root) in xml
            run('run')
            deadline = time.monotonic() + 30
            while time.monotonic() < deadline:
                if (root / 'history.sqlite').exists() and list((root / 'logs').glob('*')):
                    break
                time.sleep(0.25)
            else:
                raise AssertionError('Scheduled helper did not produce a catalogue and log')
            with closing(sqlite3.connect(root / 'history.sqlite')) as c:
                assert c.execute('SELECT count(*) FROM items').fetchone() == (0,)
            status = run('status')
            assert 'Last result: 0' in status, status
        finally:
            run('remove')
        assert 'not installed' in run('status')
    receipt={'result': 'pass', 'task': folder, 'checks': ['isolated install', 'real helper launch', 'zero personal rows', 'last result zero', 'remove and absence']}
    if args.out:
        args.out.write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
