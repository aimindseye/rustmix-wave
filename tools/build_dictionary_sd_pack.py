#!/usr/bin/env python3
"""Build an X4-safe Rustmix dictionary SD pack from dictionary.json.

Usage:
  python3 tools/build_dictionary_sd_pack.py dictionary.json examples/sd-card/RUSTMIX/APPS/DICT

The output folder is replaced atomically from the source dictionary, so INDEX.TXT
and DATA/*.JSN stay in sync. Do not merge-copy a new INDEX.TXT over old DATA.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
from pathlib import Path
from typing import Any

TARGET_SHARD_BYTES = 14 * 1024
HARD_SHARD_BYTES = 16 * 1024
MAX_PREFIX_LEN = 5
DATA_DIR_NAME = "DATA"
INDEX_FILE_NAME = "INDEX.TXT"
NAME_RE = re.compile(r"[A-Z0-9_]{1,8}")


def normalize_word_key(word: str) -> str:
    return str(word).strip().upper()


def prefix_for_word(word: str, length: int) -> str:
    clean = normalize_word_key(word)
    if not clean:
        return "OTHERS"
    first = clean[0]
    if not first.isalpha():
        return "OTHERS"
    chars: list[str] = []
    for ch in clean:
        if not ch.isalpha():
            break
        chars.append(ch)
        if len(chars) >= length:
            break
    return "".join(chars) if chars else "OTHERS"


def is_83_safe_name(name: str) -> bool:
    return bool(NAME_RE.fullmatch(name))


def json_bytes(payload: dict[str, Any]) -> bytes:
    return json.dumps(payload, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")


def group_by_prefix(data: dict[str, Any], prefix_len: int) -> dict[str, dict[str, Any]]:
    groups: dict[str, dict[str, Any]] = {}
    for word, details in data.items():
        normalized_word = normalize_word_key(word)
        prefix = prefix_for_word(normalized_word, prefix_len)
        if not is_83_safe_name(prefix):
            prefix = "OTHERS"
        groups.setdefault(prefix, {})[normalized_word] = details
    return groups


def split_large_group_by_chunks(base_name: str, words: dict[str, Any]) -> list[tuple[str, dict[str, Any]]]:
    result: list[tuple[str, dict[str, Any]]] = []
    current: dict[str, Any] = {}
    chunk_index = 1

    def make_name(index: int) -> str:
        suffix = str(index)
        base_max = 8 - len(suffix)
        name = f"{base_name[:base_max]}{suffix}"
        if not is_83_safe_name(name):
            raise ValueError(f"generated unsafe shard name: {name}")
        return name

    for word, details in sorted(words.items()):
        candidate = dict(current)
        candidate[word] = details
        if current and len(json_bytes(candidate)) > TARGET_SHARD_BYTES:
            result.append((make_name(chunk_index), current))
            chunk_index += 1
            current = {word: details}
        else:
            current = candidate
    if current:
        result.append((make_name(chunk_index), current))
    return result


def split_group_until_safe(group_name: str, words: dict[str, Any], prefix_len: int) -> list[tuple[str, dict[str, Any]]]:
    if len(json_bytes(words)) <= TARGET_SHARD_BYTES:
        return [(group_name, words)]
    if group_name == "OTHERS":
        return split_large_group_by_chunks(group_name, words)
    if prefix_len < MAX_PREFIX_LEN:
        subgroups: dict[str, dict[str, Any]] = {}
        for word, details in words.items():
            prefix = prefix_for_word(word, prefix_len + 1)
            if not is_83_safe_name(prefix):
                prefix = group_name
            subgroups.setdefault(prefix, {})[word] = details
        result: list[tuple[str, dict[str, Any]]] = []
        for sub_name, sub_words in sorted(subgroups.items()):
            result.extend(split_group_until_safe(sub_name, sub_words, prefix_len + 1))
        return result
    return split_large_group_by_chunks(group_name, words)


def validate_shards(shards: list[tuple[str, dict[str, Any]]]) -> None:
    seen: set[str] = set()
    for name, payload in shards:
        if name in seen:
            raise ValueError(f"duplicate shard name generated: {name}")
        seen.add(name)
        if not is_83_safe_name(name):
            raise ValueError(f"shard name is not 8.3-safe: {name}.JSN")
        size = len(json_bytes(payload))
        if size > HARD_SHARD_BYTES:
            raise ValueError(f"shard {name}.JSN is too large: {size} bytes, hard max {HARD_SHARD_BYTES}")


def write_pack(output_dir: Path, shards: list[tuple[str, dict[str, Any]]]) -> None:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    data_dir = output_dir / DATA_DIR_NAME
    data_dir.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    total_words = 0
    for name, payload in sorted(shards, key=lambda item: item[0]):
        filename = f"{name}.JSN"
        path = data_dir / filename
        path.write_bytes(json_bytes(payload))
        size = path.stat().st_size
        if size > HARD_SHARD_BYTES:
            raise ValueError(f"wrote oversized shard unexpectedly: {path} = {size}")
        lines.append(f"{name}|DATA/{filename}")
        total_words += len(payload)
        print(f"Created: {path} ({size} bytes, {len(payload)} words)")

    (output_dir / INDEX_FILE_NAME).write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"Created: {output_dir / INDEX_FILE_NAME} ({len(lines)} entries, {total_words} words)")


def build(source_path: Path, output_dir: Path) -> None:
    if not source_path.exists():
        raise FileNotFoundError(f"dictionary file not found: {source_path}")
    with source_path.open("r", encoding="utf-8") as f:
        data = json.load(f)
    if not isinstance(data, dict):
        raise ValueError("dictionary.json must contain a top-level JSON object")

    normalized: dict[str, Any] = {}
    for word, details in data.items():
        key = normalize_word_key(word)
        if key:
            normalized[key] = details

    shards: list[tuple[str, dict[str, Any]]] = []
    for group_name, words in sorted(group_by_prefix(normalized, 1).items()):
        shards.extend(split_group_until_safe(group_name, words, 1))
    validate_shards(shards)
    write_pack(output_dir, shards)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("dictionary_json", help="Source dictionary.json")
    parser.add_argument("output_dict_dir", help="Output DICT folder, e.g. examples/sd-card/RUSTMIX/APPS/DICT")
    args = parser.parse_args()
    build(Path(args.dictionary_json), Path(args.output_dict_dir))
    print()
    print("dictionary SD pack build ok")
    print("Expected device layout: /RUSTMIX/APPS/DICT/INDEX.TXT and /RUSTMIX/APPS/DICT/DATA/*.JSN")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
