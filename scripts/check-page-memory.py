#!/usr/bin/env python3
"""Measure settled RSS across 100 open/close cycles of the tag, object, and comparison pages (temporary repo)."""
import argparse
import json
import os
from pathlib import Path
import runpy
import shutil
import tempfile

support = runpy.run_path(str(Path(__file__).with_name('profile-tui.py')))
Session, run, sample = (support[key] for key in ['Session', 'run', 'sample'])


def build(root, git, tags):
    def g(*arguments, **kwargs):
        return run([git, *arguments], root)
    g('init', '-qb', 'main')
    g('config', 'user.name', 'Performance fixture')
    g('config', 'user.email', 'perf@example.com')
    g('config', 'commit.gpgsign', 'false')
    path = root / 'sample.txt'
    path.write_text(('before ' + 'x' * 180 + '\n') * 2490)
    g('add', '--all')
    g('commit', '-qm', 'base')
    path.write_text(('after  ' + 'x' * 180 + '\n') * 2490)
    g('add', '--all')
    g('commit', '-qm', 'large change')
    import subprocess
    refs = ''.join(f'create refs/tags/perf-{index:05} HEAD\n' for index in range(tags))
    subprocess.run([git, 'update-ref', '--stdin'], cwd=root, input=refs.encode(), check=True, capture_output=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path('target/release/gitview'))
    parser.add_argument('--tags', type=int, default=2000)
    args = parser.parse_args()
    binary = args.binary.resolve()
    git = shutil.which('git')
    with tempfile.TemporaryDirectory(prefix='gitview-page-memory-') as directory:
        root = Path(directory)
        build(root, git, args.tags)
        session = Session(binary, root, dict(os.environ, TERM='xterm-256color'), 140, 35)

        def press(key, needle, timeout=15):
            os.write(session.master, key)
            session.wait_for([needle], timeout)

        def tags():
            press(b'b', b'Local branches')
            press(b't', b'perf-00000')
            for _ in range(20):
                os.write(session.master, b'j')
            session.receive(.05)
            press(b'\x1b', b'Local branches')
            press(b'\x1b', b'GITVIEW')

        def object_page():
            press(b'o', b'INSPECT GIT OBJECT')
            press(b'\r', b'OBJECT DESCRIPTION')
            for _ in range(20):
                os.write(session.master, b'j')
            session.receive(.05)
            press(b'\x1b', b'INSPECT GIT OBJECT')
            press(b'\x1b', b'GITVIEW')

        def comparison():
            press(b'D', b'COMPARE COMMIT SNAPSHOTS')
            press(b'\r', b'COMPARISON REVIEW')
            for key in [b'v', b'\t', b'v', b'\t']:
                os.write(session.master, key)
                session.receive(.05)
            press(b'\x1b', b'COMPARE COMMIT SNAPSHOTS')
            press(b'\x1b', b'GITVIEW')

        results = {}
        try:
            session.wait_for([b'GITVIEW'])
            session.receive(1)
            os.write(session.master, b'\t')
            session.receive(.3)
            for name, cycle in [('tags', tags), ('object', object_page), ('comparison', comparison)]:
                for _ in range(10):
                    cycle()
                session.receive(1)
                warm = sample(session.child.pid)[1]
                peak = warm
                for index in range(100):
                    cycle()
                    if index % 10 == 9:
                        session.receive(.1)
                        peak = max(peak, sample(session.child.pid)[1])
                session.receive(1)
                final = sample(session.child.pid)[1]
                growth = max(0, final - warm)
                budget = max(10240, warm // 10)
                results[name] = dict(warm_rss_kib=warm, final_rss_kib=final, peak_sampled_rss_kib=peak, growth_kib=growth, budget_kib=budget, passed=growth <= budget)
                print(name, json.dumps(results[name]), flush=True)
            print(json.dumps(dict(tags=args.tags, warm_cycles=10, measured_cycles=100, results=results), indent=2), flush=True)
            assert all(item['passed'] for item in results.values()), 'Settled page RSS exceeds the performance plan budget'
            os.write(session.master, b'q')
            session.child.wait(timeout=3)
        finally:
            session.close()


if __name__ == '__main__':
    main()
