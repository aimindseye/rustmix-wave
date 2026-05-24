#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import re
import struct
import sys
from pathlib import Path

ROOT = Path.cwd()
CSV = ROOT / "partitions" / "xteink_x4_standard.csv"
BIN = ROOT / "partitions" / "xteink_x4_standard.bin"
ROOT_CFG = ROOT / "espflash.toml"
TARGET_CFG = ROOT / "target-xteink-x4" / "espflash.toml"

EXPECTED = [
    {"name": "nvs", "type": "data", "subtype": "nvs", "type_id": 0x01, "subtype_id": 0x02, "offset": 0x9000, "size": 0x5000},
    {"name": "otadata", "type": "data", "subtype": "ota", "type_id": 0x01, "subtype_id": 0x00, "offset": 0xE000, "size": 0x2000},
    {"name": "app0", "type": "app", "subtype": "ota_0", "type_id": 0x00, "subtype_id": 0x10, "offset": 0x10000, "size": 0x640000},
    {"name": "app1", "type": "app", "subtype": "ota_1", "type_id": 0x00, "subtype_id": 0x11, "offset": 0x650000, "size": 0x640000},
    {"name": "spiffs", "type": "data", "subtype": "spiffs", "type_id": 0x01, "subtype_id": 0x82, "offset": 0xC90000, "size": 0x360000},
    {"name": "coredump", "type": "data", "subtype": "coredump", "type_id": 0x01, "subtype_id": 0x03, "offset": 0xFF0000, "size": 0x10000},
]


def fail(message: str) -> None:
    print(f"x4 partition validation failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def parse_int(value: str) -> int:
    value = value.strip()
    if value.lower().endswith("k"):
        return int(value[:-1], 0) * 1024
    if value.lower().endswith("m"):
        return int(value[:-1], 0) * 1024 * 1024
    return int(value, 0)


def parse_csv() -> list[dict[str, object]]:
    if not CSV.exists():
        fail(f"missing {CSV}")
    rows: list[dict[str, object]] = []
    for raw in CSV.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = [part.strip() for part in line.split(",")]
        if len(parts) < 5:
            fail(f"bad partition csv line: {raw}")
        rows.append(
            {
                "name": parts[0],
                "type": parts[1],
                "subtype": parts[2],
                "offset": parse_int(parts[3]),
                "size": parse_int(parts[4]),
            }
        )
    return rows


def validate_csv() -> None:
    rows = parse_csv()
    if len(rows) != len(EXPECTED):
        fail(f"expected {len(EXPECTED)} csv rows, found {len(rows)}")
    for row, exp in zip(rows, EXPECTED):
        for key in ("name", "type", "subtype", "offset", "size"):
            if row[key] != exp[key]:
                fail(f"csv {row['name']} {key}: expected {exp[key]!r}, found {row[key]!r}")
    labels = {str(row["name"]) for row in rows}
    if "factory" in labels:
        fail("factory app partition must not be present")
    if any(row["type"] == "data" and row["subtype"] == "phy" for row in rows):
        fail("data/phy partition must not be present in the CrossPoint-compatible X4 layout")


def parse_bin() -> list[dict[str, object]]:
    if not BIN.exists():
        fail(f"missing {BIN}")
    data = BIN.read_bytes()
    if len(data) != 0xC00:
        fail(f"partition bin must be 0xC00 bytes, found 0x{len(data):x}")
    entries: list[dict[str, object]] = []
    for off in range(0, len(data), 32):
        chunk = data[off : off + 32]
        if chunk[:2] == b"\xff\xff" or chunk == b"\xff" * 32:
            break
        if chunk[:16] == b"\xeb\xeb" + b"\xff" * 14:
            digest = chunk[16:32]
            actual = hashlib.md5(data[:off]).digest()
            if digest != actual:
                fail("partition table MD5 marker is present but checksum does not match")
            break
        if chunk[:2] != b"\xaa\x50":
            fail(f"bad partition entry magic at 0x{off:x}: {chunk[:2].hex()}")
        typ, subtype = chunk[2], chunk[3]
        offset, size = struct.unpack("<II", chunk[4:12])
        label = chunk[12:28].split(b"\x00", 1)[0].decode("ascii")
        flags = struct.unpack("<I", chunk[28:32])[0]
        entries.append(
            {"name": label, "type_id": typ, "subtype_id": subtype, "offset": offset, "size": size, "flags": flags}
        )
    return entries


def validate_bin() -> None:
    rows = parse_bin()
    if len(rows) != len(EXPECTED):
        fail(f"expected {len(EXPECTED)} binary rows, found {len(rows)}")
    for row, exp in zip(rows, EXPECTED):
        for key in ("name", "type_id", "subtype_id", "offset", "size"):
            if row[key] != exp[key]:
                fail(f"bin {row['name']} {key}: expected {exp[key]!r}, found {row[key]!r}")
    if rows[-1]["offset"] + rows[-1]["size"] != 0x1000000:
        fail("partition table must end exactly at 16MB / 0x1000000")


def validate_cfg(path: Path, expected_path: str) -> None:
    if not path.exists():
        fail(f"missing {path}")
    text = path.read_text()
    if "[idf]" not in text:
        fail(f"{path} missing [idf] section")
    if f'partition_table = "{expected_path}"' not in text:
        fail(f"{path} must reference {expected_path}")
    if "size = \"16MB\"" not in text:
        fail(f"{path} must set flash size to 16MB")


def iter_regression_scan_files() -> list[Path]:
    """Return source files that can affect flashing/partition layout.

    Only source/config files that can influence build or flash behavior are
    scanned here. Root documentation is intentionally consolidated elsewhere.
    """
    candidates: list[Path] = []

    direct_files = [ROOT_CFG]
    for direct in direct_files:
        if direct.exists():
            candidates.append(direct)

    scan_roots = [
        ROOT / "partitions",
        ROOT / "scripts",
        ROOT / "target-xteink-x4",
    ]
    allowed_suffixes = {".csv", ".toml", ".sh", ".py", ".rs"}
    ignored_names = {
        "validate_x4_standard_partition_table_compatibility.py",
    }
    ignored_dirs = {"target", ".git", "__MACOSX"}

    for base in scan_roots:
        if not base.exists():
            continue
        for path in base.rglob("*"):
            if not path.is_file():
                continue
            rel = path.relative_to(ROOT)
            if any(part in ignored_dirs for part in rel.parts):
                continue
            if path.name in ignored_names:
                continue
            if path.suffix.lower() not in allowed_suffixes:
                continue
            candidates.append(path)

    return candidates


def validate_no_known_regression_files() -> None:
    offenders: list[str] = []
    patterns = (
        "app-factory",
        "app_factory",
        "factory,  app",
        "factory,app",
        "data-phy",
        "data,phy",
        "data,  phy",
    )
    for path in iter_regression_scan_files():
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(errors="ignore")
        if any(pattern in text for pattern in patterns):
            offenders.append(rel)

    if offenders:
        fail("legacy factory/data-phy partition references remain: " + ", ".join(offenders[:8]))


if __name__ == "__main__":
    validate_csv()
    validate_bin()
    validate_cfg(ROOT_CFG, "partitions/xteink_x4_standard.bin")
    validate_cfg(TARGET_CFG, "../partitions/xteink_x4_standard.bin")
    validate_no_known_regression_files()
    print("x4-crosspoint-partition-table-compatibility-ok")
