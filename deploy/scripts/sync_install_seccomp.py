"""Keep the chrome-seccomp.json copy in docs/self-hosting/quick-start.md in step with deploy/docker.

Usage:
  python3 deploy/scripts/sync_install_seccomp.py          # rewrite the docs block
  python3 deploy/scripts/sync_install_seccomp.py --check  # exit 1 when they differ (CI)
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PROFILE = ROOT / "deploy/docker/chrome-seccomp.json"
DOC = ROOT / "docs/self-hosting/quick-start.md"
BLOCK = re.compile(r"(<!-- seccomp:start -->\n).*?(<!-- seccomp:end -->)", re.S)


def render(profile: dict) -> str:
    """One line per syscall rule, so the block stays short enough to copy."""
    head = {k: v for k, v in profile.items() if k != "syscalls"}
    lines = ["{"]
    for key, value in head.items():
        lines.append(f"  {json.dumps(key)}: {json.dumps(value)},")
    lines.append('  "syscalls": [')
    rules = [f"    {json.dumps(rule)}" for rule in profile["syscalls"]]
    lines.append(",\n".join(rules))
    lines.append("  ]")
    lines.append("}")
    return "\n".join(lines)


def main() -> int:
    profile = json.loads(PROFILE.read_text())
    doc = DOC.read_text()
    match = BLOCK.search(doc)
    if not match:
        print(f"{DOC}: seccomp markers not found", file=sys.stderr)
        return 1
    rendered = render(profile)
    assert json.loads(rendered) == profile
    block = f"```json\n{rendered}\n```\n"
    current = re.sub(r"^```json\n|```\n$", "", doc[match.end(1) : match.start(2)])
    if "--check" in sys.argv:
        try:
            same = json.loads(current) == profile
        except json.JSONDecodeError:
            same = False
        if not same:
            print(
                "docs/self-hosting/quick-start.md seccomp block differs from deploy/docker/chrome-seccomp.json; "
                "run python3 deploy/scripts/sync_install_seccomp.py",
                file=sys.stderr,
            )
            return 1
        return 0
    DOC.write_text(doc[: match.end(1)] + block + doc[match.start(2) :])
    return 0


if __name__ == "__main__":
    sys.exit(main())
