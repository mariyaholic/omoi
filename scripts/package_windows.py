"""Create and verify a portable Windows ZIP and an optional tracked-source ZIP.

Stdlib only. Refuses to overwrite archives. Includes binaries, setup instructions
and tracked synthetic evidence; never includes application data or personal receipts.
The extracted executables are checked with an isolated empty catalogue and system-only PATH.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import zipfile


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir',required=True,type=Path)
    parser.add_argument('--out',required=True,type=Path)
    parser.add_argument('--source-out',type=Path)
    args=parser.parse_args()
    assert os.name=='nt','Windows executable verification required'
    repo=Path(__file__).resolve().parents[1]
    outputs=[args.out]+([args.source_out] if args.source_out else [])
    assert len({p.resolve() for p in outputs})==len(outputs),'Archive destinations must differ'
    for output in outputs:
        assert not output.exists(),f'Refusing to overwrite {output}'
        assert output.parent.is_dir(),f'Create output parent first: {output.parent}'
    commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
    assert subprocess.run(['git','diff','--quiet','HEAD'],cwd=repo).returncode==0,'Commit intended tracked changes before packaging'
    tracked=subprocess.check_output(['git','ls-files','-z'],cwd=repo).decode().split('\0')
    assert 'docs/headless-acceptance-20261103.md' in tracked,'Track the acceptance record before packaging'
    content={}
    for name in ('omoi.exe','omoiw.exe'):
        content[name]=(args.bin_dir/name).read_bytes()
    for name in ('Run Omoi.cmd','Run Playback.cmd','README.md'):
        content[name]=(repo/'packaging'/name).read_bytes()
    content['omoi.user.js']=(repo/'userscripts/omoi.user.js').read_bytes()
    for name in tracked:
        if name.startswith('docs/') and name.endswith(('.md','.json')) or name.startswith('ui/') and name.endswith('.json') or name in ('ui/README.md','ui/MEASUREMENTS.md','userscripts/README.md'):
            content[name]=(repo/name).read_bytes()
    for name,source in {'PLAYBACK.md':'userscripts/README.md','UI.md':'ui/README.md','MEASUREMENTS.md':'ui/MEASUREMENTS.md','ACCEPTANCE.md':'docs/headless-acceptance-20261103.md'}.items():
        text=(repo/source).read_text(encoding='utf-8').replace('(../docs/','(docs/').replace('(../ui/','(ui/').replace('(../userscripts/','(userscripts/')
        content[name]=text.encode('utf-8')
    manifest={'source_commit':commit,'created_utc':datetime.now(timezone.utc).isoformat(),'files':{name:{'bytes':len(data),'sha256':sha(data)} for name,data in sorted(content.items())},'scope':'locally built unsigned Windows development package; no application data; synthetic evidence only'}
    content['BUILD.json']=json.dumps(manifest,indent=2).encode()
    with tempfile.TemporaryDirectory(prefix='omoi-package-') as directory:
        root=Path(directory)
        archive=root/'portable.zip'
        with zipfile.ZipFile(archive,'x',zipfile.ZIP_DEFLATED,compresslevel=9) as z:
            for name,data in sorted(content.items()):
                z.writestr(name,data)
        extracted=root/'extracted'
        with zipfile.ZipFile(archive) as z:
            assert z.testzip() is None
            assert set(z.namelist())==set(content)
            z.extractall(extracted)
        for name,data in content.items():
            assert sha((extracted/name).read_bytes())==sha(data),name
        home=root/'empty-catalogue'
        home.mkdir()
        (home/'config.toml').write_text('browsers=[]\n',encoding='utf-8')
        env=os.environ|{'OMOI_HOME':str(home),'PATH':os.pathsep.join([str(Path(os.environ['SystemRoot'])/'System32'),os.environ['SystemRoot']])}
        for key in ('LOCALAPPDATA','APPDATA','HOME','XDG_CONFIG_HOME','XDG_DATA_HOME'):
            env[key]=str(root/'empty-browser-roots')
        for command in (['--help'],['db','migrate'],['db','info']):
            run=subprocess.run([str(extracted/'omoi.exe'),*command],env=env,cwd=extracted,capture_output=True,text=True,timeout=30,creationflags=subprocess.CREATE_NO_WINDOW)
            assert run.returncode==0,run.stderr
            if command==['db','info']:
                assert json.loads(run.stdout)['counts']['items']==0
        # Publish only after all content and portability checks pass.
        with args.out.open('xb') as target:
            target.write(archive.read_bytes())
    if args.source_out:
        subprocess.run(['git','archive','--format=zip','--output',str(args.source_out.resolve()),'HEAD'],cwd=repo,check=True)
        with zipfile.ZipFile(args.source_out) as z:
            assert z.testzip() is None
            assert {'Cargo.toml','src/lib.rs','docs/headless-acceptance-20261103.md'}.issubset(z.namelist())
            assert not any(name.endswith(('.sqlite','.exe')) or '.git/' in name for name in z.namelist())
    print(json.dumps({'result':'pass','source_commit':commit,'portable_files':len(content),'archives':[{'path':str(p),'bytes':p.stat().st_size,'sha256':sha(p.read_bytes())} for p in outputs],'checks':['archive integrity','all file hashes','isolated extracted CLI','system-only executable PATH','empty catalogue','tracked source only']},indent=2))


if __name__=='__main__':
    main()
