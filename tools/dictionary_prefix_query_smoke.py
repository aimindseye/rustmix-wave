#!/usr/bin/env python3
"""Smoke-test Rustmix dictionary prefix-shard lookup behavior on DICT folders.

Unlike the on-device app, this tool reports missing indexed shards clearly and
keeps scanning other candidate shards. It never emits a Python traceback for a
normal missing DATA/*.JSN condition.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Iterable

SHARD_LIMIT = 16 * 1024


def normalize_query(query: str) -> str:
    query = str(query or "").strip().upper()
    while query.endswith(("*", "_")):
        query = query[:-1].strip()
    return query


def alpha_prefix(query: str) -> str:
    q = normalize_query(query)
    if not q:
        return ""
    if not q[0].isalpha():
        return "OTHERS"
    out: list[str] = []
    for ch in q:
        if not ch.isalpha():
            break
        out.append(ch)
        if len(out) >= 5:
            break
    return "".join(out) or "OTHERS"


def shard_base(name: str) -> str:
    base = name.rstrip("0123456789")
    return base or name


def parse_index(dict_dir: Path) -> list[tuple[str, str]]:
    index_path = dict_dir / "INDEX.TXT"
    if not index_path.exists():
        raise FileNotFoundError(index_path)
    rows: list[tuple[str, str]] = []
    for line_no, raw in enumerate(index_path.read_text(encoding="utf-8").splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "|" not in line:
            raise ValueError(f"bad index line {line_no}: {line!r}")
        name, rel = [p.strip() for p in line.split("|", 1)]
        rows.append((name.upper(), rel))
    if not rows:
        raise ValueError("INDEX.TXT has no shard rows")
    return rows


def row_matches(name: str, prefix: str) -> bool:
    base = shard_base(name)
    if prefix == "OTHERS":
        return base == "OTHERS"
    return base.startswith(prefix) or prefix.startswith(base)


def candidate_rows(rows: Iterable[tuple[str, str]], query: str) -> list[tuple[str, str]]:
    prefix = alpha_prefix(query)
    candidates = [(name, rel) for name, rel in rows if row_matches(name, prefix)]
    return sorted(candidates, key=lambda r: (-len(shard_base(r[0])), r[0]))


def read_shard(dict_dir: Path, rel_path: str) -> tuple[dict[str, Any] | None, str | None]:
    path = Path(rel_path)
    if not path.is_absolute():
        path = dict_dir / path
    if not path.exists():
        return None, f"missing shard: {path}"
    data = path.read_bytes()
    if len(data) > SHARD_LIMIT:
        return None, f"shard exceeds {SHARD_LIMIT} bytes: {path} = {len(data)}"
    try:
        payload = json.loads(data.decode("utf-8"))
    except Exception as exc:  # noqa: BLE001 - smoke tool should report JSON failures.
        return None, f"invalid JSON in {path}: {exc}"
    if not isinstance(payload, dict):
        return None, f"shard must contain a JSON object: {path}"
    return payload, None


def lookup(dict_dir: Path, query: str, prefix_mode: bool = False) -> tuple[str, Any, str, list[str]]:
    q = normalize_query(query)
    rows = parse_index(dict_dir)
    warnings: list[str] = []
    matched_candidates = candidate_rows(rows, q)
    if not matched_candidates:
        raise LookupError(f"no index row matches prefix {alpha_prefix(q)!r}")

    for name, rel in matched_candidates:
        payload, warning = read_shard(dict_dir, rel)
        if warning:
            warnings.append(f"{name}: {warning}")
            continue
        assert payload is not None
        if q in payload and not prefix_mode:
            return q, payload[q], name, warnings
        if prefix_mode or len(q) <= 2:
            for key in sorted(payload):
                if key.startswith(q):
                    return key, payload[key], name, warnings

    # Exact lookup fallback: if the exact word was not present but the query is
    # a prefix, return the first prefix hit from any readable candidate shard.
    if not prefix_mode:
        for name, rel in matched_candidates:
            payload, warning = read_shard(dict_dir, rel)
            if warning:
                continue
            assert payload is not None
            for key in sorted(payload):
                if key.startswith(q):
                    return key, payload[key], name, warnings

    if warnings:
        raise LookupError(f"not found: {query!r}; skipped {len(warnings)} unreadable candidate shard(s)")
    raise LookupError(f"not found: {query!r}")


def build_sample_tree(root: Path) -> Path:
    dict_dir = root / "DICT"
    data_dir = dict_dir / "DATA"
    data_dir.mkdir(parents=True, exist_ok=True)
    (dict_dir / "INDEX.TXT").write_text(
        "GA|DATA/GA.JSN\nGRANA|DATA/GRANA.JSN\nGO|DATA/GO.JSN\nABAND1|DATA/ABAND1.JSN\nABAND2|DATA/ABAND2.JSN\n",
        encoding="utf-8",
    )
    (data_dir / "GA.JSN").write_text(json.dumps({"GAME": "play", "GARDEN": "yard"}, separators=(",", ":")), encoding="utf-8")
    # GRANA.JSN intentionally omitted to prove missing indexed shards are skipped.
    (data_dir / "GO.JSN").write_text(json.dumps({"GOOD": {"meaning": "right"}, "GOOSE": "bird"}, separators=(",", ":")), encoding="utf-8")
    (data_dir / "ABAND1.JSN").write_text(json.dumps({"ABACUS": "frame"}, separators=(",", ":")), encoding="utf-8")
    (data_dir / "ABAND2.JSN").write_text(json.dumps({"ABANDON": {"meaning": "give up"}}, separators=(",", ":")), encoding="utf-8")
    return dict_dir


def run_builtin_smoke() -> None:
    import tempfile

    with tempfile.TemporaryDirectory() as td:
        dict_dir = build_sample_tree(Path(td))
        assert lookup(dict_dir, "GOOD")[0] == "GOOD"
        assert lookup(dict_dir, "G")[0] == "GAME"
        assert lookup(dict_dir, "G*", prefix_mode=True)[0] == "GAME"
        assert lookup(dict_dir, "ABANDON")[0] == "ABANDON"
        print("dictionary prefix query smoke: ok")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("dict_dir", nargs="?", help="Path to DICT folder. Omit for built-in smoke.")
    parser.add_argument("queries", nargs="*", help="Queries to test, e.g. G GOOD A*")
    args = parser.parse_args()

    if not args.dict_dir:
        run_builtin_smoke()
        return 0

    dict_dir = Path(args.dict_dir)
    failures = 0
    for query in args.queries or ["A*", "G", "GOOD"]:
        try:
            word, value, shard, warnings = lookup(dict_dir, query, prefix_mode=query.endswith("*"))
            for warning in warnings[:8]:
                print(f"warning: {query}: skipped {warning}", file=sys.stderr)
            if len(warnings) > 8:
                print(f"warning: {query}: skipped {len(warnings) - 8} more unreadable candidate shard(s)", file=sys.stderr)
            print(f"{query} -> {word} in {shard}: {value}")
        except Exception as exc:  # noqa: BLE001 - CLI should summarize all query failures.
            failures += 1
            print(f"{query} -> FAILED: {exc}", file=sys.stderr)
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
