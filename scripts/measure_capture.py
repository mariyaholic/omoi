"""Linux synthetic UI/listener/native costs. Excludes a real GUI/browser process tree."""
import argparse
import json
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import tempfile
import time
import urllib.request
p=argparse.ArgumentParser(description=__doc__);p.add_argument('--bin-dir',type=Path,required=True);a=p.parse_args()
assert Path('/proc/self/stat').exists(),'Linux /proc counters required'
def counters(proc):
    stat=Path(f'/proc/{proc.pid}/stat').read_text().split();status=Path(f'/proc/{proc.pid}/status').read_text().splitlines()
    return {'cpu_ms':sum(float(stat[i]) for i in [13,14])*1000/os.sysconf('SC_CLK_TCK'),'peak_rss_bytes':next(int(s.split()[1])*1024 for s in status if s.startswith('VmHWM:')),'voluntary_context_switches':next(int(s.split()[1]) for s in status if s.startswith('voluntary_ctxt_switches:'))}
def disk(root):return sum(f.stat().st_size for f in root.rglob('*') if f.is_file())
def sample(proc,seconds=5):
    before=counters(proc);time.sleep(seconds);after=counters(proc)
    return {'seconds':seconds,'cpu_ms':after['cpu_ms']-before['cpu_ms'],'peak_rss_bytes':after['peak_rss_bytes'],'main_thread_voluntary_switches':after['voluntary_context_switches']-before['voluntary_context_switches']}
processes=[]
with tempfile.TemporaryDirectory(prefix='omoi-cost-') as td:
    root=Path(td);home=root/'data';home.mkdir();(home/'config.toml').write_text('browsers=[]\n')
    env=os.environ|{'OMOI_HOME':str(home),'LOCALAPPDATA':str(root/'empty'),'APPDATA':str(root/'empty'),'HOME':str(root/'empty')}
    with socket.socket() as s:s.bind(('127.0.0.1',0));port=s.getsockname()[1]
    (home/'config.toml').write_text(f'browsers=[]\nlistener_port={port}\n')
    try:
        start=time.monotonic();ui=subprocess.Popen([str((a.bin_dir/'omoi').resolve()),'ui','--port','0','--idle-seconds','120'],env=env,stdout=subprocess.PIPE,text=True);processes.append(ui)
        base,token=ui.stdout.readline().strip().split('/#token=');cold=(time.monotonic()-start)*1000
        def api(path,body=None):
            with urllib.request.urlopen(urllib.request.Request(base+'/api/'+path,data=None if body is None else json.dumps(body).encode(),headers={'X-Omoi-Token':token,'Origin':base,'Content-Type':'application/json'}),timeout=10) as r:return json.load(r)
        off=sample(ui);api('setup',{'generation':0,'choice':'enabled'});idle=sample(ui)
        local=api('setup/token',{});before_disk=disk(home);before=counters(ui);latency=[]
        for n in range(100):
            event=dict(event_id=f'fixture:{n}',producer_id='fixture',seq=n,url='https://www.youtube.com/watch?v=abcdefghijk',at_ms=1700000000000+n*1000,state='playing',position_ms=n*1000,cumulative_played_ms=n*1000,title='Synthetic resource fixture',channel=None,duration_s=600)
            start=time.monotonic()
            with urllib.request.urlopen(urllib.request.Request(f'http://127.0.0.1:{port}/event',data=json.dumps(event).encode(),headers={'X-Omoi-Token':local['token'],'Content-Type':'application/json'}),timeout=10) as r:r.read()
            deadline=time.monotonic()+5
            while api('setup')['pending']:
                assert time.monotonic()<deadline;time.sleep(.02)
            latency.append((time.monotonic()-start)*1000)
        after=counters(ui)
        hostdir=root/'host';hostdir.mkdir();shutil.copy2(a.bin_dir/'omoi-native',hostdir/'omoi-native');origin='chrome-extension://'+'a'*32+'/'
        (hostdir/'omoi-host.json').write_text(json.dumps({'home':str(home),'allowed_origins':[origin]}))
        native=subprocess.Popen([str(hostdir/'omoi-native'),origin],stdin=subprocess.PIPE,stdout=subprocess.PIPE,env=env);processes.append(native)
        message=json.dumps({'type':'hello','protocol_min':1,'protocol_max':1,'version':'0.2.0','permission':True,'binding':None}).encode();native.stdin.write(struct.pack('<I',len(message))+message);native.stdin.flush();size=struct.unpack('<I',native.stdout.read(4))[0];assert json.loads(native.stdout.read(size))['ok']
        native_idle=sample(native)
        print(json.dumps({'environment':'Linux /proc; synthetic; no GUI/browser','cold_ui_server_launch_ms':round(cold,2),'capture_off':off,'capture_on_idle':idle,'capture_100_events':{'cpu_ms':after['cpu_ms']-before['cpu_ms'],'peak_rss_bytes':after['peak_rss_bytes'],'catalogue_disk_growth_bytes':disk(home)-before_disk,'event_to_catalogue_ms':{'median':round(sorted(latency)[50],2),'max':round(max(latency),2)}},'native_host_idle':native_idle,'limits':['5-second idle sample; CPU tick quantization is not a zero-CPU guarantee','RSS includes UI server and its managed listener, not WebView2 or Edge','Context switches above measure the main thread only, not total wake frequency','Existing Windows listener 10 MB gate unchanged; Windows process-tree and long-duration measurements pending']},indent=2))
    finally:
        for proc in processes:
            if proc.poll() is None:proc.kill()
            proc.wait()
