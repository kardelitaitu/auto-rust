from __future__ import annotations

import re
from pathlib import Path


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    coverage = root / "coverage.txt"
    if not coverage.exists():
        raise SystemExit(f"coverage file not found: {coverage}")

    text = coverage.read_text(encoding="utf-8", errors="ignore")
    pattern = re.compile(
        r"(?:\\|/|^)(?:task[/\\]dsl[/\\])?([^ \t\r\n]+\.rs)[ \t]+([0-9]+)[ \t]+([0-9]+)[ \t]+([0-9.]+)%"
    )
    shown = 0
    for line in text.splitlines():
        if any(name in line for name in ("parser.rs", "evaluator.rs", "executor.rs", "control_flow.rs", "api.rs", "debug.rs")):
            m = pattern.search(line)
            if m:
                print(f"{m.group(1).split('/')[-1]}: {m.group(4)}%")
                shown += 1
    return 0 if shown else 2


if __name__ == "__main__":
    raise SystemExit(main())
