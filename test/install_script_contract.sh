#!/bin/sh

set -eu

project_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
test_root=$(mktemp -d)
trap 'rm -rf "$test_root"' EXIT HUP INT TERM

artifact="achronyme-linux-x86_64"
release_root="$test_root/release"
bundle_root="$test_root/bundle/$artifact"
fake_bin="$test_root/fake-bin"
install_root="$test_root/install"
request_log="$test_root/requests.log"

grep -Fq -- '--version 0.1.0' "$project_root/public/install.sh"
if grep -Fq '0.0.1' "$project_root/public/install.sh"; then
    printf 'error: installer documentation still references 0.0.1\n'
    exit 1
fi

mkdir -p "$release_root" "$bundle_root/bin" "$bundle_root/lib" "$fake_bin"

cat > "$bundle_root/bin/ach" <<'EOF'
#!/bin/sh
printf 'ach 0.1.0\n'
EOF
chmod +x "$bundle_root/bin/ach"
printf 'aot-runtime\n' > "$bundle_root/lib/libakron_aot_runtime.a"

tar -C "$test_root/bundle" -czf "$release_root/$artifact.tar.gz" "$artifact"
(
    cd "$release_root"
    sha256sum "$artifact.tar.gz" > "$artifact.tar.gz.sha256"
)

cat > "$fake_bin/uname" <<'EOF'
#!/bin/sh
case "${1:-}" in
    -s) printf 'Linux\n' ;;
    -m) printf 'x86_64\n' ;;
    *) printf 'Linux\n' ;;
esac
EOF
chmod +x "$fake_bin/uname"

cat > "$fake_bin/curl" <<'EOF'
#!/bin/sh
set -eu

output=""
url=""
wants_status="false"

while [ "$#" -gt 0 ]; do
    case "$1" in
        -o)
            output="$2"
            shift 2
            ;;
        -w)
            wants_status="true"
            shift 2
            ;;
        http://*|https://*)
            url="$1"
            shift
            ;;
        *)
            shift
            ;;
    esac
done

printf '%s\n' "$url" >> "$FAKE_REQUEST_LOG"

case "$url" in
    */releases/latest)
        printf '{"tag_name":"v0.1.0"}\n'
        ;;
    *)
        asset=${url##*/}
        if [ -f "$FAKE_RELEASE_ROOT/$asset" ]; then
            cp "$FAKE_RELEASE_ROOT/$asset" "$output"
            if [ "$wants_status" = "true" ]; then
                printf '200'
            fi
        else
            if [ "$wants_status" = "true" ]; then
                printf '404'
            fi
            exit 22
        fi
        ;;
esac
EOF
chmod +x "$fake_bin/curl"

PATH="$fake_bin:$PATH" \
FAKE_RELEASE_ROOT="$release_root" \
FAKE_REQUEST_LOG="$request_log" \
ACHRONYME_BIN_DIR="$install_root/bin" \
ACHRONYME_LIB_DIR="$install_root/lib" \
    sh "$project_root/public/install.sh"

test -x "$install_root/bin/ach"
test -f "$install_root/lib/libakron_aot_runtime.a"
test "$("$install_root/bin/ach" --version)" = "ach 0.1.0"
cmp "$bundle_root/lib/libakron_aot_runtime.a" \
    "$install_root/lib/libakron_aot_runtime.a"

grep -qx \
    "https://api.github.com/repos/achronyme/achronyme/releases/latest" \
    "$request_log"
grep -qx \
    "https://github.com/achronyme/achronyme/releases/download/v0.1.0/$artifact.tar.gz" \
    "$request_log"
grep -qx \
    "https://github.com/achronyme/achronyme/releases/download/v0.1.0/$artifact.tar.gz.sha256" \
    "$request_log"

printf 'installer contract verified: checksum, binary, and AOT runtime\n'
