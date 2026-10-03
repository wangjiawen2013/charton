#!/usr/bin/env bash
#
# Fail if a committed docs image is not referenced by any Markdown file.
#
# Policy: `docs/src/images/` is a source directory for the book, so every file
# in it must be displayed somewhere. Generated-but-unreferenced output (smoke
# tests, dev demos) belongs under `target/`, which is git-ignored. This check
# keeps the documented image set from growing without bound.
#
# Dangling references (a Markdown link to a missing image) are already caught by
# mdBook's built-in `links` preprocessor during `mdbook build`.
set -euo pipefail
cd "$(dirname "$0")/.."

status=0
for file in docs/src/images/*; do
    [ -e "$file" ] || continue
    base=$(basename "$file")
    if ! grep -rqs --include='*.md' -- "$base" .; then
        echo "error: docs image is not referenced by any Markdown file: $file" >&2
        status=1
    fi
done

if [ "$status" -ne 0 ]; then
    echo >&2
    echo "Reference the image from a .md file, or delete it and point its" >&2
    echo "generator at target/ instead." >&2
    exit 1
fi
echo "docs images OK"
