"""Actual-binary synthetic setup/HTTP/native journey; no real browser data. Stdlib only."""
import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import sqlite3
import struct
import subprocess
import tempfile
import time
import urllib.request
import urllib.error


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--bin-dir',type=Path,required=True);a=p.parse_args()
    suffix='.exe' if os.name=='nt' else ''
    binary=(a.bin_dir/('omoi'+suffix)).resolve();native=(a.bin_dir/('omoi-native'+suffix)).resolve()
    processes=[]
    with tempfile.TemporaryDirectory(prefix='omoi-setup-') as td:
        root=Path(td);home=root/'catalogue';home.mkdir();empty=root/'empty';empty.mkdir()
        with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
        (home/'config.toml').write_text(f'browsers=[]\nlistener_port={port}\nbusy_timeout_ms=100\n')
        env=os.environ|{k:str(empty) for k in ['LOCALAPPDATA','APPDATA','HOME','XDG_DATA_HOME','XDG_CONFIG_HOME']};env['OMOI_HOME']=str(home)
        def start_ui():
            proc=subprocess.Popen([str(binary),'ui','--port','0','--idle-seconds','60'],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True);processes.append(proc)
            launch=proc.stdout.readline().strip();assert '#token=' in launch,'UI failed to launch'
            return proc,*launch.split('/#token=')
        ui,base,credential=start_ui()
        def api(path,body=None):
            req=urllib.request.Request(base+'/api/'+path,data=None if body is None else json.dumps(body).encode(),headers={'X-Omoi-Token':credential,'Origin':base,'Content-Type':'application/json'})
            with urllib.request.urlopen(req,timeout=10) as r:return json.load(r)
        def change(**fields):return api('setup',{'generation':api('setup')['settings']['generation'],**fields})
        def event(n,producer='fixture'):
            return dict(event_id=f'{producer}:{n}',producer_id=producer,seq=n,url='https://www.youtube.com/watch?v=abcdefghijk',at_ms=1700000000000+n*1000,state='playing',position_ms=n*1000,cumulative_played_ms=n*1000,title='Synthetic setup fixture',channel=None,duration_s=600)
        def wait_for(check):
            deadline=time.monotonic()+5
            while time.monotonic()<deadline:
                value=check()
                if value:return value
                time.sleep(.05)
            raise AssertionError('Five-second materialization target missed')
        try:
            assert api('setup')['settings']['choice']=='not_chosen';assert api('history')['total']==0
            change(choice='skipped');assert api('history')['total']==0
            change(choice='enabled',step='bridge')
            assert api('setup')['client']=='unknown'
            pairing=api('setup/token',{})
            def http(path,value,origin=None):
                headers={'X-Omoi-Token':pairing['token'],'Content-Type':'application/json'}
                if origin:headers['Origin']=origin
                req=urllib.request.Request(f'http://127.0.0.1:{port}'+path,data=json.dumps(value).encode(),headers=headers)
                try:
                    with urllib.request.urlopen(req,timeout=5) as r:return r.status,json.load(r)
                except urllib.error.HTTPError as e:return e.code,json.load(e)
            assert http('/hello',dict(protocol=1,version='0.2.0',permission=True),'chrome-extension://'+'a'*32+'/')[0]==403
            assert http('/hello',dict(protocol=1,version='0.2.0',permission=True))[0]==200
            assert api('setup')['client']=='hello_observed';assert api('setup')['settings']['last_processed'] is None
            start=time.monotonic()
            for n in range(3):assert http('/event',event(n))[0]==200
            wait_for(lambda:api('history')['total']==1 and api('setup')['pending']==0)
            latency=(time.monotonic()-start)*1000
            assert api('item?id=1')['played_seconds']==2
            # Durable receipt replay doesn't double-credit.
            assert http('/event',event(2))[1]['accepted'] is False
            change(choice='stopped');assert api('setup')['transport']=='stopped'
            change(choice='enabled');assert api('setup')['client']=='unknown'
            # Actual host process uses only a disposable executable/config directory.
            hostdir=root/'host';hostdir.mkdir();shutil.copy2(native,hostdir/native.name)
            origin='chrome-extension://'+'a'*32+'/'
            (hostdir/'omoi-host.json').write_text(json.dumps(dict(home=str(home),allowed_origins=[origin])))
            def host():
                proc=subprocess.Popen([str(hostdir/native.name),origin],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env);processes.append(proc);return proc
            h=host()
            def frame(proc,value):
                body=json.dumps(value).encode();proc.stdin.write(struct.pack('<I',len(body))+body);proc.stdin.flush()
                size=proc.stdout.read(4);assert len(size)==4,'Host exited without a reply'
                length=struct.unpack('<I',size)[0];assert length<=32768
                return json.loads(proc.stdout.read(length))
            hello=dict(type='hello',protocol_min=1,protocol_max=1,version='0.2.0',permission=True,binding=None)
            assert not frame(h,hello|{'version':'9.0.0'})['ok']
            assert not frame(h,hello|{'permission':False})['ok']
            ready=frame(h,hello);assert ready['ok']
            envelope=dict(type='event',binding=ready['binding'],generation=ready['generation'],event=event(3,'native'))
            # Writer contention cannot acknowledge an event that was not enqueued.
            with sqlite3.connect(home/'history.sqlite') as locked:
                locked.execute('BEGIN IMMEDIATE')
                busy=frame(h,envelope)
                assert not busy['ok'] and busy['retry']
                locked.rollback()
            assert frame(h,envelope)['ok'];assert frame(h,envelope)['accepted'] is False
            # Progress-only setup changes preserve queued/native capture epochs.
            change(step='play')
            assert frame(h,envelope)['ok']
            assert not frame(h,envelope|{'binding':'different-home'})['ok']
            change(choice='stopped');assert not frame(h,envelope|{'event':event(4,'native')})['ok']
            change(choice='enabled');assert not frame(h,envelope)['ok']
            ready=frame(h,hello);assert ready['ok']
            ui.terminate();ui.wait(timeout=10)
            assert not frame(h,dict(type='status'))['ok'],'Foreground capture must stop after app exits'
            ui,base,credential=start_ui();assert api('setup')['settings']['choice']=='enabled'
            change(background=True)
            ready=frame(h,hello);assert ready['ok'];ui.terminate();ui.wait(timeout=10)
            assert frame(h,dict(type='event',binding=ready['binding'],generation=ready['generation'],event=event(4,'native')))['ok']
            h.stdin.close();assert h.wait(timeout=5)==0
            # Wrong origin and truncated input cannot enqueue.
            bad=subprocess.run([str(hostdir/native.name),'chrome-extension://'+'b'*32+'/'],input=b'',capture_output=True,env=env,timeout=5);assert bad.returncode!=0
            truncated=host();truncated.stdin.write(b'\x10\0');truncated.stdin.close();truncated.wait(timeout=5);assert not truncated.stdout.read()
            with sqlite3.connect(home/'history.sqlite') as c:
                assert c.execute('pragma user_version').fetchone()[0]==3
                assert c.execute('select count(*) from raw_events').fetchone()[0]==0
            print(json.dumps({'result':'pass','environment':os.name,'http_event_to_catalogue_ms':round(latency,2),'checks':['history-only','resumable consent','hello distinct from recording','origin rejection','automatic materialization','dedupe','stop/re-enable generation','native framing/origin/binding/version/permission','foreground lifetime','explicit background','EOF/truncated input','schema 3 retained'],'limitations':['No Edge permission UI, WebView2, physical sleep or installer execution proved by this script.']},indent=2))
        finally:
            for proc in processes:
                if proc.poll() is None:proc.kill()
                proc.wait(timeout=10)
                for stream in (proc.stdin,proc.stdout,proc.stderr):
                    if stream and not stream.closed:stream.close()
if __name__=='__main__':main()
