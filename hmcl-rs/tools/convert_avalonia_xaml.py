"""Converts the XAML of the Avalonia version of HMCL into the XAML of FerroUI.

FerroUI is a port of Avalonia, so the markup of the two is the same; the
conversion only has to change the parts that name the framework and the
parts that belong to the C# version of the launcher.

The script reads the XAML of the C# version from the branch `HMCL.NET` of
the repository and writes the result into the crate of the Rust version.

Usage:

    python tools/convert_avalonia_xaml.py [--branch HMCL.NET] [--output ui/pages]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

# The namespace of the framework of Avalonia and the one of FerroUI.
AVALONIA_NAMESPACE = "https://github.com/avaloniaui"
FERROUI_NAMESPACE = "https://github.com/ferroui"

# The texts that the C# version wrote into the pages instead of using the
# resources of the launcher.
TEXT_OVERRIDES = {
    "正在运行 {0} 个任务": "Running {0} tasks",
}

# The converters of the C# version are taken from the resources of the
# application; the Rust version declares them as static fields instead, so
# the pages do not need the resources of the application to be loaded.
CONVERTER_OVERRIDES = {
    "{StaticResource BoolToTextConverter}": "{x:Static util:BoolToTextConverter.Instance}",
    "{StaticResource NullToBoolConverter}": "{x:Static util:NullToBoolConverter.Instance}",
    "{StaticResource NotNullToBoolConverter}": "{x:Static util:NotNullToBoolConverter.Instance}",
    "{StaticResource VersionTypeConverter}": "{x:Static util:VersionTypeConverter.Instance}",
}

# The prefix of the namespace of a type of the C# version and the namespace
# the type has in the Rust version.
#
# The namespaces of the Rust version are registered by the crate itself, which
# maps the modules of the crate to them; see `register_types` in `main.rs`.
MODULE_PREFIXES = {
    "HMCL.NET.Views": "HMCL.Views",
    "HMCL.NET.ViewModels": "HMCL.ViewModels",
    "HMCL.NET.Controls": "HMCL.Controls",
    "HMCL.NET.Util": "HMCL.Util",
    "HMCLCore.NET.Addon": "HMCL.Addon",
    "HMCLCore.NET.Auth": "HMCL.Auth",
    "HMCLCore.NET.Download": "HMCL.Download",
    "HMCLCore.NET.Game": "HMCL.Game",
    "HMCLCore.NET.Java": "HMCL.Java",
    "HMCLCore.NET.Setting": "HMCL.Setting",
    "HMCLCore.NET.Terracotta": "HMCL.Terracotta",
    "HMCLCore.NET.Util": "HMCL.Util",
}

# The attributes that only the designer of Avalonia uses.
DESIGN_ATTRIBUTES = ("d:DesignWidth", "d:DesignHeight", "d:DataContext", "mc:Ignorable")


def repository_root() -> Path:
    """Returns the root of the repository."""
    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
    )
    if result.returncode != 0:
        raise RuntimeError("the repository cannot be found")
    return Path(result.stdout.strip())


def read_xaml(branch: str, path: str) -> str:
    """Reads a XAML file of the C# version."""
    result = subprocess.run(
        ["git", "show", f"{branch}:{path}"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        cwd=repository_root(),
    )
    if result.returncode != 0:
        raise RuntimeError(f"the file {path} cannot be read: {result.stderr.strip()}")
    return result.stdout


def convert_namespace(text: str) -> str:
    """Rewrites the namespace of the framework."""
    return text.replace(f'"{AVALONIA_NAMESPACE}"', f'"{FERROUI_NAMESPACE}"')


def convert_module(text: str) -> str:
    """Rewrites the namespaces of the types of the C# version.

    The namespaces of the Rust version are the ones the crate registers for
    its modules; see `register_types` in `main.rs`.
    """
    for csharp, rust in MODULE_PREFIXES.items():
        text = text.replace(f"clr-namespace:{csharp}", f"clr-namespace:{rust}")

    # The assemblies of the C# version have no counterpart in Rust.
    text = re.sub(r";assembly=[A-Za-z0-9_.]+", "", text)

    # The namespaces that name the controls of Avalonia.
    text = text.replace("using:Avalonia.Controls", "using:FerroUI.Controls")
    text = text.replace("using:Avalonia", "using:FerroUI")

    return text


# The events the C# version handles in the code behind the document.
EVENTS = (
    "Click",
    "Command",
    "Tapped",
    "SelectionChanged",
    "SelectionChanging",
    "TextChanged",
    "KeyDown",
    "PointerPressed",
    "DoubleTapped",
)


def remove_event_handlers(text: str) -> str:
    """Removes the handlers of the events of the code behind the document.

    The run-time loader of FerroUI does not know the code behind the
    document, so a handler named there is a failure of the load. The Rust
    version attaches the handlers in code instead.
    """
    for event in EVENTS:
        # The handlers that are written as a name are removed; the ones that
        # are written as a binding are kept.
        text = re.sub(
            rf'\s+{event}="(?![{{])[^"]*"',
            "",
            text,
        )
    return text


def remove_attributes(text: str) -> str:
    """Removes the attributes that only the designer and the C# version use."""
    # The class of the code behind the document: the Rust version builds the
    # objects itself.
    text = re.sub(r'\s+x:Class="[^"]*"', "", text)
    text = re.sub(r'\s+x:CompileBindings="[^"]*"', "", text)

    for attribute in DESIGN_ATTRIBUTES:
        text = re.sub(rf'\s+{re.escape(attribute)}="[^"]*"', "", text)

    # The namespace of the designer, when nothing uses it any more.
    if "d:" not in text:
        text = re.sub(r'\s+xmlns:d="[^"]*"', "", text)
    if "mc:" not in text:
        text = re.sub(r'\s+xmlns:mc="[^"]*"', "", text)

    return text


def convert_text(text: str) -> str:
    """Replaces the texts that the C# version wrote into the pages."""
    for chinese, english in TEXT_OVERRIDES.items():
        text = text.replace(chinese, english)
    return text


def convert_converters(text: str) -> str:
    """Replaces the references to the converters of the C# version."""
    for csharp, rust in CONVERTER_OVERRIDES.items():
        text = text.replace(csharp, rust)
    return text


# The namespaces the pages may name, and the ones they are declared with.
NAMESPACES = {
    "util": "HMCL.Util",
    "vm": "HMCL.ViewModels",
    "views": "HMCL.Views",
    "controls": "HMCL.Controls",
    "game": "HMCL.Game",
    "addon": "HMCL.Addon",
    "auth": "HMCL.Auth",
    "core": "HMCL.Setting",
    "dl": "HMCL.Download",
    "java": "HMCL.Java",
    "terracotta": "HMCL.Terracotta",
}


def add_missing_namespaces(text: str) -> str:
    """Declares the namespaces the document names but does not declare."""
    missing = [
        (prefix, namespace)
        for prefix, namespace in NAMESPACES.items()
        if f"{prefix}:" in text and f'xmlns:{prefix}=' not in text
    ]
    if not missing:
        return text

    declarations = " ".join(
        f'xmlns:{prefix}="clr-namespace:{namespace}"' for prefix, namespace in missing
    )
    # The declarations go after the namespace of the framework.
    marker = 'xmlns="https://github.com/ferroui"'
    if marker in text:
        return text.replace(marker, f"{marker}\n             {declarations}", 1)

    return text


def convert(text: str) -> str:
    """Converts a XAML document of the C# version into one of the Rust version."""
    text = convert_namespace(text)
    text = convert_module(text)
    text = remove_attributes(text)
    text = remove_event_handlers(text)
    text = convert_text(text)
    text = convert_converters(text)
    text = add_missing_namespaces(text)
    return text


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--branch", default="HMCL.NET", help="the branch of the C# version")
    parser.add_argument(
        "--output", default="crates/hmcl/ui/pages", help="the directory the documents are written to"
    )
    arguments = parser.parse_args()

    root = Path(__file__).resolve().parent.parent
    output = root / arguments.output
    output.mkdir(parents=True, exist_ok=True)

    listing = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", arguments.branch, "--", "src/HMCL.NET"],
        capture_output=True,
        text=True,
        cwd=repository_root(),
    )
    if listing.returncode != 0:
        print("the branch cannot be read", file=sys.stderr)
        return 1

    documents = [
        path
        for path in listing.stdout.splitlines()
        if path.endswith(".axaml") and "/Strings/" not in path
    ]

    for path in documents:
        name = Path(path).name
        converted = convert(read_xaml(arguments.branch, path))
        (output / name).write_text(converted, encoding="utf-8", newline="\n")
        print(f"converted {path} -> {output / name}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
