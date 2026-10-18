#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "backend"
CALL = re.compile(r"sqlx::query(?:_as|_scalar|_file|_file_as)?(?:::<[^>]*>)?\s*!?\s*\(")
LITERAL = re.compile(r'\s*(?:[A-Za-z_][A-Za-z0-9_]*\s*,\s*)?(?:r#*)?"')


def violations(path: pathlib.Path):
    text = path.read_text(encoding="utf-8")
    for match in CALL.finditer(text):
        tail = text[match.end():match.end() + 200]
        if LITERAL.match(tail):
            continue
        line = text.count("\n", 0, match.start()) + 1
        yield line, text[match.start():match.end() + 60].split("\n")[0]


def main() -> int:
    found = []
    for path in sorted(ROOT.rglob("*.rs")):
        if "/target/" in str(path):
            continue
        for line, snippet in violations(path):
            found.append(f"{path.relative_to(ROOT.parent)}:{line}: {snippet}")
    if found:
        print("SQL must be a string literal passed to sqlx (no format!, concatenation or variables); see issue #399", file=sys.stderr)
        print("\n".join(found), file=sys.stderr)
        return 1
    print("sql check: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
