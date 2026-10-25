"""Builds the map from the keys of the C# version to the keys of the Java version.

The C# version of the launcher names its texts with the keys of an older
version of the Java launcher (`hmcl.tasklistpage.468_` and the like); the
Java launcher names them with readable keys (`task`, `settings`, ...).

The map is built by matching the Chinese texts of both versions, which are
the same texts, and it is written as a JSON file the conversion script reads.
"""

from __future__ import annotations

import io
import json
import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

KEY = re.compile(r"^([^#!\s][^=]*)=(.*)$")


def read_properties(text: str) -> dict[str, str]:
    """Reads a property file into its entries."""
    entries: dict[str, str] = {}

    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("!"):
            continue

        match = KEY.match(line)
        if match:
            entries[match.group(1).strip()] = match.group(2)

    return entries


def git_show(branch: str, path: str) -> str:
    """Reads a file of the repository."""
    result = subprocess.run(
        ["git", "show", f"{branch}:{path}"], capture_output=True, text=True, encoding="utf-8"
    )
    if result.returncode != 0:
        raise RuntimeError(f"the file {path} cannot be read: {result.stderr.strip()}")
    return result.stdout


def main() -> int:
    # The Chinese texts of the C# version, which are the texts of the Java
    # version as well.
    csharp = read_properties(git_show("HMCL.NET", "src/HMCL.NET/Strings/lang/HMCL_zh_Hans.properties"))
    java = read_properties(
        io.open(
            r"D:\works\HMCL\HMCL\hmcl-rs\crates\hmcl-core\resources\lang\I18N_zh_Hans.properties",
            encoding="utf-8",
        ).read()
    )

    # The Java keys of every text, by the text itself.
    by_text: dict[str, list[str]] = defaultdict(list)
    for key, value in java.items():
        by_text[value].append(key)

    mapping: dict[str, str] = {}
    ambiguous: list[str] = []
    missing: list[str] = []

    for csharp_key, text in csharp.items():
        candidates = by_text.get(text, [])
        if len(candidates) == 1:
            mapping[csharp_key] = candidates[0]
        elif len(candidates) > 1:
            ambiguous.append(csharp_key)
        else:
            missing.append(csharp_key)

    print(f"the C# version has {len(csharp)} keys")
    print(f"  mapped:   {len(mapping)}")
    print(f"  ambiguous:{len(ambiguous)}")
    print(f"  missing:  {len(missing)}")

    for key in missing[:10]:
        print(f"    missing: {key} = {csharp[key]!r}")
    for key in ambiguous[:10]:
        print(f"    ambiguous: {key} = {csharp[key]!r} -> {by_text[csharp[key]]}")

    output = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("resource-key-map.json")
    output.write_text(json.dumps(mapping, ensure_ascii=False, indent=2, sort_keys=True), encoding="utf-8")
    print(f"the map is written to {output}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
