#!/usr/bin/env python3
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent / "backend"
QUERY_FN = r"query(?:_as|_scalar|_file|_file_as|_file_scalar)?(?:_with)?|raw_sql"
QUERY_NAME = re.compile(rf"^(?:{QUERY_FN})$")
CRATE_ALIAS = re.compile(r"\buse\s+sqlx\s+as\s+([A-Za-z_][A-Za-z0-9_]*)\s*;")
SINGLE_IMPORT = re.compile(rf"\buse\s+sqlx::({QUERY_FN})(?:\s+as\s+([A-Za-z_][A-Za-z0-9_]*))?\s*;")
GROUP_IMPORT = re.compile(r"\buse\s+sqlx::\{([^}]*)\}\s*;", re.S)
OPEN = re.compile(r"\s*!?\s*\(")


def call_matcher(text: str) -> re.Pattern:
    crates = {"sqlx"} | set(CRATE_ALIAS.findall(text))
    bare = {alias or name for name, alias in SINGLE_IMPORT.findall(text)}
    for group in GROUP_IMPORT.findall(text):
        for item in group.split(","):
            parts = item.split()
            if not parts or not QUERY_NAME.match(parts[0]):
                continue
            bare.add(parts[2] if len(parts) == 3 and parts[1] == "as" else parts[0])
    qualified = rf"(?:{'|'.join(sorted(map(re.escape, crates)))})::(?:{QUERY_FN})"
    if bare:
        unqualified = rf"(?<![A-Za-z0-9_:.])(?:{'|'.join(sorted(map(re.escape, bare)))})(?![A-Za-z0-9_])"
        return re.compile(rf"(?:{qualified}|{unqualified})")
    return re.compile(qualified)
LITERAL = re.compile(r'\s*(?:[A-Za-z_][A-Za-z0-9_:]*(?:<[^"()]*>)?\s*,\s*)?(?:r#*)?"')


def skip_turbofish(text: str, at: int) -> int:
    if not text.startswith("::<", at):
        return at
    depth = 0
    for index in range(at + 2, len(text)):
        if text[index] == "<":
            depth += 1
        elif text[index] == ">":
            depth -= 1
            if depth == 0:
                return index + 1
    return at


def calls(text: str):
    for name in call_matcher(text).finditer(text):
        after = skip_turbofish(text, name.end())
        opening = OPEN.match(text, after)
        if opening:
            yield name.start(), opening.end()

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
    ('sqlx::query_scalar::<_, Option<i64>>(statement)', 1),
    ('sqlx::query_scalar::<_, Option<i64>>("SELECT 1")', 0),
    ('sqlx::query_as::<_, Vec<Option<i64>>>(&sql)', 1),
    ('sqlx::query_as!(models::Row, "SELECT 1")', 0),
    ('sqlx::query_as!(crate::dto::Row<Inner>,\n    "SELECT 1",\n    id)', 0),
    ('use sqlx::query;\nquery(&sql)', 1),
    ('use sqlx::query;\nquery!("SELECT 1")', 0),
    ('use sqlx::{query_as as rows, Row};\nrows::<_, R>(&sql)', 1),
    ('use sqlx::{\n    Pool,\n    query_scalar,\n};\nquery_scalar("SELECT 1")', 0),
    ('use sqlx as db;\ndb::query(sql)', 1),
    ('use sqlx as db;\ndb::query("SELECT 1")', 0),
    ('use sqlx::Row;\nlet query = build();\nquery(&sql)', 0),
    ('self.query(&sql)', 0),
]


def violations_in(text: str):
    for start, end in calls(text):
        tail = text[end:end + 200]
        if LITERAL.match(tail):
            continue
        line = text.count("\n", 0, start) + 1
        yield line, text[start:end + 60].split("\n")[0]


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
