"""Repairs the English texts of the C# version of the launcher.

The English texts of the C# version are shifted: the text of an element is
written under the key of the element before it. The Chinese texts are
correct, and the texts of the Java version tell which text belongs to which
key, so the shift of every page can be found and undone.

The script writes the repaired texts, so the Rust version of the launcher can
use them for the pages that were ported from the C# version.
"""

from __future__ import annotations

import io
import re
import subprocess
import sys
from collections import Counter, defaultdict
from pathlib import Path

KEY = re.compile(r"^([^#!\s][^=]*)=(.*)$")

# The pages whose texts the Java version does not know, so the shift of their
# texts cannot be found; the texts of the C# version are moved by hand.
MANUAL_SHIFTS = {
    "tasklistpage": -2,
}

# The texts the C# version does not hold at all.
OVERRIDES = {
    "hmcl.tasklistpage.468_": "Task list",
    "hmcl.tasklistpage.470_": "Cancel All",
}


def read_properties(text: str) -> list[tuple[str, str]]:
    """Reads a property file into its entries, in the order of the file."""
    entries: list[tuple[str, str]] = []

    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or line.startswith("!"):
            continue

        match = KEY.match(line)
        if match:
            entries.append((match.group(1).strip(), match.group(2)))

    return entries


def git_show(path: str) -> str:
    """Reads a file of the repository."""
    result = subprocess.run(
        ["git", "show", f"HMCL.NET:{path}"], capture_output=True, text=True, encoding="utf-8"
    )
    if result.returncode != 0:
        raise RuntimeError(f"the file {path} cannot be read: {result.stderr.strip()}")
    return result.stdout


def page_of(key: str) -> str:
    """Returns the page the key belongs to."""
    parts = key.split(".")
    if len(parts) >= 2:
        return parts[1]
    return key


def main() -> int:
    root = Path(__file__).resolve().parent.parent

    chinese = read_properties(
        git_show("src/HMCL.NET/Strings/lang/HMCL_zh_Hans.properties")
    )
    english = read_properties(git_show("src/HMCL.NET/Strings/lang/HMCL.properties"))

    java_chinese = dict(
        read_properties(
            io.open(
                root / "crates/hmcl-core/resources/lang/I18N_zh_Hans.properties", encoding="utf-8"
            ).read()
        )
    )
    java_english = dict(
        read_properties(
            io.open(root / "crates/hmcl-core/resources/lang/I18N.properties", encoding="utf-8").read()
        )
    )

    if len(chinese) != len(english):
        print("the two files of the C# version do not have the same number of keys")
        return 1

    # The Java key of every key of the C# version, by the Chinese text.
    by_text: dict[str, list[str]] = defaultdict(list)
    for key, value in java_chinese.items():
        by_text[value].append(key)

    # The text every key of the C# version should have, when the Java
    # version knows it.
    expected: dict[int, str] = {}
    for index, (key, text) in enumerate(chinese):
        candidates = by_text.get(text, [])
        if len(candidates) == 1:
            expected[index] = java_english.get(candidates[0], "")

    # The shift of every page: the text of a key is the text of the key that
    # comes the shift positions later.
    shifts: dict[str, Counter[int]] = defaultdict(Counter)
    for index, text in expected.items():
        page = page_of(chinese[index][0])
        for shift in range(-4, 5):
            other = expected.get(index + shift)
            if other is not None and other == english[index][1]:
                shifts[page][shift] += 1

    # The shift of every page is the one that holds most often.
    page_shift: dict[str, int] = {}
    for page, counter in shifts.items():
        if counter:
            shift, count = counter.most_common(1)[0]
            if count >= 2:
                page_shift[page] = shift

    page_shift.update(MANUAL_SHIFTS)

    print(f"the pages with a shift: {len(page_shift)} of {len(shifts)}")
    for page in sorted(page_shift)[:12]:
        print(f"  {page}: {page_shift[page]:+d}")

    # The texts are put back where they belong.
    repaired: list[tuple[str, str]] = []
    fixed = 0
    for index, (key, _) in enumerate(chinese):
        page = page_of(key)
        shift = page_shift.get(page, 0)
        source = index + shift
        text = english[source][1] if 0 <= source < len(english) else english[index][1]
        if shift != 0:
            fixed += 1
        repaired.append((key, text))

    # The texts that the Java version knows are used, because they are the
    # texts of the current version of the launcher.
    for index, (key, _) in enumerate(chinese):
        text = expected.get(index)
        if text is not None:
            repaired[index] = (key, text)

    for key, text in OVERRIDES.items():
        for index, (name, _) in enumerate(chinese):
            if name == key:
                repaired[index] = (key, text)

    folder = root / "crates/hmcl-core/resources/lang"

    output = folder / "legacy.properties"
    with io.open(output, "w", encoding="utf-8", newline="\n") as file:
        file.write(
            "# The texts of the pages that were ported from the C# version of the\n"
            "# launcher. The keys are the ones of the C# version, because the pages\n"
            "# of that version name them.\n"
        )
        for key, text in repaired:
            file.write(f"{key}={text}\n")

    # The Chinese texts of the C# version are correct, so they are used as
    # they are.
    output_chinese = folder / "legacy_zh_Hans.properties"
    with io.open(output_chinese, "w", encoding="utf-8", newline="\n") as file:
        file.write(
            "# The Chinese texts of the pages that were ported from the C# version of\n"
            "# the launcher.\n"
        )
        for key, text in chinese:
            file.write(f"{key}={text}\n")

    print(f"the texts of {fixed} keys were moved back")
    print(f"the texts are written to {output} and {output_chinese}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
