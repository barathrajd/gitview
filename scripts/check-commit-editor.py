#!/usr/bin/env python3
"""Check that saving a commit message in the external editor commits without a second Enter (temporary repos)."""
import argparse
import os
from pathlib import Path
import runpy
import shutil
import tempfile

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session'];run=support['run']
def wait_for(session,needles,timeout=10):
 # Answer cursor-position queries like a real terminal after the editor returns.
 import time
 start=time.monotonic()
 while time.monotonic()-start<timeout:
  if b'\x1b[6n' in session.receive(.02):os.write(session.master,b'\x1b[1;1R')
  if any(needle in session.screen.text() for needle in needles):return
 raise RuntimeError(f'Timed out waiting for {needles!r}')
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'));args=parser.parse_args();binary=args.binary.resolve();git=shutil.which('git')
 for saves in [True,False]:
  with tempfile.TemporaryDirectory(prefix='gitview-editor-check-') as temp:
   temp=Path(temp);root=temp/'repo';root.mkdir()
   def g(*args):return run([git,*args],root)
   g('init','-qb','main');g('config','user.name','Test');g('config','user.email','test@example.com');g('config','commit.gpgsign','false')
   (root/'sample.txt').write_text('base\n');g('add','--all');g('commit','-qm','base');(root/'sample.txt').write_text('staged\n');g('add','--all')
   editor=temp/'editor.sh'
   editor.write_text('#!/bin/sh\nprintf "feat: saved in editor\\n\\nbody from editor\\n" > "$1"\n' if saves else '#!/bin/sh\nexit 0\n');editor.chmod(0o700)
   env=dict(os.environ,TERM='xterm-256color',GIT_EDITOR=str(editor))
   session=Session(binary,root,env,140,35)
   try:
    session.wait_for([b'GITVIEW']);session.receive(.8);os.write(session.master,b'c');session.wait_for([b'COMMIT MESSAGE']);session.receive(.5)
    os.write(session.master,b'draft subject');session.receive(.2);os.write(session.master,b'\x05')
    if saves:
     wait_for(session,[b'Commit created.']);assert g('rev-list','--count','HEAD')=='2';assert g('log','-1','--format=%s')=='feat: saved in editor';assert g('log','-1','--format=%b')=='body from editor'
     os.write(session.master,b'q');session.child.wait(timeout=3)
    else:
     wait_for(session,[b'review the message before committing']);session.receive(.5);assert g('rev-list','--count','HEAD')=='1';assert b'draft subject' in session.screen.text(),session.screen.text().decode()
     os.write(session.master,b'\x1b');session.wait_for([b'WORKTREE']);os.write(session.master,b'q');session.child.wait(timeout=3)
   finally:session.close()
 print('PASS: an edited editor save committed once without a second Enter; an unchanged save stayed in the composer without committing')
if __name__=='__main__':main()
