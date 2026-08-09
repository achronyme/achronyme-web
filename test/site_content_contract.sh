#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
landing="$project_root/src/components/PageGrid.astro"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
sidebar="$project_root/src/data/docs-sidebar.ts"
commands_en="$project_root/src/content/docs-en/cli/commands.mdx"
commands_es="$project_root/src/content/docs-es/cli/commands.mdx"
concurrency_en="$project_root/src/content/docs-en/language/concurrency-and-io.mdx"
concurrency_es="$project_root/src/content/docs-es/language/concurrency-and-io.mdx"
diagnostics_en="$project_root/src/content/docs-en/language/diagnostics.mdx"
diagnostics_es="$project_root/src/content/docs-es/language/diagnostics.mdx"
modules_en="$project_root/src/content/docs-en/language/modules.mdx"
modules_es="$project_root/src/content/docs-es/language/modules.mdx"
r1cs_en="$project_root/src/content/docs-en/zk-concepts/r1cs.mdx"
r1cs_es="$project_root/src/content/docs-es/zk-concepts/r1cs.mdx"
proofs_en="$project_root/src/content/docs-en/zk-concepts/proof-generation.mdx"
proofs_es="$project_root/src/content/docs-es/zk-concepts/proof-generation.mdx"
hello_en="$project_root/src/content/docs-en/getting-started/hello-world.mdx"
hello_es="$project_root/src/content/docs-es/getting-started/hello-world.mdx"
release_en="$project_root/src/content/docs-en/releases/0.1.1.mdx"
release_es="$project_root/src/content/docs-es/releases/0.1.1.mdx"
trust_release_en="$project_root/src/content/docs-en/releases/0.1.0.mdx"
trust_release_es="$project_root/src/content/docs-es/releases/0.1.0.mdx"
dossier_url="https://github.com/achronyme/achronyme/blob/cd0601402e03bbdff4b4ac4cae88c0e672d53ac8/release-evidence/0.1.0/final/README.md"
docs_route_en="$project_root/src/pages/docs/[...slug].astro"
docs_route_es="$project_root/src/pages/es/docs/[...slug].astro"

grep -Fq '"title": "Structured concurrency"' "$english"
grep -Fq '"title": "Concurrencia estructurada"' "$spanish"
grep -Fq -- '--insecure-dev-setup' "$landing"
grep -Fq -- '--trusted-key-dir' "$landing"

if grep -Eq '4,400\+|0 JS deps|37 opcodes|40 opcodes' \
    "$landing" "$english" "$spanish"; then
    printf 'error: landing still contains stale counters or dependency claims\n'
    exit 1
fi

for needle in \
    '## `verify`' \
    '## `trusted-setup`' \
    '## `aot`' \
    '`--insecure-dev-setup`' \
    '`--trusted-key-dir <dir>`' \
    '`--allow-read <dir>`' \
    '`--max-channel-operations <count>`' \
    '`--blocking-workers <count>`' \
    '`--engine <engine>`'
do
    grep -Fq "$needle" "$commands_en"
done

for needle in \
    '## `verify`' \
    '## `trusted-setup`' \
    '## `aot`' \
    '`--insecure-dev-setup`' \
    '`--trusted-key-dir <dir>`' \
    '`--allow-read <dir>`' \
    '`--max-channel-operations <count>`' \
    '`--blocking-workers <count>`' \
    '`--engine <engine>`'
do
    grep -Fq "$needle" "$commands_es"
done

grep -Fq 'releases/0.1.1' "$sidebar"
grep -Fq 'releases/0.1.0' "$sidebar"
grep -Fq 'fails closed' "$proofs_en"
grep -Fq 'falla de forma cerrada' "$proofs_es"
grep -Fq -- '--insecure-dev-setup' "$hello_en"
grep -Fq -- '--insecure-dev-setup' "$hello_es"
grep -Fq 'slug: "releases/0.1.1"' "$release_en"
grep -Fq 'slug: "releases/0.1.1"' "$release_es"
grep -Fq 'Imported-module diagnostics' "$release_en"
grep -Fq 'Diagnosticos de modulos importados' "$release_es"
grep -Fq 'Tilino Lab' "$release_en" "$release_es"
grep -Fq "$dossier_url" "$trust_release_en"
grep -Fq "$dossier_url" "$trust_release_es"
grep -Fq 'entry.data.slug ?? entry.id' "$docs_route_en"
grep -Fq 'entry.data.slug ?? entry.id' "$docs_route_es"
grep -Fq '`--manifest`' "$commands_en"
grep -Fq '`--manifest`' "$commands_es"
grep -Fq '`AKRON_ALLOW_READ`' "$commands_en"
grep -Fq '`AKRON_ALLOW_READ`' "$commands_es"
grep -Fq 'explicit `spawn` and implicit `await` child tasks' "$commands_en"
grep -Fq 'simultaneously live structured scopes' "$commands_en"
grep -Fq 'tareas hijas de `spawn` explícito y `await` implícito' "$commands_es"
grep -Fq 'scopes estructurados vivos simultáneamente' "$commands_es"
grep -Fq 'the root task is excluded' "$concurrency_en"
grep -Fq 'la tarea raíz queda excluida' "$concurrency_es"
grep -Fq 'canonical imported file' "$diagnostics_en"
grep -Fq 'archivo importado canónico' "$diagnostics_es"
grep -Fq 'Typed array captures' "$modules_en"
grep -Fq 'captures de arrays tipados' "$modules_es"
grep -Fq 'PRE-OPTIMIZATION ESTIMATE' "$r1cs_en" "$r1cs_es"
grep -Fq 'FINAL PROVING CONSTRAINTS' "$r1cs_en" "$r1cs_es"

if grep -Fq 'planned path and is not yet wired' "$proofs_en" ||
   grep -Fq 'wires the native proving backend automatically' "$hello_en" ||
   grep -Fq 'conecta el backend nativo automaticamente' "$hello_es"; then
    printf 'error: documentation still describes the pre-0.1.0 trust model\n'
    exit 1
fi

printf 'site content contract verified: current releases, features, and proving trust\n'
