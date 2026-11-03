"""Exercise the shipped scheduler script against a fake COM service; no tasks change."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'src/scheduler/windows.ps1'
WRAPPER = r'''
param($Target,$Scenario,$Mode)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class FixtureCollection { public int Count; public FixtureCollection(int n){Count=n;} }
public class FixtureFolder {
    string scenario; public FixtureFolder(string s){scenario=s;}
    public void DeleteFolder(string name,int flags){throw new Exception("hidden sibling would be deleted");}
    public void DeleteTask(string name,int flags){}
    public FixtureCollection GetTasks(int flags){return new FixtureCollection(flags==1?1:0);}
    public FixtureCollection GetFolders(int flags){return new FixtureCollection(0);}
    public object GetTask(string name){if(scenario=="missing-task")throw new COMException("missing",-2147024894);return new object();}
}
public class FixtureService {
    string scenario; public FixtureService(string s){scenario=s;}
    public void Connect(){}
    public FixtureFolder GetFolder(string name){if(name!="\\" && scenario=="missing-folder")throw new COMException("missing",-2147024894);return new FixtureFolder(scenario);}
}
'@
$global:omoiFixtureService=[FixtureService]::new($Scenario)
function New-Object { param($ComObject) if($ComObject -ne 'Schedule.Service'){throw 'Unexpected COM service'};return $global:omoiFixtureService }
& $Target -Mode $Mode -FolderName 'OmoiVerify-State'
exit $LASTEXITCODE
'''


@unittest.skipUnless(os.name == 'nt', 'Windows PowerShell script')
class SchedulerTransitions(unittest.TestCase):
    def run_case(self, scenario, mode):
        with tempfile.TemporaryDirectory(prefix='omoi-scheduler-state-') as temporary:
            wrapper = Path(temporary) / 'fixture.ps1'
            wrapper.write_text(WRAPPER, encoding='utf-8')
            engine = Path(os.environ['SystemRoot']) / 'System32/WindowsPowerShell/v1.0/powershell.exe'
            return subprocess.run([str(engine), '-NoProfile', '-NonInteractive', '-ExecutionPolicy',
                                   'Bypass', '-File', str(wrapper), str(SCRIPT), scenario, mode],
                                  capture_output=True, text=True, timeout=15,
                                  creationflags=subprocess.CREATE_NO_WINDOW)

    def test_run_requires_existing_folder_and_task(self):
        for scenario in ('missing-folder', 'missing-task'):
            with self.subTest(scenario=scenario):
                result = self.run_case(scenario, 'run')
                self.assertNotEqual(result.returncode, 0, result.stdout)

    def test_remove_preserves_hidden_siblings(self):
        for scenario in ('present', 'missing-task'):
            with self.subTest(scenario=scenario):
                result = self.run_case(scenario, 'remove')
                self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == '__main__':
    unittest.main()
