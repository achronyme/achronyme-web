#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
source_version="0.1.1"
source_revision="44ab7868b81d69f6a5bb21f792ef5769eec8311f"
metadata_file=$(mktemp)
trap 'rm -f "$metadata_file"' EXIT HUP INT TERM

cd "$project_root"

test "$(grep -c "rev = \"$source_revision\"" server/Cargo.toml)" -eq 13
grep -qx 'channel = "1.96.0"' server/rust-toolchain.toml
grep -Fqx \
    '      - run: cargo test --release --manifest-path server/Cargo.toml --locked' \
    .github/workflows/ci.yml
grep -Fq 'EXPECTED_VERSION=' .github/workflows/deploy.yml
grep -Fq 'http://127.0.0.1:3100/version' .github/workflows/deploy.yml

cargo metadata --manifest-path server/Cargo.toml --locked --format-version 1 \
    > "$metadata_file"

METADATA_FILE="$metadata_file" \
SOURCE_VERSION="$source_version" \
SOURCE_REVISION="$source_revision" \
    node <<'EOF'
const fs = require("node:fs");
const metadata = JSON.parse(fs.readFileSync(process.env.METADATA_FILE, "utf8"));
const upstream = metadata.packages.filter((pkg) =>
  pkg.source && pkg.source.includes("github.com/achronyme/achronyme"));

if (upstream.length === 0 ||
    upstream.some((pkg) => pkg.version !== process.env.SOURCE_VERSION) ||
    upstream.some((pkg) => !pkg.source.includes(process.env.SOURCE_REVISION))) {
  throw new Error("ach-server dependencies do not match the Achronyme release pin");
}
EOF

printf 'server source contract verified: Achronyme %s\n' "$source_version"
