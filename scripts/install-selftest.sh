#!/bin/sh
# Hermetic contract tests for the withpointbreak/pointbreak Unix installer.

set -eu

repo_root=$(CDPATH='' cd -- "$(dirname "$0")/.." && pwd)
installer="${repo_root}/scripts/install.sh"
temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/pointbreak-installer-test.XXXXXX")
cleanup() {
    rm -rf "$temp_dir"
}
trap cleanup 0
trap 'exit 1' 1 2 3 15

case "$(uname -s)" in
    Darwin) os=darwin ;;
    Linux)
        if [ -f /etc/alpine-release ] \
            || { command -v ldd >/dev/null 2>&1 && ldd --version 2>&1 | grep -qi musl; }; then
            os=alpine
        else
            os=linux
        fi
        ;;
    *)
        printf 'installer self-test only supports macOS and Linux\n' >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64|amd64) arch=x64 ;;
    arm64|aarch64) arch=arm64 ;;
    *)
        printf 'unsupported self-test architecture: %s\n' "$(uname -m)" >&2
        exit 1
        ;;
esac

tag=v9.8.7-test
version=${tag#v}
target="${os}-${arch}"
archive="pointbreak-${version}-${target}.tar.gz"
release_dir="${temp_dir}/releases/${tag}"
payload_dir="${temp_dir}/payload"
install_dir="${temp_dir}/bin"
destination="${install_dir}/pointbreak"
neighbor="${install_dir}/shore"
mkdir -p "$release_dir" "$install_dir"

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

write_candidate() {
    candidate_version=$1
    installed_version=$2
    output=$3
    cat > "$output" <<EOF
#!/bin/sh
if [ "\$#" -ne 3 ] || [ "\$1" != version ] || [ "\$2" != --format ] || [ "\$3" != json ]; then
    exit 64
fi
candidate_version='$candidate_version'
case "\$0" in
    */pointbreak) candidate_version='$installed_version' ;;
esac
printf '{"schema":"pointbreak.version","version":1,"cliVersion":"%s","documents":{"pointbreak.version":1},"diagnostics":[]}\\n' "\$candidate_version"
EOF
    chmod +x "$output"
}

make_archive() {
    candidate_version=$1
    installed_version=$2
    layout=$3
    rm -rf "$payload_dir"
    mkdir -p "$payload_dir"
    write_candidate "$candidate_version" "$installed_version" "${payload_dir}/pointbreak"
    cp "${repo_root}/LICENSE" "${repo_root}/NOTICE" "$payload_dir/"
    case "$layout" in
        exact)
            tar -czf "${release_dir}/${archive}" -C "$payload_dir" \
                pointbreak LICENSE NOTICE
            ;;
        extra)
            printf 'unexpected payload\n' > "${payload_dir}/unexpected.txt"
            tar -czf "${release_dir}/${archive}" -C "$payload_dir" \
                pointbreak LICENSE NOTICE unexpected.txt
            ;;
        *)
            printf 'unknown fixture layout: %s\n' "$layout" >&2
            exit 1
            ;;
    esac
}

write_checksum() {
    checksum=$(sha256_file "${release_dir}/${archive}")
    printf '%s  %s\n' "$checksum" "$archive" > "${release_dir}/checksums.txt"
}

write_invalid_checksum() {
    printf '%064d  %s\n' 0 "$archive" > "${release_dir}/checksums.txt"
}

write_previous_install() {
    cat > "$destination" <<'EOF'
#!/bin/sh
printf 'previous pointbreak\n'
EOF
    chmod +x "$destination"
}

write_neighbor() {
    printf 'arbitrary neighboring bytes\nnot an executable\n' > "$neighbor"
}

prepare_upgrade() {
    mkdir -p "$install_dir"
    write_previous_install
    write_neighbor
    previous_hash=$(sha256_file "$destination")
    neighbor_hash=$(sha256_file "$neighbor")
}

assert_neighbor_unchanged() {
    test -f "$neighbor"
    test "$(sha256_file "$neighbor")" = "$neighbor_hash"
}

assert_previous_restored() {
    test -x "$destination"
    test "$(sha256_file "$destination")" = "$previous_hash"
    assert_neighbor_unchanged
    if find "$install_dir" -maxdepth 1 \
        \( -name '.pointbreak-install.*' -o -name '.pointbreak-backup.*' \
        -o -name '.pointbreak-transaction.*' \) \
        | grep -q .; then
        printf 'installer left transaction files behind\n' >&2
        exit 1
    fi
}

run_installer() {
    POINTBREAK_INSTALLER_FIXTURE_ROOT="${temp_dir}/releases" \
        "$installer" --version="$tag" --prefix="$install_dir"
}

expect_failure() {
    scenario=$1
    shift
    if "$@" > "${temp_dir}/${scenario}.log" 2>&1; then
        printf 'installer accepted %s\n' "$scenario" >&2
        exit 1
    fi
    assert_previous_restored
}

