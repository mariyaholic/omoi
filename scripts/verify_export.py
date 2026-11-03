"""Measure CSV/JSON export of an existing synthetic catalogue on Windows.

Exports are created only in disposable temporary storage and are deleted before
the aggregate receipt is written. This does not run browser discovery or collect.
"""
import argparse
from contextlib import closing
import csv
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile
import time

from measure import Memory


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary',required=True,type=Path)
    parser.add_argument('--synthetic-home',required=True,type=Path)
    parser.add_argument('--out',required=True,type=Path)
    args=parser.parse_args()
    assert os.name=='nt','Windows process counters required'
    database=args.synthetic_home.resolve()/'history.sqlite'
    with closing(sqlite3.connect(database.as_uri()+'?mode=ro',uri=True)) as c:
        expected=c.execute('SELECT count(*) FROM views').fetchone()[0]
    assert expected>=100000,'Use the 100k-view synthetic measurement fixture'
    before=hashlib.sha256(database.read_bytes()).hexdigest()
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    psapi=ctypes.WinDLL('psapi',use_last_error=True)
    psapi.GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(Memory),wintypes.DWORD]
    kernel.GetProcessTimes.argtypes=[wintypes.HANDLE]+[ctypes.POINTER(wintypes.FILETIME)]*4
    results=[]
    with tempfile.TemporaryDirectory(prefix='omoi-export-acceptance-') as directory:
        root=Path(directory)
        env=os.environ|{'OMOI_HOME':str(args.synthetic_home.resolve())}
        for key in ('LOCALAPPDATA','APPDATA','HOME','XDG_CONFIG_HOME','XDG_DATA_HOME'):
            env[key]=str(root/'empty')
        for fmt in ('csv','json'):
            for index in range(3):
                destination=root/f'{index}.{fmt}'
                start=time.perf_counter()
                proc=subprocess.Popen([str(args.binary.resolve()),'export','--'+fmt,'--out',str(destination)],env=env,stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW)
                try:
                    _,error=proc.communicate(timeout=120)
                except BaseException:
                    proc.kill()
                    proc.communicate()
                    raise
                elapsed=(time.perf_counter()-start)*1000
                assert proc.returncode==0,error.decode(errors='replace')
                memory=Memory()
                memory.cb=ctypes.sizeof(memory)
                assert psapi.GetProcessMemoryInfo(int(proc._handle),ctypes.byref(memory),memory.cb)
                times=[wintypes.FILETIME() for _ in range(4)]
                assert kernel.GetProcessTimes(int(proc._handle),*(ctypes.byref(t) for t in times))
                cpu_ms=sum((t.dwHighDateTime<<32)|t.dwLowDateTime for t in times[2:])/10000
                with destination.open(encoding='utf-8',newline='') as f:
                    rows=sum(1 for _ in csv.DictReader(f)) if fmt=='csv' else len(json.load(f))
                assert rows==expected,(fmt,rows,expected)
                results.append({'format':fmt,'run':index+1,'rows':rows,'wall_ms':elapsed,'cpu_ms':cpu_ms,'peak_working_set_bytes':memory.peak_working,'export_bytes':destination.stat().st_size})
                destination.unlink()
    assert before==hashlib.sha256(database.read_bytes()).hexdigest()
    passed=all(0<r['peak_working_set_bytes']<30000000 for r in results)
    report={'result':'pass' if passed else 'failed','binary_sha256':hashlib.sha256(args.binary.read_bytes()).hexdigest(),'runs':results,'scope':'existing generated 100k-view fixture; Windows lifetime process counters; three runs per format; not a p95 estimate; parser and browser memory excluded','catalogue_main_file_unchanged':True,'temporary_exports_deleted':True}
    args.out.write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps(report,indent=2))
    if not passed:
        raise SystemExit('Export process exceeded the 30 MB collector budget')


if __name__=='__main__':
    main()
