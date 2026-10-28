#!/usr/bin/env python3
"""Profile native Unix TUI startup, idle CPU/RSS, and delayed commit preview.

Use an explicitly built binary. Fixtures and a Git delay shim are temporary.
Other workflow measurements remain tracked in the performance plan.
"""
import argparse
import hashlib
import fcntl
import json
import os
from pathlib import Path
import platform
import pty
import re
import select
import shutil
import struct
import subprocess
import tempfile
import termios
import time


def run(argv, cwd=None):
    return subprocess.run(argv, cwd=cwd, check=True, capture_output=True).stdout.decode().strip()


def fixture(root, kind, git):
    counts = {'small': (50, 100, 10, 0), 'medium': (200, 1000, 100, 1000), 'stress': (200, 10000, 10000, 1000)}
    commits, files, changed, refs = counts[kind]
    run([git, 'init', '-q', '-b', 'main'], root)
    for name, value in [('user.name', 'Performance Fixture'), ('user.email', 'perf@example.com'), ('commit.gpgsign', 'false')]:
        run([git, 'config', name, value], root)
    for index in range(files):
        (root / f'file-{index:05}.txt').write_text('base\n')
    run([git, 'add', '--all'], root)
    run([git, 'commit', '-qm', 'base'], root)
    for index in range(commits - 1):
        run([git, 'commit', '--allow-empty', '-qm', f'fixture {index}'], root)
    if refs:
        data=''.join(f'create refs/heads/perf-{index:04} HEAD\n' for index in range(refs))
        subprocess.run([git, 'update-ref', '--stdin'], cwd=root, input=data.encode(), check=True, capture_output=True)
    for index in range(changed):
        (root / f'file-{index:05}.txt').write_text('changed\n')
    run([git, 'add', '--all'], root)
    return dict(commits=commits, tracked_files=files, staged_paths=changed, extra_refs=refs)


class Session:
    def __init__(self, binary, root, env, width, height):
        self.master, slave = pty.openpty()
        fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
        self.child = subprocess.Popen([str(binary)], cwd=root, stdin=slave, stdout=slave, stderr=slave, env=env)
        os.close(slave)

    def receive(self, seconds):
        output=b''; deadline=time.monotonic()+seconds
        while time.monotonic()<deadline:
            if select.select([self.master], [], [], min(.01, max(0, deadline-time.monotonic())))[0]:
                try:
                    part=os.read(self.master, 65536)
                except OSError:
                    break
                if not part:
                    break
                output+=part
        return output

    def wait_for(self, needles, timeout=10):
        output=b''; start=time.monotonic()
        while time.monotonic()-start<timeout:
            output+=self.receive(.01)
            plain=re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]', b'', output)
            if any(needle in plain for needle in needles):
                return (time.monotonic()-start)*1000
        raise RuntimeError(f'Timed out waiting for {needles!r}; tail={output[-400:]!r}')

    def close(self):
        if self.child.poll() is None:
            self.child.kill()
        self.child.wait()
        os.close(self.master)


