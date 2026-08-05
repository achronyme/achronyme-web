#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
header="$project_root/src/components/Header.astro"
roadmap="$project_root/src/components/RoadmapTimeline.astro"
release_data="$project_root/src/data/release-status.ts"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
changelog_en="$project_root/src/content/docs-en/releases/changelog.mdx"
changelog_es="$project_root/src/content/docs-es/releases/changelog.mdx"

test "$(node -p "require('$project_root/package.json').version")" = "0.1.0"
test "$(node -p "require('$project_root/package-lock.json').version")" = "0.1.0"
test "$(node -p "require('$project_root/package-lock.json').packages[''].version")" = "0.1.0"

grep -Fq "stable: '0.0.1'" "$release_data"
grep -Fq "candidate: '0.1.0'" "$release_data"
grep -Fq "stable: '0.2.0'" "$release_data"
grep -Fq "candidate: '0.3.0'" "$release_data"
grep -Fq 'releaseStatus.core.stable' "$header"
grep -Fq 'releaseStatus.core.candidate' "$header"
grep -Fq 'releaseStatus.core.stable' "$roadmap"
grep -Fq 'releaseStatus.core.candidate' "$roadmap"
grep -Fq '"stableLabel": "Stable"' "$english"
grep -Fq '"candidateLabel": "Release candidate"' "$english"
grep -Fq '"stableLabel": "Estable"' "$spanish"
grep -Fq '"candidateLabel": "Candidato de release"' "$spanish"
grep -Fq '## 0.1.0 - release candidate (unpublished)' "$changelog_en"
grep -Fq '## 0.1.0 - candidato de release (sin publicar)' "$changelog_es"

if grep -Fq 'Latest release.' "$english" ||
   grep -Fq 'Ultimo release.' "$spanish"; then
    printf 'error: candidate release is still presented as published\n'
    exit 1
fi

printf 'release surfaces verified: stable 0.0.1, candidate 0.1.0\n'
