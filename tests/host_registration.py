"""Windows-only registration lifecycle in an isolated HKCU test namespace."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
import uuid

@unittest.skipUnless(os.name=='nt','Windows PowerShell and registry required')
class Registration(unittest.TestCase):
    def test_install_repair_remove_and_foreign_owner(self):
        import winreg
        repo=Path(__file__).resolve().parents[1]
        key='Software\\OmoiTests\\'+uuid.uuid4().hex
        root='HKCU:\\'+key
        with tempfile.TemporaryDirectory(prefix='omoi-registration-') as td:
            base=Path(td);home=base/'data';home.mkdir();(home/'keep').write_text('synthetic settings')
            def package(name):
                target=base/name;target.mkdir();shutil.copy2(repo/'packaging/Register-OmoiHost.ps1',target);(target/'omoi-native.exe').write_bytes(b'fixture-only');return target
            first=package('first');second=package('moved')
            def run(folder,action='Install',identifier='a'*32,ok=True):
                result=subprocess.run(['powershell.exe','-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',str(folder/'Register-OmoiHost.ps1'),'-Action',action,'-ExtensionId',identifier,'-CatalogueHome',str(home),'-RegistryRoot',root],capture_output=True,text=True,timeout=30)
                self.assertEqual(result.returncode==0,ok,result.stderr)
            try:
                run(first);run(first)
                manifest=json.loads((first/'org.omoi.capture_dev.json').read_text(encoding='utf-8-sig'))
                self.assertEqual(manifest['allowed_origins'],['chrome-extension://'+'a'*32+'/'])
                run(first,identifier='*',ok=False)
                run(second) # Same catalogue and readable ownership marker permits path repair.
                run(first,'Remove',ok=False) # Old installation must not remove moved registration.
                run(second,'Remove');run(second,'Remove')
                self.assertEqual((home/'keep').read_text(),'synthetic settings')
            finally:
                try:winreg.DeleteKey(winreg.HKEY_CURRENT_USER,key+'\\org.omoi.capture_dev')
                except FileNotFoundError:pass
                try:winreg.DeleteKey(winreg.HKEY_CURRENT_USER,key)
                except FileNotFoundError:pass

if __name__=='__main__':unittest.main()