def sample(pid):
    fields=run(['ps', '-p', str(pid), '-o', 'time=', '-o', 'rss=']).split()
    values=list(map(float, fields[0].split(':')))
    cpu=0
    for value in values:
        cpu=cpu*60+value
    return cpu, int(fields[1])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/gitview'))
    parser.add_argument('--fixture', choices=['small', 'medium', 'stress'], default='small')
    parser.add_argument('--samples', type=int, default=20)
    parser.add_argument('--idle-seconds', type=float, default=30)
    parser.add_argument('--width', type=int, default=140)
    parser.add_argument('--height', type=int, default=35)
    parser.add_argument('--action', choices=['commit','amend'], default='commit')
    parser.add_argument('--check-idle', action='store_true', help='Require idle CPU <=2% and <=256 output bytes')
    parser.add_argument('--check-preview', action='store_true', help='Fail if any sampled preview feedback/cancel exceeds 100 ms')
    parser.add_argument('--output', type=Path)
    args=parser.parse_args()
    if args.samples<1 or args.idle_seconds<=0 or args.width<40 or args.height<12:
        parser.error('Use samples >=1, idle-seconds >0, width >=40, and height >=12')
    binary=args.binary.resolve()
    if not binary.is_file():
        parser.error('Build the requested binary before profiling')
    git=shutil.which('git')
    if not git:
        parser.error('Git is required')
    result={'binary':str(binary), 'binary_mtime':binary.stat().st_mtime, 'platform':platform.platform(), 'architecture':platform.machine(), 'git':run([git,'--version']), 'rust':run(['rustc','--version']), 'revision':run([git,'rev-parse','HEAD']), 'terminal':[args.width,args.height], 'samples':args.samples, 'build_profile':'caller-selected; use cargo build --release --locked', 'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(), 'worktree_dirty':bool(run([git,'status','--porcelain'])), 'action':args.action, 'cancel_timing':'after first loading feedback or form; inspect feedback latency to identify blocking', 'metrics':{}, 'unmeasured':['diff first-use and switching','navigation and large-list actions','path/ref previews','editor/hook/signing response','Git-child CPU','memory plateau cycles','Linux evidence if running on macOS']}
    with tempfile.TemporaryDirectory(prefix='gitview-perf-') as temporary:
        temp=Path(temporary);root=temp/'repo';root.mkdir()
        result['fixture']=fixture(root,args.fixture,git)
        shim=temp/'bin';shim.mkdir();marker=temp/'delay'
        (shim/'git').write_text('#!/bin/sh\nif [ -f "$PERF_DELAY_MARKER" ]; then\ncase "$*" in *"status --porcelain"*|*"diff --cached --raw"*) sleep 0.25;; esac\nfi\nexec "$PERF_REAL_GIT" "$@"\n')
        (shim/'git').chmod(0o700)
        env=dict(os.environ, TERM='xterm-256color', PATH=str(shim)+os.pathsep+os.environ['PATH'], PERF_DELAY_MARKER=str(marker), PERF_REAL_GIT=git)
        starts=[];feedback=[];cancel=[]
        for index in range(args.samples):
            before=time.monotonic();session=Session(binary,root,env,args.width,args.height)
            try:
                session.wait_for([b'GITVIEW'])
                starts.append((time.monotonic()-before)*1000)
                session.receive(.8)
                if index==0:
                    cpu0,rss0=sample(session.child.pid);start=time.monotonic();idle_output=session.receive(args.idle_seconds);duration=time.monotonic()-start;cpu1,rss1=sample(session.child.pid)
                    result['metrics']['idle']={'wall_seconds':duration,'process_cpu_seconds':cpu1-cpu0,'process_cpu_percent_one_core':100*(cpu1-cpu0)/duration,'rss_kib_before':rss0,'rss_kib_after':rss1,'excludes_git_child_cpu':True,'terminal_output_bytes':len(idle_output)}
                marker.touch();os.write(session.master,b'c' if args.action=='commit' else b'a')
                feedback.append(session.wait_for([b'PREPARING COMMIT',b'COMMIT MESSAGE'] if args.action=='commit' else [b'PREPARING AMEND',b'AMEND MESSAGE']))
                os.write(session.master,b'\x1b');cancel.append(session.wait_for([b'WORKTREE']))
                # Let any cancelled read finish; a late result must not reopen the form.
                late=session.receive(1)
                plain=re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]',b'',late)
                if b'COMMIT MESSAGE' in plain or b'AMEND MESSAGE' in plain:
                    raise RuntimeError('A cancelled preview reopened the commit form')
                marker.unlink();os.write(session.master,b'q');session.child.wait(timeout=3)
            finally:
                marker.unlink(missing_ok=True);session.close()
        def summary(values):
            ordered=sorted(values)
            return {'samples_ms':values,'p50_ms':ordered[len(ordered)//2],'p95_ms':ordered[max(0, __import__('math').ceil(.95*len(ordered))-1)]}
        result['metrics'].update(startup_first_frame=summary(starts),slow_git_preview_feedback=summary(feedback),slow_git_preview_cancel=summary(cancel))
        result['injected_git_delay_ms_per_matching_call']=250
    text=json.dumps(result,indent=2)
    if args.output:
        args.output.write_text(text+'\n')
    print(text)
    if args.check_idle and (result['metrics']['idle']['process_cpu_percent_one_core']>2 or result['metrics']['idle']['terminal_output_bytes']>256):
        raise SystemExit('FAIL: idle CPU/output exceeded the budget')
    if args.check_preview and max(feedback+cancel)>100:
        raise SystemExit('FAIL: preview response/cancel exceeded the 100 ms target')

if __name__=='__main__':
    main()