help_output=$($installer --help)
printf '%s\n' "$help_output" | grep -F 'Pointbreak Review installer' >/dev/null
if printf '%s\n' "$help_output" | grep -i 'shore' >/dev/null; then
    printf 'installer help teaches a second executable\n' >&2
    exit 1
fi

# Fresh install: create only a regular Pointbreak executable.
make_archive "$version" "$version" exact
write_checksum
fresh_output=$(SHELL=/bin/zsh run_installer)
printf '%s\n' "$fresh_output"
test -x "$destination"
test ! -L "$destination"
test ! -e "$neighbor"
test "$($destination version --format json)" = \
    "{\"schema\":\"pointbreak.version\",\"version\":1,\"cliVersion\":\"$version\",\"documents\":{\"pointbreak.version\":1},\"diagnostics\":[]}"
printf '%s\n' "$fresh_output" | grep -F "Installed Pointbreak Review $version to $destination" >/dev/null
printf '%s\n' "$fresh_output" | grep -F "export PATH=\"${install_dir}:\$PATH\"" >/dev/null
printf '%s\n' "$fresh_output" | grep -F 'Then run: pointbreak --help' >/dev/null
if printf '%s\n' "$fresh_output" | grep -i 'shore' >/dev/null; then
    printf 'installer success output teaches a second executable\n' >&2
    exit 1
fi

# Upgrade: replace only Pointbreak and preserve an arbitrary neighbor byte-for-byte.
prepare_upgrade
make_archive "$version" "$version" exact
write_checksum
upgrade_output=$(run_installer)
printf '%s\n' "$upgrade_output"
test "$(sha256_file "$destination")" = "$(sha256_file "${payload_dir}/pointbreak")"
test ! -L "$destination"
assert_neighbor_unchanged

# A hostile collision at the old predictable stage name must not be followed or removed.
prepare_upgrade
make_archive "$version" "$version" exact
write_checksum
collision_path_file="${temp_dir}/collision-path"
collision_output=$(INSTALLER="$installer" INSTALL_DIR="$install_dir" NEIGHBOR="$neighbor" \
    TAG="$tag" COLLISION_PATH_FILE="$collision_path_file" \
    POINTBREAK_INSTALLER_FIXTURE_ROOT="${temp_dir}/releases" /bin/sh -c '
        collision="${INSTALL_DIR}/.pointbreak-install.$$"
        ln -s "$NEIGHBOR" "$collision"
        printf "%s\n" "$collision" > "$COLLISION_PATH_FILE"
        set -- --version="$TAG" --prefix="$INSTALL_DIR"
        . "$INSTALLER"
    ')
printf '%s\n' "$collision_output"
collision_path=$(sed -n '1p' "$collision_path_file")
test -L "$collision_path"
test "$(readlink "$collision_path")" = "$neighbor"
test "$(sha256_file "$neighbor")" = "$neighbor_hash"
test -f "$destination"
test ! -L "$destination"
test "$(sha256_file "$destination")" = "$(sha256_file "${payload_dir}/pointbreak")"
rm -f "$collision_path"

# Every failure must preserve the prior destination and the arbitrary neighbor.
prepare_upgrade
make_archive "$version" "$version" exact
write_invalid_checksum
expect_failure checksum-failure run_installer
grep -F 'checksum mismatch' "${temp_dir}/checksum-failure.log" >/dev/null

prepare_upgrade
make_archive "$version" "$version" extra
write_checksum
expect_failure archive-layout-failure run_installer
grep -F 'invalid archive layout' "${temp_dir}/archive-layout-failure.log" >/dev/null

prepare_upgrade
make_archive 9.8.6-test 9.8.6-test exact
write_checksum
expect_failure version-mismatch run_installer
grep -F 'version document did not match' "${temp_dir}/version-mismatch.log" >/dev/null

prepare_upgrade
make_archive "$version" "$version" exact
write_checksum
expect_failure replacement-failure env POINTBREAK_INSTALLER_FIXTURE_ROOT="${temp_dir}/releases" \
    POINTBREAK_INSTALLER_TEST_FAIL_REPLACE=1 "$installer" --version="$tag" --prefix="$install_dir"
grep -F 'could not replace' "${temp_dir}/replacement-failure.log" >/dev/null

prepare_upgrade
make_archive "$version" 9.8.6-test exact
write_checksum
expect_failure post-replacement-verification-failure run_installer
grep -F 'installed Pointbreak version document did not match' \
    "${temp_dir}/post-replacement-verification-failure.log" >/dev/null

# The implementation itself must have no second executable path or cleanup branch.
if grep -i 'shore' "$installer" >/dev/null; then
    printf 'installer implementation references a neighboring executable\n' >&2
    exit 1
fi

printf 'install.sh self-test ok\n'
