#!/usr/bin/env python3
"""Exercise ordinary diff cancellation, committed views, and hunk staging in temporary repos."""
import argparse
import os
from pathlib import Path
import runpy
import shutil
import tempfile
import time

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session'];run=support['run']

def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'));args=parser.parse_args();git=shutil.which('git');binary=args.binary.resolve()
 with tempfile.TemporaryDirectory(prefix='gitview-ordinary-check-') as directory:
  temp=Path(directory);root=temp/'repo';root.mkdir();shim=temp/'bin';shim.mkdir();marker=temp/'delay'
  def g(*args):return run([git,*args],root)
  g('init','-qb','main');g('config','user.name','Test');g('config','user.email','test@example.com');g('config','commit.gpgsign','false')
  def rust(number):return ''.join(f'pub fn value_{index}() -> usize {{ let number = {number}; number + {index} }}\n' for index in range(400))
  original=''.join(f'context line {index}\n' for index in range(40))
  (root/'sample.rs').write_text(rust(1));(root/'two-hunks.txt').write_text(original);g('add','--all');g('commit','-qm','base')
  (root/'sample.rs').write_text(rust(2));g('add','--all');g('commit','-qm','target');(root/'sample.rs').write_text(rust(3))
  changed=original.replace('context line 2\n','first_changed_unique\n').replace('context line 30\n','second_changed_unique\n');(root/'two-hunks.txt').write_text(changed)
  (shim/'git').write_text('#!/bin/sh\nif [ -f "$PERF_DELAY_MARKER" ]; then sleep 0.25; fi\nexec "$PERF_REAL_GIT" "$@"\n');(shim/'git').chmod(0o700)
  env=dict(os.environ,TERM='xterm-256color',PATH=str(shim)+os.pathsep+os.environ['PATH'],PERF_DELAY_MARKER=str(marker),PERF_REAL_GIT=git)
  def open_session():
   session=Session(binary,root,env,140,35);session.wait_for([b'GITVIEW']);session.receive(.8);return session
  def send(session,keys):os.write(session.master,keys);session.receive(.15)
  session=open_session()
  try:
   marker.touch();os.write(session.master,b'\r');loading=session.wait_for([b'Loading diff']);os.write(session.master,b'\x1b');cancel=session.wait_for([b'WORKTREE']);marker.unlink();session.receive(.8)
   assert b'WORKTREE' in session.screen.text() and b'BEFORE' not in session.screen.text()
   send(session,b'q');session.child.wait(timeout=3)
   print(f'Ordinary loading feedback {loading:.1f} ms; Escape {cancel:.1f} ms with delayed Git')
  finally:marker.unlink(missing_ok=True);session.close()
  for raw,unified in [(False,False),(True,False),(False,True),(True,True)]:
   g('reset','-q','HEAD','--','.');session=open_session()
   try:
    send(session,b'j');os.write(session.master,b'\r');session.wait_for([b'HUNK 1/2'])
    if raw:send(session,b'v')
    if unified:send(session,b'\t')
    os.write(session.master,b']');session.receive(.2);assert b'second_changed_unique' in session.screen.text() and b'first_changed_unique' not in session.screen.text(),session.screen.text().decode()
    os.write(session.master,b's');deadline=time.monotonic()+10
    while True:
     session.receive(.1)
     if '+second_changed_unique' in g('diff','--cached') and b'first_changed_unique' in session.screen.text() and b'second_changed_unique' not in session.screen.text():break
     assert time.monotonic()<deadline,session.screen.text().decode()
    staged=g('diff','--cached');assert '+second_changed_unique' in staged and '+first_changed_unique' not in staged
    send(session,b'\x1b');send(session,b'q');session.child.wait(timeout=3)
   finally:session.close()
  g('reset','-q','HEAD','--','.');session=open_session()
  try:
   send(session,b'H');os.write(session.master,b'\r');session.wait_for([b'CHANGED FILES']);os.write(session.master,b'\r');session.wait_for([b'BEFORE']);session.receive(.2)
   assert b'number = 1' in session.screen.text() and b'number = 2' in session.screen.text()
   for keys in [b'v',b'\t',b'v',b'\t',b'j',b'k']:send(session,keys)
   send(session,b's');assert g('diff','--cached')==''
   send(session,b'\x1b');send(session,b'\x1b');send(session,b'q');session.child.wait(timeout=3)
  finally:session.close()
  assert g('rev-list','--count','HEAD')=='2'
 print('PASS: cancelled loading stayed closed; second-hunk staging matched all four variants; committed-file diff stayed read-only')
if __name__=='__main__':main()
