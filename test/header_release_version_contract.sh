#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
header="$project_root/src/components/Header.astro"
roadmap="$project_root/src/components/RoadmapTimeline.astro"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
changelog_en="$project_root/src/content/docs-en/releases/changelog.mdx"
changelog_es="$project_root/src/content/docs-es/releases/changelog.mdx"

test "$(node -p "require('$project_root/package.json').version")" = "0.1.0"
test "$(node -p "require('$project_root/package-lock.json').version")" = "0.1.0"
test "$(node -p "require('$project_root/package-lock.json').packages[''].version")" = "0.1.0"

if ! grep -Fq '>v0.1.0</span>' "$header"; then
    printf 'error: header does not advertise v0.1.0\n'
    exit 1
fi

grep -Fq '"badge": "v0.1.0 -' "$english"
grep -Fq '"badge": "v0.1.0 -' "$spanish"
grep -Fq '>v0.1.0</span>' "$roadmap"
grep -Fq '## 0.1.0 - 2026-08-04' "$changelog_en"
grep -Fq '## 0.1.0 - 2026-08-04' "$changelog_es"

if grep -Fq 'v0.0.1' "$header" "$english" "$spanish"; then
    printf 'error: current release surfaces still advertise v0.0.1\n'
    exit 1
fi

printf 'public release version verified: v0.1.0\n'
