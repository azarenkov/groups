#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "backend"
CALL = re.compile(r"sqlx::(?:query(?:_as|_scalar|_file|_file_as|_file_scalar)?(?:_with)?|raw_sql)(?:::<[^>]*>)?\s*!?\s*\(")
LITERAL = re.compile(r'\s*(?:[A-Za-z_][A-Za-z0-9_]*\s*,\s*)?(?:r#*)?"')

SELF_TEST = [
    ('sqlx::query("SELECT 1").fetch_one(pool)', 0),
    ('sqlx::query!(r#"SELECT $1"#, id)', 0),
    ('sqlx::query_as!(Row,\n    "SELECT 1",\n    id)', 0),
    ('sqlx::query_scalar::<_, i64>("SELECT 1")', 0),
    ('sqlx::query(&format!("SELECT {}", table))', 1),
    ('sqlx::query_as(&sql)', 1),
    ('sqlx::query_scalar(sql.as_str())', 1),
    ('sqlx::query_with(&sql, args)', 1),
    ('sqlx::query_as_with::<_, Row, _>(&sql, args)', 1),
    ('sqlx::query_scalar_with(statement, args)', 1),
    ('sqlx::raw_sql(&script)', 1),
]


def violations_in(text: str):
    for match in CALL.finditer(text):
        tail = text[match.end():match.end() + 200]
        if LITERAL.match(tail):
            continue
        line = text.count("\n", 0, match.start()) + 1
        yield line, text[match.start():match.end() + 60].split("\n")[0]


def self_test() -> int:
    failures = [snippet for snippet, expected in SELF_TEST if len(list(violations_in(snippet))) != expected]
    if failures:
        print("sql check self-test failed for:\n" + "\n".join(failures), file=sys.stderr)
        return 1
    print("sql check self-test: ok")
    return 0


def violations(path: pathlib.Path):
    yield from violations_in(path.read_text(encoding="utf-8"))


def main() -> int:
    if "--self-test" in sys.argv[1:]:
        return self_test()
    if self_test() != 0:
        return 1
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
