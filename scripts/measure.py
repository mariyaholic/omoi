"""Measure only generated Omoi history on Windows; stdlib only. See --help."""
import argparse
import ctypes
from ctypes import wintypes
import json
import hashlib
import os
from pathlib import Path
import platform
import sqlite3
import statistics
import subprocess
import time

class Memory(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD)] + [(n, ctypes.c_size_t) for n in ("peak_working", "working", "peak_paged", "paged", "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile")]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--exe", type=Path, required=True)
    p.add_argument("--generator", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True, help="New directory; refuses reuse")
    p.add_argument("--runs", type=int, default=20)
    p.add_argument("--full-runs", type=int, default=1)
    p.add_argument("--views", type=int, default=100000)
    p.add_argument("--items", type=int, default=40000)
    a = p.parse_args()
    if os.name != "nt" or not 1 <= a.runs <= 100 or not 1 <= a.full_runs <= 100:
        p.error("Windows required; runs must be 1..100")
    a.out.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    empty = a.out.resolve() / "empty-browser-roots"
    empty.mkdir()
    for key in ("LOCALAPPDATA", "APPDATA", "HOME", "XDG_CONFIG_HOME"):
        env[key] = str(empty)
    fixture = a.out.resolve() / "fixture"
    subprocess.run([str(a.generator.resolve()), "--out", str(fixture), "--views", str(a.views), "--items", str(a.items)], check=True, env=env, capture_output=True)
    env["OMOI_HOME"] = str(fixture / "home")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Memory), wintypes.DWORD]
    kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    def run(label):
        start = time.perf_counter()
        proc = subprocess.Popen([str(a.exe.resolve()), "collect"], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
        output, stderr = proc.communicate(timeout=600)
        elapsed = (time.perf_counter() - start) * 1000
        if proc.returncode:
            raise RuntimeError(f"collect exit {proc.returncode}: {stderr.decode(errors='replace')}")
        memory = Memory()
        memory.cb = ctypes.sizeof(memory)
        if not psapi.GetProcessMemoryInfo(int(proc._handle), ctypes.byref(memory), memory.cb):
            raise ctypes.WinError(ctypes.get_last_error())
        times = [wintypes.FILETIME() for _ in range(4)]
        if not kernel.GetProcessTimes(int(proc._handle), *(ctypes.byref(t) for t in times)):
            raise ctypes.WinError(ctypes.get_last_error())
        cpu_ms = sum((t.dwHighDateTime << 32) | t.dwLowDateTime for t in times[2:]) / 10000
        summary = json.loads(output)
        return {"case": label, "wall_ms": elapsed, "cpu_ms": cpu_ms, "peak_working_set_bytes": memory.peak_working, "summary": summary}
    results = []
    for index in range(a.full_runs):
        if index:
            for name in ("history.sqlite", "history.sqlite-wal", "history.sqlite-shm"):
                (fixture / "home" / name).unlink(missing_ok=True)
        result = run("full-import")
        results.append(result)
        print(json.dumps({"full_run": index+1, "wall_ms": result["wall_ms"], "peak_bytes": result["peak_working_set_bytes"]}), flush=True)
    if results[0]["summary"]["views_inserted"] != a.views or results[0]["summary"]["items_inserted"] != a.items:
        raise RuntimeError("synthetic counts differ from requested counts")
    results += [run("unchanged") for _ in range(a.runs)]
    if any(r["summary"]["views_inserted"] for r in results if r["case"] == "unchanged"):
        raise RuntimeError("unchanged rerun inserted views")
    with sqlite3.connect(fixture / "History") as c:
        last_id, last_time = c.execute("SELECT max(id),max(visit_time) FROM visits").fetchone()
        c.executemany("INSERT INTO visits VALUES(?,?,?,?)", [(last_id+i, i, last_time+i*3600000000, 1000000) for i in range(1,11)])
    results.append(run("ten-new-rows"))
    db = fixture / "home/history.sqlite"
    before = sum(x.stat().st_size for x in db.parent.glob("history.sqlite*"))
    with sqlite3.connect(db) as c:
        c.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        counts = {t: c.execute(f"SELECT count(*) FROM {t}").fetchone()[0] for t in ("items", "views")}
        page_bytes = c.execute("PRAGMA page_count").fetchone()[0] * c.execute("PRAGMA page_size").fetchone()[0]
    c.close()
    after = sum(x.stat().st_size for x in db.parent.glob("history.sqlite*"))
    warm = [r for r in results if r["case"] == "unchanged"]
    def stats(key):
        values = sorted(r[key] for r in warm)
        return {"median": statistics.median(values), "p95": values[max(0, (len(values)*95+99)//100-1)], "max": max(values)}
    report = {"platform": platform.platform(), "cpu": platform.processor(), "python": platform.python_version(), "sqlite_measurement_version": sqlite3.sqlite_version, "runs": a.runs, "counts_after_increment": counts, "catalogue_bytes_before_checkpoint": before, "catalogue_bytes_after_checkpoint": after, "page_bytes": page_bytes, "binary_bytes": a.exe.stat().st_size, "warm_wall_ms": stats("wall_ms"), "warm_cpu_ms": stats("cpu_ms"), "warm_peak_bytes": stats("peak_working_set_bytes"), "results": results, "limitations": ["One full import; 20-run full-import distribution not measured", "Storage type and temp/spool peak not captured", "No private history; mixed-platform synthetic baseline only", "No long-title stress run in this report"]}
    report["binary_sha256"] = hashlib.sha256(a.exe.read_bytes()).hexdigest()
    report["full_runs"] = a.full_runs
    if a.full_runs > 1:
        report["limitations"][0] = "Full-import distribution measured; first run coldness not controlled"
    (a.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "results"}, indent=2))

if __name__ == "__main__":
    main()
