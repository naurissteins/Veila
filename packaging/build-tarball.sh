#!/usr/bin/env bash
set -euo pipefail

# Package payloads must be readable regardless of the developer shell umask.
umask 022

version="${1:?usage: build-tarball.sh <version> <arch> [output-dir]}"
architecture="${2:?usage: build-tarball.sh <version> <arch> [output-dir]}"
output_dir="${3:-dist}"
binary="${CARGO_TARGET_DIR:-target}/release/veila"

# Labels become archive paths; keep them single, portable directory components.
for label in "$version" "$architecture"; do
  [[ "$label" =~ ^[[:alnum:]][[:alnum:].+_~-]*$ ]] || { echo "invalid artifact label: $label" >&2; exit 1; }
done
staging="$(mktemp -d)"
trap 'rm -rf "$staging"' EXIT
name="veila-${version}-${architecture}"
root="$staging/$name"
mkdir -p "$root/bin" "$root/share/man/man1" "$root/share/veila" "$output_dir"
install -m755 "$binary" "$root/bin/veila"
ln -s veila "$root/bin/veilad"
install -m644 docs/man/veila.1 "$root/share/man/man1/veila.1"
cp -R assets/{fonts,icons,systemd,themes} "$root/share/veila/"
install -m644 README.md LICENSE "$root/"
find "$root" -type d -exec chmod 755 {} +
find "$root" -type f -exec chmod 644 {} +
chmod 755 "$root/bin/veila"
tar -C "$staging" -czf "$output_dir/$name.tar.gz" "$name"
printf '%s\n' "$output_dir/$name.tar.gz"
