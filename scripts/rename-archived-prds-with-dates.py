#!/usr/bin/env python3
"""Rename archived PRD and task-list files with a date-time prefix.

The files in ``tasks/archive/`` come in pairs: a PRD/analysis file ``X.md`` and
its matching task list ``tasks-X.md`` (for example ``prd-bookmarks.md`` and
``tasks-prd-bookmarks.md``). This script prepends a ``YYYY-MM-DD-HHMMSS-``
prefix to each file so that the pairs sort next to each other in a file
listing.

The timestamp is taken from when the file was first added to git. For a pair,
the PRD file's creation time is used for *both* files so they share an identical
prefix. A file that has no ``tasks-`` partner (or vice versa) simply uses its
own git creation time.

Files that already carry a date prefix (``2026-06-04-091014-...``) are skipped,
as are non-Markdown files such as the saved ``.txt`` transcripts.

Usage:
    python scripts/rename-archived-prds-with-dates.py [--apply]

Without ``--apply`` the script only prints the planned renames (dry run).
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

# tasks/archive relative to the repo root (this script lives in scripts/).
ARCHIVE_DIR = Path(__file__).resolve().parent.parent / "tasks" / "archive"

# Matches names that already start with a YYYY-MM-DD-HHMMSS- prefix.
DATE_PREFIX_RE = re.compile(r"^\d{4}-\d{2}-\d{2}-\d{6}-")


def git_created_at(path: Path) -> str | None:
    """Return the YYYY-MM-DD-HHMMSS timestamp of when *path* was added to git."""
    result = subprocess.run(
        [
            "git",
            "log",
            "--diff-filter=A",
            "--follow",
            "--format=%ad",
            "--date=format:%Y-%m-%d-%H%M%S",
            "-1",
            "--",
            str(path),
        ],
        cwd=ARCHIVE_DIR,
        capture_output=True,
        text=True,
    )
    stamp = result.stdout.strip().splitlines()
    return stamp[0] if stamp else None


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Perform the renames (default is a dry run).",
    )
    args = parser.parse_args()

    if not ARCHIVE_DIR.is_dir():
        print(f"Archive directory not found: {ARCHIVE_DIR}", file=sys.stderr)
        return 1

    # Candidate files: Markdown files without an existing date prefix.
    md_files = {
        p.name
        for p in ARCHIVE_DIR.iterdir()
        if p.is_file()
        and p.suffix == ".md"
        and not DATE_PREFIX_RE.match(p.name)
    }

    # Pair each `tasks-X.md` with its partner `X.md`.
    # The PRD/main file is the partner of a tasks file; its timestamp wins.
    renames: list[tuple[str, str]] = []
    handled: set[str] = set()

    for name in sorted(md_files):
        if name in handled:
            continue

        if name.startswith("tasks-"):
            main_name = name[len("tasks-"):]
            tasks_name = name
        else:
            main_name = name
            tasks_name = f"tasks-{name}"

        # The timestamp source is always the PRD/main file when it exists,
        # otherwise the tasks file stands alone.
        timestamp_source = main_name if main_name in md_files else tasks_name
        stamp = git_created_at(ARCHIVE_DIR / timestamp_source)
        if stamp is None:
            print(
                f"WARNING: no git creation date for {timestamp_source}; skipping",
                file=sys.stderr,
            )
            handled.update({main_name, tasks_name} & md_files)
            continue

        for member in (main_name, tasks_name):
            if member in md_files and member not in handled:
                renames.append((member, f"{stamp}-{member}"))
                handled.add(member)

    for old, new in sorted(renames):
        print(f"{old}\n  -> {new}")
        if args.apply:
            subprocess.run(
                ["git", "mv", old, new],
                cwd=ARCHIVE_DIR,
                check=True,
            )

    if not args.apply:
        print(f"\nDry run: {len(renames)} file(s) would be renamed. "
              "Re-run with --apply to perform the renames.")
    else:
        print(f"\nRenamed {len(renames)} file(s).")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
