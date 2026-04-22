#!/usr/bin/env python3
"""Copy bold_definitions table from a source SQLite database to a target database.

Offline script. Derives column names from the target schema so it is resilient
to schema changes and to column-order drift between source and target. Row ids
are preserved by copying the `id` column explicitly.
"""

import os
import sqlite3
import sys


CHUNK_SIZE = 10_000
TABLE = "bold_definitions"


def table_columns(conn: sqlite3.Connection, table: str) -> list[str]:
    rows = conn.execute(f"PRAGMA table_info({table})").fetchall()
    if not rows:
        raise RuntimeError(f"Table {table!r} does not exist")
    return [r[1] for r in rows]


def copy_bold_definitions(source_path: str, target_path: str) -> None:
    for label, path in (("source", source_path), ("target", target_path)):
        if not os.path.isfile(path):
            raise RuntimeError(f"{label} database does not exist: {path}")

    if os.path.realpath(source_path) == os.path.realpath(target_path):
        raise RuntimeError("source and target paths resolve to the same file")

    source = sqlite3.connect(source_path)
    target = sqlite3.connect(target_path, isolation_level=None)  # autocommit; manual tx

    try:
        source_cols = table_columns(source, TABLE)
        target_cols = table_columns(target, TABLE)

        missing_in_source = [c for c in target_cols if c not in source_cols]
        if missing_in_source:
            raise RuntimeError(
                f"Source {TABLE} is missing columns required by target: {missing_in_source}"
            )

        extra_in_source = [c for c in source_cols if c not in target_cols]
        if extra_in_source:
            print(
                f"Note: source has extra columns not in target (will be ignored): {extra_in_source}"
            )

        # Select columns by name in the target's order — immune to column-order drift.
        col_list = ",".join(f'"{c}"' for c in target_cols)
        placeholders = ",".join("?" * len(target_cols))

        total_source = source.execute(f"SELECT COUNT(*) FROM {TABLE}").fetchone()[0]
        print(f"Found {total_source} rows in source {TABLE}. Copying {len(target_cols)} columns: {target_cols}")

        cursor = source.execute(f"SELECT {col_list} FROM {TABLE}")

        target.execute("BEGIN EXCLUSIVE")
        copied = 0
        try:
            target.execute(f"DELETE FROM {TABLE}")
            insert_sql = f"INSERT INTO {TABLE} ({col_list}) VALUES ({placeholders})"
            while True:
                chunk = cursor.fetchmany(CHUNK_SIZE)
                if not chunk:
                    break
                target.executemany(insert_sql, chunk)
                copied += len(chunk)
                print(f"  {copied}/{total_source} rows copied...", end="\r")
            target.execute("COMMIT")
        except Exception:
            target.execute("ROLLBACK")
            raise

        print(f"\nDone. Copied {copied} rows to target {TABLE}.")

    finally:
        source.close()
        target.close()


if __name__ == "__main__":
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <source_db> <target_db>")
        sys.exit(1)

    copy_bold_definitions(sys.argv[1], sys.argv[2])
