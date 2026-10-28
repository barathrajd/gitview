#!/usr/bin/env python3
"""Profile native keypress-to-screen latency for list navigation (temporary fixture).

Each sample sends one key and waits for the reconstructed screen to change.
Latency includes the PTY and screen reconstruction, so treat it as an upper bound.
"""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import tempfile
import time

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session'];fixture=support['fixture']
def p95(values):
 values=sorted(values);return values[max(0,-(-len(values)*95//100)-1)]
def press(session,key,timeout=2):
 before=session.screen.text();start=time.monotonic();os.write(session.master,key)
 while time.monotonic()-start<timeout:
  session.receive(.002)
  if session.screen.text()!=before:return (time.monotonic()-start)*1000
 raise RuntimeError(f'Screen did not change after {key!r}')
def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'))
 parser.add_argument('--fixture',choices=['small','medium','stress'],default='stress')
 parser.add_argument('--samples',type=int,default=100)
 parser.add_argument('--check',action='store_true',help='Fail if any p95 exceeds 100 ms')
 args=parser.parse_args();binary=args.binary.resolve();git=shutil.which('git');results={}
 with tempfile.TemporaryDirectory(prefix='gitview-navigation-') as temp:
  root=Path(temp);meta=fixture(root,args.fixture,git)
  for width,height in [(140,35),(80,24)]:
   session=Session(binary,root,dict(os.environ,TERM='xterm-256color'),width,height)
   try:
    session.wait_for([b'GITVIEW']);session.receive(1.5)
    worktree=[press(session,b'j') for _ in range(args.samples)]
    results[f'{width}x{height}']={'worktree_down_ms':worktree,'worktree_down_p95_ms':p95(worktree)}
    os.write(session.master,b'\t');session.receive(.5)
    history=[]
    for _ in range(min(args.samples,150)):history.append(press(session,b'j'))
    results[f'{width}x{height}'].update({'history_down_ms':history,'history_down_p95_ms':p95(history)})
    os.write(session.master,b'q');session.child.wait(timeout=3)
   finally:session.close()
 summary={key:{name:round(value,2) for name,value in item.items() if name.endswith('p95_ms')} for key,item in results.items()}
 print(json.dumps({'fixture':meta,'samples_per_series':args.samples,'p95':summary},indent=1))
 if args.check:
  assert all(value<=100 for item in summary.values() for value in item.values()),'navigation p95 exceeded 100 ms'
if __name__=='__main__':main()
