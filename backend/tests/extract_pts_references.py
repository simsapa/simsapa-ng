#!/usr/bin/env python3
"""
Extract PTS references from Markdown chapter files.

Parses links in the format:
[<sutta_ref> / <pts_ref>](<url>)

And outputs CSV with columns:
pts_ref_query,sutta_ref,url
"""

import re
import csv
import sys
from pathlib import Path


def extract_pts_references(markdown_content):
    """
    Extract PTS references from markdown content.

    Pattern: [<sutta_ref> / <pts_ref>](<url>)
    Examples:
        [MN 87 / M II 110](https://suttacentral.net/mn87/pli/ms)
        [SN 20.7 / S II 267](https://suttacentral.net/sn20.7/pli/ms)
        [MN 135 / M iii 203](https://suttacentral.net/mn135/pli/ms)
        [Snp 3.9 / Sn 118-19](https://suttacentral.net/snp3.9/pli/ms)
    """
    # Pattern matches: [text with / in it](url)
    # Uses non-greedy matching and handles non-breaking spaces
    pattern = r'\[([^\]]+?)\s*/\s*([^\]]+?)\]\((https://suttacentral\.net/[^\)]+)\)'

    references = []
    for match in re.finditer(pattern, markdown_content):
        sutta_ref = match.group(1).strip()
        pts_ref = match.group(2).strip()
        url = match.group(3).strip()

        # Clean up any remaining special characters
        # Replace non-breaking space with regular space
        pts_ref = pts_ref.replace('\u00a0', ' ')
        sutta_ref = sutta_ref.replace('\u00a0', ' ')

        references.append({
            'pts_ref_query': pts_ref,
            'sutta_ref': sutta_ref,
            'url': url
        })

    return references


def main():
    # Path to chapters directory
    chapters_dir = Path(__file__).parent / 'data' / 'chapters'

    if not chapters_dir.exists():
        print(f"Error: Directory not found: {chapters_dir}", file=sys.stderr)
        sys.exit(1)

    # Collect all references
    all_references = []
    markdown_files = sorted(chapters_dir.glob('*.md'))

    print(f"Processing {len(markdown_files)} markdown files...", file=sys.stderr)

    for md_file in markdown_files:
        print(f"  Processing: {md_file.name}", file=sys.stderr)
        content = md_file.read_text(encoding='utf-8')
        references = extract_pts_references(content)
        all_references.extend(references)

    # Remove duplicates while preserving order
    seen = set()
    unique_references = []
    for ref in all_references:
        # Create a tuple key for deduplication
        key = (ref['pts_ref_query'], ref['sutta_ref'], ref['url'])
        if key not in seen:
            seen.add(key)
            unique_references.append(ref)

    print(f"\nFound {len(all_references)} total references", file=sys.stderr)
    print(f"Found {len(unique_references)} unique references", file=sys.stderr)

    # Write to CSV
    output_file = Path(__file__).parent / 'data' / 'pts_references_lookup_test.csv'

    with output_file.open('w', newline='', encoding='utf-8') as csvfile:
        fieldnames = ['pts_ref_query', 'sutta_ref', 'url']
        writer = csv.DictWriter(csvfile, fieldnames=fieldnames)

        writer.writeheader()
        for ref in unique_references:
            writer.writerow(ref)

    print(f"\nWrote {len(unique_references)} references to: {output_file}", file=sys.stderr)

    # Print first few entries as sample
    print("\nSample entries:", file=sys.stderr)
    for ref in unique_references[:5]:
        print(f"  {ref['pts_ref_query']} | {ref['sutta_ref']} | {ref['url']}", file=sys.stderr)


if __name__ == '__main__':
    main()
