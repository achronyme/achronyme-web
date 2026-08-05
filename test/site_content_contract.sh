#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
landing="$project_root/src/components/PageGrid.astro"
english="$project_root/src/i18n/en.json"
spanish="$project_root/src/i18n/es.json"
sidebar="$project_root/src/data/docs-sidebar.ts"
commands_en="$project_root/src/content/docs-en/cli/commands.mdx"
commands_es="$project_root/src/content/docs-es/cli/commands.mdx"
proofs_en="$project_root/src/content/docs-en/zk-concepts/proof-generation.mdx"
proofs_es="$project_root/src/content/docs-es/zk-concepts/proof-generation.mdx"
hello_en="$project_root/src/content/docs-en/getting-started/hello-world.mdx"
hello_es="$project_root/src/content/docs-es/getting-started/hello-world.mdx"
release_en="$project_root/src/content/docs-en/releases/0.1.0.mdx"
release_es="$project_root/src/content/docs-es/releases/0.1.0.mdx"
docs_route_en="$project_root/src/pages/docs/[...slug].astro"
docs_route_es="$project_root/src/pages/es/docs/[...slug].astro"

grep -Fq 'structured concurrency' "$english"
grep -Fq 'concurrencia estructurada' "$spanish"
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

grep -Fq 'releases/0.1.0' "$sidebar"
grep -Fq 'fails closed' "$proofs_en"
grep -Fq 'falla de forma cerrada' "$proofs_es"
grep -Fq -- '--insecure-dev-setup' "$hello_en"
grep -Fq -- '--insecure-dev-setup' "$hello_es"
grep -Fq 'slug: "releases/0.1.0"' "$release_en"
grep -Fq 'slug: "releases/0.1.0"' "$release_es"
grep -Fq 'entry.data.slug ?? entry.id' "$docs_route_en"
grep -Fq 'entry.data.slug ?? entry.id' "$docs_route_es"
grep -Fq '`--manifest`' "$commands_en"
grep -Fq '`--manifest`' "$commands_es"
grep -Fq '`AKRON_ALLOW_READ`' "$commands_en"
grep -Fq '`AKRON_ALLOW_READ`' "$commands_es"

if grep -Fq 'planned path and is not yet wired' "$proofs_en" ||
   grep -Fq 'wires the native proving backend automatically' "$hello_en" ||
   grep -Fq 'conecta el backend nativo automaticamente' "$hello_es"; then
    printf 'error: documentation still describes the pre-0.1.0 trust model\n'
    exit 1
fi

printf 'site content contract verified: current releases, features, and proving trust\n'
