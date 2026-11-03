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
import xml.etree.ElementTree as ET


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
            registered=run('status')
            tree=ET.fromstring(registered[registered.index('<Task'):registered.index('</Task>')+len('</Task>')])
            ns={'t':'http://schemas.microsoft.com/windows/2004/02/mit/task'}
            settings={name:tree.findtext('t:Settings/t:'+name,namespaces=ns) for name in ('DisallowStartIfOnBatteries','StopIfGoingOnBatteries','WakeToRun','MultipleInstancesPolicy','Hidden')}
            # The service omits default-valued XML elements. Inspect the COM value.
            inspect=root/'inspect-task.ps1'
            inspect.write_text("param([string]$Folder)\n$ErrorActionPreference='Stop'\n$s=New-Object -ComObject Schedule.Service\n$s.Connect()\n$s.GetFolder('\\'+$Folder).GetTask('Collect').Definition.Settings.WakeToRun\n",encoding='utf-8')
            wake=subprocess.run([str(Path(os.environ['SystemRoot'])/'System32/WindowsPowerShell/v1.0/powershell.exe'),'-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',str(inspect),folder],capture_output=True,text=True,timeout=15,creationflags=subprocess.CREATE_NO_WINDOW)
            assert wake.returncode==0,wake.stderr
            settings['WakeToRun']=wake.stdout.strip().lower()
            assert settings=={'DisallowStartIfOnBatteries':'true','StopIfGoingOnBatteries':'true','WakeToRun':'false','MultipleInstancesPolicy':'IgnoreNew','Hidden':'true'},settings
            assert tree.find('t:Triggers/t:LogonTrigger',ns) is not None
            assert tree.findtext('t:Triggers/t:TimeTrigger/t:Repetition/t:Interval',namespaces=ns)=='PT1H'
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
    receipt={'result': 'pass', 'task': folder, 'registered_settings':settings,'checks': ['isolated install', 'registered hourly/logon triggers and battery/overlap/wake settings', 'real helper launch', 'zero personal rows', 'last result zero', 'remove and absence'],'not_tested':['actual logon transition','actual AC-to-battery transition','visual observation of hidden launch']}
    if args.out:
        args.out.write_text(json.dumps(receipt,indent=2),encoding='utf-8')
    print(json.dumps(receipt))


if __name__ == '__main__':
    main()
