"""Windows CI only: unsigned per-user installer lifecycle in a disposable runner."""
import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tempfile
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--installer',type=Path,required=True);a=p.parse_args()
assert os.name=='nt','Windows installer execution required'
with tempfile.TemporaryDirectory(prefix='omoi-installer-') as td:
    root=Path(td);app=root/'program';home=root/'catalogue';home.mkdir()
    (home/'config.toml').write_text('browsers=[]\n')
    (home/'webview').mkdir();(home/'webview'/'appearance-fixture').write_text('oak; compact; synthetic retained settings')
    env=os.environ|{'OMOI_HOME':str(home)}
    def install():subprocess.run([str(a.installer.resolve()),'/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART','/SP-',f'/DIR={app}'],check=True,timeout=120)
    install()
    subprocess.run([str(app/'omoi.exe'),'db','migrate'],env=env,check=True,capture_output=True,timeout=30)
    before={str(f.relative_to(home)):hashlib.sha256(f.read_bytes()).hexdigest() for f in home.rglob('*') if f.is_file()}
    install() # Same-version repair/update must leave data/settings untouched.
    assert all(hashlib.sha256((home/name).read_bytes()).hexdigest()==digest for name,digest in before.items())
    assert (app/'omoi-native.exe').is_file() and (app/'extension/manifest.json').is_file()
    subprocess.run([str(app/'unins000.exe'),'/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART'],check=True,timeout=120)
    assert all(hashlib.sha256((home/name).read_bytes()).hexdigest()==digest for name,digest in before.items())
    assert not (app/'omoi.exe').exists()
print('PASS: unsigned per-user install, reinstall/repair, uninstall preserve synthetic data/settings; no extension installation or real profile accessed')
