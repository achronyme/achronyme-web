#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
header="$project_root/src/components/Header.astro"

if ! grep -Fq '>v0.0.1</span>' "$header"; then
    printf 'error: header does not advertise v0.0.1\n'
    exit 1
fi

if grep -Fq 'v0.1.0-beta.22' "$header"; then
    printf 'error: header still advertises the historical beta release\n'
    exit 1
fi

printf 'header release version verified: v0.0.1\n'
