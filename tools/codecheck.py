#!/usr/bin/env python3

import itertools
import os
import pathlib
import re
import sys

PROJECT_ROOT_DIR = pathlib.Path(__file__).resolve().parent.parent

LICENSE_TEMPLATE1 = [
    r"/\*{77}",
    r" \*   Mintlayer Ledger App\.",
    r" \*   \(c\) 202[0-9](-202[0-9])? RBB S\.r\.l\.",
    r" \*",
    r' \*  Licensed under the Apache License, Version 2\.0 \(the "License"\);',
    r" \*  you may not use this file except in compliance with the License\.",
    r" \*  You may obtain a copy of the License at",
    r" \*",
    r" \*      http://www\.apache\.org/licenses/LICENSE-2\.0",
    r" \*",
    r" \*  Unless required by applicable law or agreed to in writing, software",
    r' \*  distributed under the License is distributed on an "AS IS" BASIS,',
    r" \*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied\.",
    r" \*  See the License for the specific language governing permissions and",
    r" \*  limitations under the License\.",
    r" \*{77}/",
]

LICENSE_TEMPLATE2 = [
    r"/\*{77}",
    r" \*   Mintlayer Ledger App\.",
    r" \*   \(c\) 202[0-9](-202[0-9])? Ledger SAS\.",
    r" \*   \(c\) 202[0-9](-202[0-9])? RBB S\.r\.l\.",
    r" \*",
    r' \*  Licensed under the Apache License, Version 2\.0 \(the "License"\);',
    r" \*  you may not use this file except in compliance with the License\.",
    r" \*  You may obtain a copy of the License at",
    r" \*",
    r" \*      http://www\.apache\.org/licenses/LICENSE-2\.0",
    r" \*",
    r" \*  Unless required by applicable law or agreed to in writing, software",
    r' \*  distributed under the License is distributed on an "AS IS" BASIS,',
    r" \*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied\.",
    r" \*  See the License for the specific language governing permissions and",
    r" \*  limitations under the License\.",
    r" \*{77}/",
]

COMMON_EXCLUDE_DIRS = ["target", ".git", ".mypy_cache"]


# List source files with the given extension. If the extension is None, list all files.
def sources_with_extension(ext: str | None, top_dir=".", exclude=[]):
    exclude_full_paths = [
        os.path.normpath(rel_path) for rel_path in COMMON_EXCLUDE_DIRS + exclude
    ]
    exclude_names = ["__pycache__"]

    def is_excluded(dirpath, entry_name):
        return (
            entry_name in exclude_names
            or os.path.normpath(os.path.join(dirpath, entry_name).lower())
            in exclude_full_paths
        )

    for dirpath, dirnames, filenames in os.walk(top_dir, topdown=True):
        dirnames[:] = [d for d in dirnames if not is_excluded(dirpath, d)]
        for filename in filenames:
            if not is_excluded(dirpath, filename):
                if ext is None or os.path.splitext(filename)[1].lower() == ext:
                    yield os.path.join(dirpath, filename)


# List all files
def all_files(exclude=[]):
    return sources_with_extension(None, exclude=exclude)


# List Rust source files
def rs_sources(exclude=[]):
    return sources_with_extension(".rs", exclude=exclude)


# List Cargo config files
def cargo_config_files(exclude=[]):
    return sources_with_extension(".toml", exclude=exclude)


# List Python source files
def py_sources(exclude=[]):
    return sources_with_extension(".py", exclude=exclude)


# List GitHub workflow files
def github_workflows(exclude=[]):
    return sources_with_extension(".yml", top_dir=".github", exclude=exclude)


# Check license header in Rust source files
def check_local_licenses():
    print("==== Checking local license headers:")

    # List of files/dirs excluded from the check
    exclude = []

    template1 = re.compile("(?:" + r")\n(?:".join(LICENSE_TEMPLATE1) + ")")
    template2 = re.compile("(?:" + r")\n(?:".join(LICENSE_TEMPLATE2) + ")")

    ok = True
    for path in rs_sources(exclude):
        with open(path, "r", encoding="utf-8") as file:
            file_contents = file.read()
            if not (template1.search(file_contents) or template2.search(file_contents)):
                ok = False
                print("{}: license missing or incorrect".format(path))

    print()
    return ok


# Check TODO(PR) and FIXME instances
def check_todos():
    print("==== Checking TODO(PR) and FIXME instances:")

    # List of files/dirs excluded from the check
    exclude = [
        # Exclude itself
        "tools/codecheck.py",
    ]

    ok = True
    for path in itertools.chain(
        rs_sources(exclude),
        cargo_config_files(exclude),
        py_sources(exclude),
        github_workflows(exclude),
    ):
        with open(path, "r", encoding="utf-8") as file:
            file_data = file.read()
            if "TODO(PR)" in file_data or "FIXME" in file_data:
                ok = False
                print("{}: found TODO(PR) or FIXME instances".format(path))

    print()
    return ok


# Check for trailing whitespaces
def check_trailing_whitespaces():
    print("==== Checking for trailing whitespaces:")

    # List of files/dirs excluded from the check
    exclude = [
        "media",
        "tests/snapshots",
        "tests/snapshots-tmp",
    ]

    ok = True
    for path in all_files(exclude):
        with open(path, "r", encoding="utf-8") as file:
            try:
                for line_idx, line in enumerate(file, start=1):
                    line = line.rstrip("\n\r")
                    if line != line.rstrip():
                        ok = False
                        print(f"{path}: trailing whitespaces at line {line_idx}")
            except:
                ok = False
                print(
                    f"{path}: can't check for trailing whitespaces, "
                    "perhaps the file should be in the 'exclude' list?"
                )

    print()
    return ok


def run_checks():
    return all(
        [
            check_local_licenses(),
            check_todos(),
            check_trailing_whitespaces(),
        ]
    )


if __name__ == "__main__":
    # Note: this script expects the current directory to be the project root.
    os.chdir(PROJECT_ROOT_DIR)

    if not run_checks():
        sys.exit(1)
