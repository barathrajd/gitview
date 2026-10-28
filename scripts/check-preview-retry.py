#!/usr/bin/env python3
"""Check failed-commit preview refresh without losing the message (temporary repos)."""
import argparse
import os
from pathlib import Path
import runpy
import shutil
import tempfile

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session'];run=support['run']
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'));args=parser.parse_args();binary=args.binary.resolve();git=shutil.which('git')
 for cancel in [True,False]:
  with tempfile.TemporaryDirectory(prefix='gitview-retry-check-') as temp:
   temp=Path(temp);root=temp/'repo';root.mkdir();shim=temp/'bin';shim.mkdir();marker=temp/'delay'
   def g(*args):return run([git,*args],root)
   g('init','-qb','main');g('config','user.name','Test');g('config','user.email','test@example.com');g('config','commit.gpgsign','false');g('config','core.hooksPath','.git/hooks')
   (root/'sample.txt').write_text('base\n');g('add','--all');g('commit','-qm','base');(root/'sample.txt').write_text('staged\n');g('add','--all')
   hook=root/'.git/hooks/pre-commit';hook.write_text('#!/bin/sh\necho "fixture hook rejected" >&2\nexit 1\n');hook.chmod(0o700)
   (shim/'git').write_text('#!/bin/sh\nif [ -f "$PERF_DELAY_MARKER" ]; then sleep 0.25; fi\nexec "$PERF_REAL_GIT" "$@"\n');(shim/'git').chmod(0o700)
   env=dict(os.environ,TERM='xterm-256color',PATH=str(shim)+os.pathsep+os.environ['PATH'],PERF_DELAY_MARKER=str(marker),PERF_REAL_GIT=git)
   session=Session(binary,root,env,140,35)
   try:
    session.wait_for([b'GITVIEW']);session.receive(.8);os.write(session.master,b'c');session.wait_for([b'COMMIT MESSAGE']);os.write(session.master,b'perf: retained subject\tretained description\t');session.receive(.2)
    marker.touch();os.write(session.master,b'\r');session.wait_for([b'PREPARING COMMIT'])
    if cancel:os.write(session.master,b'\x1b')
    session.wait_for([b'COMMIT MESSAGE']);session.receive(.1);screen=session.screen.text();assert b'perf: retained subject' in screen and b'retained description' in screen,screen.decode().decode()
    assert b'fixture hook rejected' in screen,screen.decode()
    marker.unlink();session.receive(.5);assert g('rev-list','--count','HEAD')=='1'
    if cancel:
     assert b'COMMIT MESSAGE' in session.screen.text()
     assert b'perf: retained subject' in session.screen.text() and b'retained description' in session.screen.text()
     os.write(session.master,b'\r');session.wait_for([b'Commit preview unavailable']);assert g('rev-list','--count','HEAD')=='1'
     os.write(session.master,b'\x1b');session.wait_for([b'WORKTREE']);os.write(session.master,b'q');session.child.wait(timeout=3)
    else:
     hook.unlink();os.write(session.master,b'\r');session.wait_for([b'Commit created.']);assert g('rev-list','--count','HEAD')=='2';assert g('log','-1','--format=%s')=='perf: retained subject';assert g('log','-1','--format=%b')=='retained description';os.write(session.master,b'q');session.child.wait(timeout=3)
   finally:session.close()
 print('PASS: rejected hook preserved subject/body; refresh cancellation stayed dismissed; refreshed retry committed once after hook removal')
if __name__=='__main__':main()
