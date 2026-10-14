#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = []
# ///
"""Sign off the pushed HEAD commit after its local checks passed.

No Actions workflow tests a change before it merges. Run the local checks in
AGENTS.md on this exact commit first; this script runs none of them. It refuses local
changes and a commit GitHub does not hold, then uses the GitHub CLI to set the
`signoff` commit status that main's branch protection requires. The status
records who signed off and the checks you name.
"""
import argparse
import subprocess

REPOSITORY = 'Reidond/kedra'
CONTEXT = 'signoff'
# GitHub rejects longer commit status descriptions.
DESCRIPTION_LIMIT = 140


def capture(*arguments):
    result = subprocess.run(arguments, check=False, stdin=subprocess.DEVNULL, capture_output=True, text=True)
    if result.returncode:
        raise SystemExit(f'signoff: `{" ".join(arguments[:3])}` failed: {result.stderr.strip()}')
    return result.stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--checks', required=True,
                        help='The local checks that passed on this commit, for example "fmt clippy e2e build container"')
    args = parser.parse_args()
    if not args.checks.strip() or len(args.checks) > DESCRIPTION_LIMIT:
        raise SystemExit(f'signoff: --checks must name the passed checks in at most {DESCRIPTION_LIMIT} characters')
    if capture('git', 'status', '--porcelain'):
        raise SystemExit('signoff: commit or remove every local change first; a sign-off covers only the pushed commit')
    head = capture('git', 'rev-parse', '--verify', 'HEAD^{commit}')
    # Fails unless the exact commit is pushed: a sign-off names a commit others can check out.
    capture('gh', 'api', f'repos/{REPOSITORY}/commits/{head}', '--jq', '.sha')
    capture('gh', 'api', '--method', 'POST', f'repos/{REPOSITORY}/statuses/{head}',
            '-f', 'state=success', '-f', f'context={CONTEXT}', '-f', f'description={args.checks}')
    print(f'signoff: {CONTEXT} set on {head}')


if __name__ == '__main__':
    main()
