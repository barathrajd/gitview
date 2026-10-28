#!/usr/bin/env python3
"""Check Escape closes a slow comparison or object load promptly and no stale result appears (temporary repo, injected Git delay)."""
import argparse
import os
from pathlib import Path
import runpy
import shutil
import tempfile
import time

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session'];run=support['run']
GITVIEW=b'GITVIEW'
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'));parser.add_argument('--samples',type=int,default=5);args=parser.parse_args();binary=args.binary.resolve();git=shutil.which('git')
 with tempfile.TemporaryDirectory(prefix='gitview-cancel-check-') as temp:
  temp=Path(temp);root=temp/'repo';root.mkdir();shim=temp/'bin';shim.mkdir();marker=temp/'delay'
  def g(*a):return run([git,*a],root)
  g('init','-qb','main');g('config','user.name','Test');g('config','user.email','test@example.com');g('config','commit.gpgsign','false')
  (root/'sample.txt').write_text('base\n');g('add','--all');g('commit','-qm','base');(root/'sample.txt').write_text('changed\n');g('add','--all');g('commit','-qm','change')
  (shim/'git').write_text('#!/bin/sh\nif [ -f "$PERF_DELAY_MARKER" ]; then sleep 0.25; fi\nexec "$PERF_REAL_GIT" "$@"\n');(shim/'git').chmod(0o700)
  env=dict(os.environ,TERM='xterm-256color',PATH=str(shim)+os.pathsep+os.environ['PATH'],PERF_DELAY_MARKER=str(marker),PERF_REAL_GIT=git)
  session=Session(binary,root,env,140,35);results={}
  try:
   session.wait_for([b'GITVIEW']);session.receive(.8);os.write(session.master,b'\t');session.receive(.3)
   for name,open_key,input_title,result_title in [('comparison',b'D',b'COMPARE COMMIT SNAPSHOTS',b'COMPARISON REVIEW'),('object',b'o',b'INSPECT GIT OBJECT',b'OBJECT DESCRIPTION')]:
    escapes=[]
    for _ in range(args.samples):
     marker.touch();session.wait_for([GITVIEW]);os.write(session.master,open_key);session.wait_for([input_title]);session.receive(.1)
     os.write(session.master,b'\r');session.receive(.05)
     start=time.monotonic();os.write(session.master,b'\x1b')
     while GITVIEW not in session.screen.text() or input_title in session.screen.text():
      session.receive(.002)
      assert time.monotonic()-start<5,f'{name}: Escape did not close the pending load'
     escapes.append((time.monotonic()-start)*1000)
     # The abandoned load finishes in the background; its result must never reappear.
     session.receive(2.5);screen=session.screen.text()
     assert result_title not in screen and GITVIEW in screen,f'{name}: stale result appeared\n'+screen.decode()
    marker.unlink()
    os.write(session.master,open_key);session.wait_for([input_title]);os.write(session.master,b'\r');session.wait_for([result_title],timeout=10)
    os.write(session.master,b'\x1b');session.wait_for([input_title]);os.write(session.master,b'\x1b');session.wait_for([GITVIEW])
    results[name]=dict(escape_to_cockpit_ms=[round(v,1) for v in escapes],max_ms=round(max(escapes),1))
   os.write(session.master,b'q');session.child.wait(timeout=3)
  finally:session.close()
 import json;print(json.dumps(dict(injected_git_delay_ms_per_command=250,results=results),indent=1))
 assert all(item['max_ms']<=100 for item in results.values()),'Escape exceeded 100 ms'
 print('PASS: Escape closed slow comparison and object loads within 100 ms; no stale result appeared; the next load succeeded')
if __name__=='__main__':main()
