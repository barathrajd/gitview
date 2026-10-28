#!/usr/bin/env python3
"""Measure ordinary-diff RSS across 100 native PTY lifecycle cycles in a temporary repo."""
import argparse
import fcntl
import json
import os
from pathlib import Path
import runpy
import shutil
import signal
import struct
import tempfile
import termios

support = runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session, TerminalScreen, run, sample = (support[key] for key in ['Session', 'TerminalScreen', 'run', 'sample'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/gitview'))
    args = parser.parse_args()
    binary = args.binary.resolve()
    git = shutil.which('git')
    with tempfile.TemporaryDirectory(prefix='gitview-diff-memory-') as directory:
        root = Path(directory)
        def g(*arguments):
            return run([git, *arguments], root)
        g('init', '-qb', 'main')
        g('config', 'user.name', 'Performance fixture')
        g('config', 'user.email', 'perf@example.com')
        g('config', 'commit.gpgsign', 'false')
        path = root / 'sample.txt'
        path.write_text(('before ' + 'x' * 180 + '\n') * 2490)
        g('add', '--all')
        g('commit', '-qm', 'fixture')
        path.write_text(('after  ' + 'x' * 180 + '\n') * 2490)
        patch = g('diff', '--no-ext-diff')
        session = Session(binary, root, dict(os.environ, TERM='xterm-256color'), 140, 35)
        try:
            session.wait_for([b'GITVIEW'])
            session.receive(1)
            def resize(width, height):
                session.screen = TerminalScreen(width, height)
                fcntl.ioctl(session.master, termios.TIOCSWINSZ, struct.pack('HHHH', height, width, 0, 0))
                os.kill(session.child.pid, signal.SIGWINCH)
                session.receive(.06)
            def cycle():
                os.write(session.master, b'\r')
                session.wait_for([b'BEFORE'])
                for width, height in [(140, 35), (80, 24), (140, 35)]:
                    resize(width, height)
                    for key in [b'v', b'\t', b'v', b'\t']:
                        os.write(session.master, key)
                        session.receive(.05)
                os.write(session.master, b'\x1b')
                session.wait_for([b'WORKTREE'])
            for _ in range(10):
                cycle()
            session.receive(1)
            warm = sample(session.child.pid)[1]
            samples = []
            for index in range(100):
                cycle()
                if index % 10 == 9:
                    session.receive(.1)
                    rss = sample(session.child.pid)[1]
                    samples.append(rss)
                    print(f'cycle={index + 1} rss_kib={rss}', flush=True)
            session.receive(1)
            final = sample(session.child.pid)[1]
            growth = max(0, final - warm)
            budget = max(10240, warm // 10)
            result = dict(fixture='2490 replaced text lines', patch_bytes=len(patch.encode()), dimensions=['140x35', '80x24'], warm_cycles=10, measured_cycles=100, warm_rss_kib=warm, final_rss_kib=final, peak_sampled_settled_rss_kib=max(samples), growth_kib=growth, budget_kib=budget, passed=growth <= budget)
            print(json.dumps(result, indent=2), flush=True)
            assert g('diff', '--cached') == ''
            assert g('diff', '--no-ext-diff') == patch
            assert result['passed'], 'Native settled RSS exceeds the performance plan budget'
            os.write(session.master, b'q')
            session.child.wait(timeout=3)
        finally:
            session.close()


if __name__ == '__main__':
    main()
