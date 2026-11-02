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
import threading

class Memory(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD)] + [(n, ctypes.c_size_t) for n in ("peak_working", "working", "peak_paged", "paged", "peak_nonpaged", "nonpaged", "pagefile", "peak_pagefile")]

class IoCounters(ctypes.Structure):
    _fields_ = [(name, ctypes.c_ulonglong) for name in ("read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes", "other_bytes")]

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--exe", type=Path, required=True)
    p.add_argument("--generator", type=Path, required=True)
    p.add_argument("--out", type=Path, required=True, help="New directory; refuses reuse")
    p.add_argument("--runs", type=int, default=20)
    p.add_argument("--full-runs", type=int, default=1)
    p.add_argument("--views", type=int, default=100000)
    p.add_argument("--items", type=int, default=40000)
    p.add_argument("--long-titles", action="store_true")
    a = p.parse_args()
    if os.name != "nt" or not 1 <= a.runs <= 100 or not 1 <= a.full_runs <= 100:
        p.error("Windows required; runs must be 1..100")
    if a.items < 10 or a.views < a.items:
        p.error("at least 10 items required, with views >= items")
    a.out.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    empty = a.out.resolve() / "empty-browser-roots"
    empty.mkdir()
    for key in ("LOCALAPPDATA", "APPDATA", "HOME", "XDG_CONFIG_HOME"):
        env[key] = str(empty)
    fixture = a.out.resolve() / "fixture"
    subprocess.run([str(a.generator.resolve()), "--out", str(fixture), "--views", str(a.views), "--items", str(a.items)] + (["--long-titles"] if a.long_titles else []), check=True, env=env, capture_output=True)
    env["OMOI_HOME"] = str(fixture / "home")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Memory), wintypes.DWORD]
    kernel.GetProcessTimes.argtypes = [wintypes.HANDLE] + [ctypes.POINTER(wintypes.FILETIME)] * 4
    kernel.GetProcessIoCounters.argtypes = [wintypes.HANDLE, ctypes.POINTER(IoCounters)]
    def disk_bytes(root):
        size = 0
        for path in root.rglob("*"):
            try:
                if path.is_file():
                    size += path.stat().st_size
            except FileNotFoundError:
                pass  # Collector owns and removes transient files.
        return size
    def run(label):
        stop = threading.Event()
        sampled = [0]
        def sample():
            while not stop.is_set():
                sampled[0] = max(sampled[0], disk_bytes(fixture / "home/tmp"))
                stop.wait(0.05)
        sampler = threading.Thread(target=sample, daemon=True)
        sampler.start()
        start = time.perf_counter()
        proc = subprocess.Popen([str(a.exe.resolve()), "collect"], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, creationflags=subprocess.CREATE_NO_WINDOW)
        try:
            output, stderr = proc.communicate(timeout=600)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.communicate()
            raise
        finally:
            stop.set()
            sampler.join()
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
        io = IoCounters()
        if not kernel.GetProcessIoCounters(int(proc._handle), ctypes.byref(io)):
            raise ctypes.WinError(ctypes.get_last_error())
        summary = json.loads(output)
        if memory.peak_working == 0:
            raise RuntimeError("OS returned a zero lifetime peak")
        return {"case": label, "wall_ms": elapsed, "cpu_ms": cpu_ms, "peak_working_set_bytes": memory.peak_working, "sampled_temp_peak_bytes": sampled[0], "io": {name: getattr(io, name) for name, _ in io._fields_}, "summary": summary}
    results = []
    for index in range(a.full_runs):
        if index:
            for name in ("history.sqlite", "history.sqlite-wal", "history.sqlite-shm"):
                (fixture / "home" / name).unlink(missing_ok=True)
        result = run("full-import")
        if result["summary"]["views_inserted"] != a.views or result["summary"]["items_inserted"] != a.items:
            raise RuntimeError("synthetic full-import counts differ")
        results.append(result)
        print(json.dumps({"full_run": index+1, "wall_ms": result["wall_ms"], "peak_bytes": result["peak_working_set_bytes"]}), flush=True)
    if results[0]["summary"]["views_inserted"] != a.views or results[0]["summary"]["items_inserted"] != a.items:
        raise RuntimeError("synthetic counts differ from requested counts")
    results += [run("unchanged") for _ in range(a.runs)]
    if any(r["summary"]["views_inserted"] for r in results if r["case"] == "unchanged"):
        raise RuntimeError("unchanged rerun inserted views")
    for _ in range(a.runs):
        with sqlite3.connect(fixture / "History") as c:
            last_id, last_time = c.execute("SELECT max(id),max(visit_time) FROM visits").fetchone()
            c.executemany("INSERT INTO visits VALUES(?,?,?,?)", [(last_id+i, i, last_time+i*3600000000, 1000000) for i in range(1,11)])
        c.close()
        result = run("ten-new-rows")
        if result["summary"]["views_inserted"] != 10:
            raise RuntimeError("incremental count differs")
        results.append(result)
    with sqlite3.connect(fixture / "History") as c:
        c.execute("UPDATE urls SET title='Metadata update' WHERE id=1")
    c.close()
    results.append(run("metadata-only"))
    c = sqlite3.connect(fixture / "places.sqlite")
    c.execute("PRAGMA journal_mode=WAL")
    c.execute("PRAGMA wal_autocheckpoint=0")
    last_id, last_time = c.execute("SELECT max(id),max(visit_date) FROM moz_historyvisits").fetchone()
    c.execute("INSERT INTO moz_historyvisits VALUES(?,?,?)", (last_id+1, 1, last_time+100000000000000))
    c.commit()
    result = run("wal-only")
    if result["summary"]["views_inserted"] != 1:
        raise RuntimeError("WAL-only visit was not imported exactly once")
    results.append(result)
    c.close()
    db = fixture / "home/history.sqlite"
    before = sum(x.stat().st_size for x in db.parent.glob("history.sqlite*"))
    with sqlite3.connect(db) as c:
        c.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        counts = {t: c.execute(f"SELECT count(*) FROM {t}").fetchone()[0] for t in ("items", "views")}
        page_bytes = c.execute("PRAGMA page_count").fetchone()[0] * c.execute("PRAGMA page_size").fetchone()[0]
        freelist_bytes = c.execute("PRAGMA freelist_count").fetchone()[0] * c.execute("PRAGMA page_size").fetchone()[0]
    c.close()
    after = sum(x.stat().st_size for x in db.parent.glob("history.sqlite*"))
    warm = [r for r in results if r["case"] == "unchanged"]
    def stats(key):
        values = sorted(r[key] for r in warm)
        return {"median": statistics.median(values), "p95": values[max(0, (len(values)*95+99)//100-1)], "max": max(values)}
    report = {"platform": platform.platform(), "cpu": platform.processor(), "python": platform.python_version(), "sqlite_measurement_version": sqlite3.sqlite_version, "runs": a.runs, "counts_after_increment": counts, "catalogue_bytes_before_checkpoint": before, "catalogue_bytes_after_checkpoint": after, "page_bytes": page_bytes, "binary_bytes": a.exe.stat().st_size, "warm_wall_ms": stats("wall_ms"), "warm_cpu_ms": stats("cpu_ms"), "warm_peak_bytes": stats("peak_working_set_bytes"), "results": results, "limitations": ["One full import; 20-run full-import distribution not measured", "Storage type and temp/spool peak not captured", "No private history; mixed-platform synthetic baseline only", "No long-title stress run in this report"]}
    report["binary_sha256"] = hashlib.sha256(a.exe.read_bytes()).hexdigest()
    report["full_runs"] = a.full_runs
    report["long_titles"] = a.long_titles
    report["freelist_bytes"] = freelist_bytes
    report["logs_bytes"] = disk_bytes(fixture / "home/logs")
    report["helper_bytes"] = (a.exe.parent / "omoiw.exe").stat().st_size
    info = subprocess.run([str(a.exe.resolve()), "db", "info"], env=env, capture_output=True, check=True)
    report["collector_sqlite_version"] = json.loads(info.stdout)["sqlite_version"]
    report["limitations"] = ["First run coldness and competing system load not controlled", "Temporary disk peak sampled every 50ms: lower bound, not a guaranteed lifetime maximum", "No private history; synthetic data only", "Storage model recorded separately; no Linux resource measurements"]
    report["case_stats"] = {}
    for label in {r["case"] for r in results}:
        subset = [r for r in results if r["case"] == label]
        report["case_stats"][label] = {"runs": len(subset)}
        for key in ("wall_ms", "cpu_ms", "peak_working_set_bytes", "sampled_temp_peak_bytes"):
            values = sorted(r[key] for r in subset)
            report["case_stats"][label][key] = {"median": statistics.median(values), "p95": values[max(0,(len(values)*95+99)//100-1)], "max": max(values)}
    if a.full_runs > 1:
        report["limitations"][0] = "Full-import distribution measured; first run coldness not controlled"
    (a.out / "report.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps({k: v for k, v in report.items() if k != "results"}, indent=2))

if __name__ == "__main__":
    main()
