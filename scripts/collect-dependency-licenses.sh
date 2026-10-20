#!/bin/sh
# Collect notices from downloaded Cargo packages; never copy font assets.
set -eu
cargo_root=$1
target_root=$2
case "$target_root" in /*) ;; *) echo 'License output must be absolute' >&2; exit 2 ;; esac
mkdir -p "$target_root"
count=0
for package_dir in "$cargo_root"/registry/src/*/*; do
    [ -d "$package_dir" ] || continue
    package_target="$target_root/$(basename "$package_dir")"
    mkdir -p "$package_target"
    cp "$package_dir/Cargo.toml" "$package_target/Cargo.toml"
    (
        cd "$package_dir"
        find . -type f \( -iname '*license*' -o -iname '*copying*' -o -iname '*notice*' -o -iname 'copyright*' -o -iname 'unlicense*' \) \
            -exec cp --parents '{}' "$package_target/" ';'
    )
    count=$((count + 1))
done
if [ "$count" -eq 0 ]; then
    echo 'No downloaded Cargo packages found for license collection' >&2
    exit 1
fi
toolchain_root=$(rustc --print sysroot)
mkdir -p "$target_root/rust-toolchain"
for notice in "$toolchain_root"/share/doc/rust/COPYRIGHT* "$toolchain_root"/share/doc/rust/LICENSE*; do
    [ ! -f "$notice" ] || cp "$notice" "$target_root/rust-toolchain/"
done
printf 'Collected notices from %s Cargo packages\n' "$count"
