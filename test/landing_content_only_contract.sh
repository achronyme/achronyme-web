#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
landing="$project_root/src/components/PageGrid.astro"
layout="$project_root/src/layouts/Layout.astro"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
release_en="$project_root/src/content/docs-en/releases/0.1.2.mdx"
release_es="$project_root/src/content/docs-es/releases/0.1.2.mdx"

# Preserve the established landing composition. Release copy may change, but
# the content refresh must not replace the site's component and style system.
grep -Fq "import TerminalMockup from './TerminalMockup.astro'" "$landing"
grep -Fq "import HeroCircuit from './HeroCircuit.astro'" "$landing"
grep -Fq "import TriArchitecture from './TriArchitecture.astro'" "$landing"
grep -Fq '<HeroCircuit />' "$landing"
grep -Fq '<TriArchitecture locale={locale} />' "$landing"
grep -Fq 'id="features-wrapper"' "$landing"

if grep -Fq "from './landing/" "$landing" ||
   grep -Fq "../styles/landing.css" "$layout" ||
   test -e "$project_root/src/styles/landing.css" ||
   test -d "$project_root/src/components/landing"; then
    printf 'error: content refresh replaced the established landing design\n'
    exit 1
fi

# Keep the current product truth and the canonical release documentation.
grep -Fq '"cta": "Install 0.1.2"' "$english"
grep -Fq '"cta": "Instalar 0.1.2"' "$spanish"
grep -Fq '"banner": "Achronyme 0.1.2 is published:' "$english"
grep -Fq '"banner": "Achronyme 0.1.2 publicado:' "$spanish"
grep -Fq '"title": "Structured concurrency"' "$english"
grep -Fq '"title": "Concurrencia estructurada"' "$spanish"
grep -Fq 'slug: "releases/0.1.2"' "$release_en"
grep -Fq 'slug: "releases/0.1.2"' "$release_es"

printf 'landing contract verified: established design with current release content\n'
