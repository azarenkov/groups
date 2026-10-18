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
EXECUTOR = re.compile(r"\.(?:execute|execute_many|fetch|fetch_many|fetch_all|fetch_one|fetch_optional|prepare|prepare_with|describe)\s*\(")
DYNAMIC_SQL = re.compile(
    r"^\s*&?\s*(?:format!|concat!)|\.as_str\(\)\s*$|^\s*&?\s*(?:mut\s+)?[A-Za-z_]*(?:sql|query|statement|stmt)[A-Za-z0-9_]*\s*$",
    re.I,
)


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
    ('/// Avoid `sqlx::query(statement)` here\nfn f() {}', 0),
    ('let doc = "sqlx::query(statement)";', 0),
    ('/* sqlx::query(&sql) */ let x = 1;', 0),
    ('sqlx::query(r#"SELECT \'a\' FROM t WHERE x = \'"\'"#)', 0),
    ('sqlx::query(&sql) // sqlx::query("x")', 1),
    ("let c = '\"'; sqlx::query(&sql)", 1),
    ('pool.execute("CREATE TABLE t (id int)").await?', 0),
    ('sqlx::query("SELECT 1").fetch_all(&pool).await?', 0),
    ('query.execute(&mut *tx).await?', 0),
    ('rows.fetch_one(&mut conn).await?', 0),
    ('pool.execute(&format!("CREATE TABLE {}", name)).await?', 1),
    ('conn.fetch_all(sql.as_str()).await?', 1),
    ('conn.execute(&statement).await?', 1),
    ('tx.execute(query_text).await?', 1),
]


def mask_non_code(text: str) -> str:
    out = list(text)
    i, n = 0, len(text)

    def blank(start: int, stop: int) -> None:
        for k in range(start, stop):
            if out[k] != "\n":
                out[k] = " "

    while i < n:
        c = text[i]
        if text.startswith("//", i):
            stop = text.find("\n", i)
            stop = n if stop == -1 else stop
            blank(i, stop)
            i = stop
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == "r" and re.match(r'r#*"', text[i:i + 8]):
            hashes = len(text[i + 1:i + 8].split('"')[0])
            close = '"' + "#" * hashes
            body = i + 1 + hashes + 1
            stop = text.find(close, body)
            stop = n if stop == -1 else stop
            blank(body, stop)
            i = stop + len(close)
        elif c == '"' and not (i > 0 and text[i - 1].isalnum()):
            j = i + 1
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            blank(i + 1, j)
            i = j + 1
        elif c == "'" and i + 2 < n and (text[i + 2] == "'" or (text[i + 1] == "\\" and text.find("'", i + 2) != -1 and text.find("'", i + 2) - i <= 8)):
            stop = text.find("'", i + 2) if text[i + 1] == "\\" else i + 2
            blank(i + 1, stop)
            i = stop + 1
        else:
            i += 1
    return "".join(out)


def first_argument(text: str, start: int) -> str:
    depth, i = 0, start
    while i < len(text):
        c = text[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            if depth == 0:
                return text[start:i]
            depth -= 1
        elif c == "," and depth == 0:
            return text[start:i]
        i += 1
    return text[start:]


def executor_violations(masked: str):
    for call in EXECUTOR.finditer(masked):
        argument = first_argument(masked, call.end())
        if DYNAMIC_SQL.search(argument):
            yield call.start()


def violations_in(text: str):
    masked = mask_non_code(text)
    found = []
    for start, end in calls(masked):
        tail = masked[end:end + 200]
        if not LITERAL.match(tail):
            found.append((start, end))
    for start in executor_violations(masked):
        found.append((start, start + 1))
    for start, end in sorted(found):
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
