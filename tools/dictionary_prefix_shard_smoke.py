#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path

MAX_PREFIX_LEN = 5


def normalize_word_key(word: str) -> str:
    return str(word).strip().upper()


def prefix_candidates(word: str) -> list[str]:
    normalized = normalize_word_key(word)
    if not normalized:
        return []
    if not normalized[0].isalpha():
        return ["OTHERS"]

    chars: list[str] = []
    for ch in normalized:
        if not ch.isalpha():
            break
        chars.append(ch)
        if len(chars) >= MAX_PREFIX_LEN:
            break

    if not chars:
        return ["OTHERS"]

    full = "".join(chars)
    return [full[:i] for i in range(len(full), 0, -1)] + ["OTHERS"]


def name_matches(name: str, prefix: str) -> bool:
    if name == prefix:
        return True
    if not name.startswith(prefix):
        return False
    suffix = name[len(prefix):]
    return bool(suffix) and suffix.isdigit()


def load_index(dict_dir: Path) -> list[tuple[str, Path]]:
    index_path = dict_dir / "INDEX.TXT"
    entries: list[tuple[str, Path]] = []
    for raw in index_path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        name, rel = line.split("|", 1)
        entries.append((name.strip(), dict_dir / rel.strip()))
    return entries


def lookup(dict_dir: Path, word: str) -> tuple[str, Path, object]:
    normalized = normalize_word_key(word)
    entries = load_index(dict_dir)

    for prefix in prefix_candidates(normalized):
        matched = False
        for name, shard_path in entries:
            if not name_matches(name, prefix):
                continue
            matched = True
            payload = json.loads(shard_path.read_text(encoding="utf-8"))
            if normalized in payload:
                return name, shard_path, payload[normalized]
        if matched:
            raise KeyError(f"word not found in matching shard group: {normalized}")

    raise KeyError(f"no shard found for word: {normalized}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-test a generated Rustmix DICT prefix-shard pack.")
    parser.add_argument("word", help="word to look up")
    parser.add_argument("--dict-dir", default="DICT", help="path to generated DICT folder")
    args = parser.parse_args()

    dict_dir = Path(args.dict_dir)
    name, path, value = lookup(dict_dir, args.word)
    print(f"word={normalize_word_key(args.word)}")
    print(f"shard={name}")
    print(f"path={path}")
    print(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
