"""Bundle reviewed local capture sources; --check detects shipped artifact drift."""
import argparse
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--check',action='store_true');a=p.parse_args()
r=Path(__file__).resolve().parents[1]
core=(r/'capture/core.js').read_text()
adapter=(r/'capture/userscript-adapter.js').read_text()
pos=adapter.index('/* Capture requires')
outputs={'userscripts/omoi.user.js':adapter[:pos]+core+'\n'+adapter[pos:], 'extension/core.js':core}
for name,text in outputs.items():
    if a.check: assert (r/name).read_text()==text,name+' differs; run scripts/build_capture.py'
    else: (r/name).write_text(text)
