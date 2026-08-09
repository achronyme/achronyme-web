#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
header="$project_root/src/components/Header.astro"
roadmap="$project_root/src/components/RoadmapTimeline.astro"
landing="$project_root/src/components/PageGrid.astro"
sidebar="$project_root/src/data/docs-sidebar.ts"
release_data="$project_root/src/data/release-status.ts"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
changelog_en="$project_root/src/content/docs-en/releases/changelog.mdx"
changelog_es="$project_root/src/content/docs-es/releases/changelog.mdx"

test "$(node -p "require('$project_root/package.json').version")" = "0.1.2"
test "$(node -p "require('$project_root/package-lock.json').version")" = "0.1.2"
test "$(node -p "require('$project_root/package-lock.json').packages[''].version")" = "0.1.2"

test "$(grep -Fc "stable: '0.1.2'" "$release_data")" -eq 2
grep -Fq "stableRevision: 'cd7a6e66e133bebd8e2026e321a4c85023c311f7'" "$release_data"
grep -Fq "stableUrl: 'https://github.com/achronyme/achronyme/releases/tag/v0.1.2'" "$release_data"
grep -Fq "stable: '0.3.1'" "$release_data"
grep -Fq "stableUrl: 'https://github.com/achronyme/achronyme-editor/releases/tag/v0.3.1'" "$release_data"
grep -Fq 'releaseStatus.core.stable' "$header"
grep -Fq 'releaseStatus.core.stable' "$roadmap"
grep -Fq 'releaseStatus.core.stableRevision' "$roadmap"
grep -Fq 'releaseStatus.editor.stable' "$roadmap"
grep -Fq 'releaseStatus.web.stable' "$roadmap"
grep -Fq 'releaseStatus.core.stable' "$landing"
grep -Fq 'releaseStatus.core.stableRevision' "$landing"
grep -Fq 'releaseStatus.editor.stable' "$landing"
grep -Fq 'releaseStatus.web.stable' "$landing"
grep -Fq '"stableLabel": "Stable"' "$english"
grep -Fq '"stableLabel": "Estable"' "$spanish"
grep -Fq '"banner": "Achronyme 0.1.2 is published:' "$english"
grep -Fq '"banner": "Achronyme 0.1.2 publicado:' "$spanish"
grep -Fq '## 0.1.2 - detached verification patch' "$changelog_en"
grep -Fq '## 0.1.2 - parche de verificacion separada' "$changelog_es"
grep -Fq "label: '0.1.2 - Verification patch'" "$sidebar"
grep -Fq "es: '0.1.2 - Parche de verificacion'" "$sidebar"

if grep -Fq 'candidate:' "$release_data" ||
   grep -Fq 'candidateRevision:' "$release_data" ||
   grep -Fq 'releaseStatus.core.candidate' "$header" "$roadmap" "$landing" ||
   grep -Fq 'releaseStatus.editor.candidate' "$roadmap" "$landing" ||
   grep -Fiq 'candidate' "$sidebar" ||
   grep -Fiq 'candidato' "$sidebar" ||
   grep -Fq 'candidateLabel' "$english" "$spanish"; then
    printf 'error: published release surfaces still expose candidate state\n'
    exit 1
fi

printf 'release surfaces verified: core 0.1.2, editor 0.3.1, web 0.1.2 stable\n'
