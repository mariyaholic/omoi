"""Stage the GNU WebView2 loader and verify Omoi's hidden desktop window.

Stdlib only. Uses a disposable empty catalogue and browser profile. Does not open
a visible window, read personal history or configure normal collection.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def remove_closed_profile(profile):
    # WebView2 children release their profile asynchronously after controller close.
    assert profile.resolve().is_relative_to(Path(tempfile.gettempdir()).resolve())
    for _ in range(300):
        try:
            shutil.rmtree(profile)
            return
        except FileNotFoundError:
            # A WebView2 child may delete a nested file during rmtree. Only
            # disappearance of the profile root means cleanup has completed.
            if not profile.exists():
                return
            time.sleep(0.1)
        except OSError as error:
            if error.winerror not in (5,32,145):
                raise
            time.sleep(0.1)
    raise AssertionError('WebView2 profile remained locked after app exit')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',type=Path,required=True)
    parser.add_argument('--stage-only',action='store_true',help='Copy the x64 GNU loader from Cargo build output without running the app')
    args=parser.parse_args()
    assert os.name=='nt','Windows desktop verification only'
    binaries=args.bin_dir.resolve()
    candidates=list((binaries/'build').glob('webview2-com-sys-*/out/x64/WebView2Loader.dll'))
    assert candidates,'Build the Windows binaries first'
    hashes={hashlib.sha256(p.read_bytes()).hexdigest() for p in candidates}
    assert len(hashes)==1,'Multiple loader versions: use a clean Cargo target'
    shutil.copyfile(candidates[0],binaries/'WebView2Loader.dll')
    if args.stage_only:
        print('PASS: WebView2 loader staged')
        return
    with tempfile.TemporaryDirectory(prefix='omoi-desktop-') as directory:
        root=Path(directory)
        home=root/'catalogue'
        home.mkdir()
        (home/'config.toml').write_text('browsers=[]\n',encoding='utf-8')
        env=os.environ|{'OMOI_HOME':str(home)}
        for key in ('LOCALAPPDATA','APPDATA','HOME','XDG_CONFIG_HOME','XDG_DATA_HOME'):
            env[key]=str(root/'empty-browser-roots')
        run=subprocess.run([str(binaries/'omoi-desktop.exe'),'--check'],env=env,capture_output=True,text=True,timeout=45,creationflags=subprocess.CREATE_NO_WINDOW)
        remove_closed_profile(home/'webview')
        assert run.returncode==0,f'Desktop check exited {run.returncode}: {run.stderr}'
    print('PASS: hidden native window, authenticated catalogue, five themes, appearance/reload, clean exit')


if __name__=='__main__':
    main()
