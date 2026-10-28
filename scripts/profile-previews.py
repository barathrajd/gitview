#!/usr/bin/env python3
"""Measure read-only preview feedback/cancellation with delayed Git; never confirm writes."""
import argparse
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import tempfile

support=runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session=support['Session']
run=support['run']
CASES={
 'discard':(b'',b'd',[b'CONFIRM DISCARD'],b'WORKTREE'),
 'untrack':(b'',b'x',[b'CONFIRM STOP TRACKING'],b'WORKTREE'),
 'remove':(b'R',b'clean.txt\r',[b'CONFIRM TRACKED PATH REMOVAL'],b'REMOVE TRACKED PATH'),
 'move':(b'M',b'clean.txt\rnew.txt\r',[b'CONFIRM PATH MOVE'],b'MOVE TRACKED PATH'),
 'branch-rename':(b'b',b'r',[b'RENAME LOCAL BRANCH'],b'Local branches'),
 'branch-delete':(b'b',b'd',[b'CONFIRM BRANCH OPERATION'],b'Local branches'),
 'previous':(b'b',b'-',[b'CONFIRM BRANCH OPERATION'],b'Local branches'),
 'ignore':(b'',b'i',[b'No ignore rule',b'Ignored by'],b'WORKTREE'),
 'tag-create':([b'b',b't',b'c'],b'v-test\r',[b'CONFIRM TAG OPERATION'],b'CREATE LIGHTWEIGHT TAG'),
 'tag-verify':([b'b',b't'],b'v',[b'Git signature verification'],b'LOCAL TAGS'),
 'ignore-path':(b'I',b'clean.txt\r',[b'No ignore rule',b'Ignored by'],b'INSPECT IGNORE RULE'),
}

def main():
 parser=argparse.ArgumentParser(description=__doc__)
 parser.add_argument('--binary',type=Path,default=Path('target/release/gitview'))
 parser.add_argument('--samples',type=int,default=3)
 parser.add_argument('--case',choices=['all',*CASES],default='all')
 parser.add_argument('--check',action='store_true')
 parser.add_argument('--output',type=Path)
 args=parser.parse_args()
 if args.samples<1:parser.error('samples must be positive')
 git=shutil.which('git');binary=args.binary.resolve()
 if not git or not binary.is_file():parser.error('Git and a built binary are required')
 results={}
 with tempfile.TemporaryDirectory(prefix='gitview-preview-perf-') as temp:
  temp=Path(temp);root=temp/'repo';root.mkdir();shim=temp/'bin';shim.mkdir();marker=temp/'delay'
  def g(*args):return run([git,*args],root)
  g('init','-qb','main');g('config','user.name','Test');g('config','user.email','test@example.com');g('config','commit.gpgsign','false')
  (root/'dirty.txt').write_text('base\n');(root/'clean.txt').write_text('clean\n');g('add','--all');g('commit','-qm','base');g('switch','-qc','topic');g('switch','-q','main');g('-c','tag.gpgSign=false','tag','--no-sign','v-fixture');(root/'dirty.txt').write_text('changed\n')
  before=(g('status','--porcelain=v2'),g('show-ref'))
  (shim/'git').write_text('#!/bin/sh\nif [ -f "$PERF_DELAY_MARKER" ]; then sleep 0.25; fi\nexec "$PERF_REAL_GIT" "$@"\n');(shim/'git').chmod(0o700)
  env=dict(os.environ,TERM='xterm-256color',PATH=str(shim)+os.pathsep+os.environ['PATH'],PERF_DELAY_MARKER=str(marker),PERF_REAL_GIT=git)
  for name,(setup,trigger,completion,returned)in CASES.items():
   if args.case!='all' and name!=args.case:continue
   measurements=[]
   for _ in range(args.samples):
    session=Session(binary,root,env,140,35)
    try:
     session.wait_for([b'GITVIEW']);session.receive(.8)
     if setup:
      for keys in (setup if isinstance(setup,list) else [setup]):
       os.write(session.master,keys);session.receive(.4)
     # Fill path fields before timing their submit key.
     if b'\r' in trigger:
      os.write(session.master,trigger[:-1]);session.receive(.1);key=trigger[-1:]
     else:key=trigger
     marker.touch();os.write(session.master,key)
     feedback=session.wait_for([b'PREPARING PREVIEW',*completion],timeout=15)
     os.write(session.master,b'\x1b');cancel=session.wait_for([returned]);marker.unlink()
     late=session.receive(.8)
     plain=session.screen.text()
     if b'CONFIRM ' in plain or b'RENAME LOCAL BRANCH' in plain or b'Git signature verification' in plain or (name.startswith('ignore') and (b'No ignore rule' in plain or b'Ignored by' in plain)):raise RuntimeError(f'{name}: cancelled preview reopened')
     measurements.append({'feedback_ms':feedback,'cancel_ms':cancel})
    finally:
     marker.unlink(missing_ok=True);session.close()
   results[name]=measurements
  assert (g('status','--porcelain=v2'),g('show-ref'))==before,'Read-only preview check changed repository state'
 report={'binary':str(args.binary),'build_profile':'caller-selected; use cargo build --release --locked for native measurements','revision':run([git,'rev-parse','HEAD']),'worktree_dirty':bool(run([git,'status','--porcelain'])),'platform':support['platform'].platform(),'git':run([git,'--version']),'rust':run(['rustc','--version']),'binary_sha256':support['hashlib'].sha256(binary.read_bytes()).hexdigest(),'terminal':[140,35],'delay_ms_per_git_command':250,'samples_per_case':args.samples,'measurements':results,'scope':'temporary two-file repository; preview feedback and cancel, not mutation completion; 10 ms read windows'}
 text=json.dumps(report,indent=2)
 if args.output:args.output.write_text(text+'\n')
 print(text)
 if args.check and any(sample[key]>100 for samples in results.values() for sample in samples for key in ['feedback_ms','cancel_ms']):raise SystemExit('FAIL: preview feedback/cancel exceeded 100 ms')

if __name__=='__main__':main()
