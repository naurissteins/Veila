#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: build-deb.sh <version> [output-dir]}"
output_dir="${2:-dist}"
revision="${DEB_REVISION:-1}"
architecture="${DEB_ARCHITECTURE:-amd64}"
package_version="${version}-${revision}"

if [[ ! "$package_version" =~ ^[0-9] ]]; then
  echo "invalid Debian package version: $package_version" >&2
  echo "Debian versions must start with a digit, for example 0.1.5-1 or 0.1.5~manual-1." >&2
  exit 1
fi

package_root="$(mktemp -d)"
staging="$package_root/veila_${package_version}_${architecture}"
deb_path="$output_dir/veila_${package_version}_${architecture}.deb"

cleanup() {
  rm -rf "$package_root"
}
trap cleanup EXIT

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_file target/release/veila
require_file target/release/veilad
require_file target/release/veila-curtain
require_file LICENSE
require_file README.md
require_file assets/systemd/veilad.service

rm -rf "$staging" "$deb_path"
mkdir -p \
  "$staging/DEBIAN" \
  "$staging/etc/pam.d" \
  "$staging/usr/bin" \
  "$staging/usr/lib/systemd/user" \
  "$staging/usr/share/doc/veila" \
  "$staging/usr/share/veila" \
  "$output_dir"

install -m755 target/release/veila "$staging/usr/bin/veila"
install -m755 target/release/veilad "$staging/usr/bin/veilad"
install -m755 target/release/veila-curtain "$staging/usr/bin/veila-curtain"

install -m644 assets/systemd/veilad.service "$staging/usr/lib/systemd/user/veilad.service"
install -m644 README.md "$staging/usr/share/doc/veila/README.md"
install -m644 LICENSE "$staging/usr/share/doc/veila/copyright"

cp -R assets/bg "$staging/usr/share/veila/"
cp -R assets/fonts "$staging/usr/share/veila/"
cp -R assets/icons "$staging/usr/share/veila/"
cp -R assets/systemd "$staging/usr/share/veila/"
cp -R assets/themes "$staging/usr/share/veila/"

cat >"$staging/etc/pam.d/veila" <<'PAM'
# PAM service for Veila screen locker.
@include common-auth
@include common-account
PAM

cat >"$staging/DEBIAN/conffiles" <<'CONFFILES'
/etc/pam.d/veila
CONFFILES

installed_size="$(du -sk "$staging" | awk '{print $1}')"
cat >"$staging/DEBIAN/control" <<CONTROL
Package: veila
Version: $package_version
Section: utils
Priority: optional
Architecture: $architecture
Maintainer: Nauris Steins <me@naurissteins.com>
Installed-Size: $installed_size
Depends: libc6, libpam0g, libwayland-client0, libxkbcommon0
Recommends: systemd
Homepage: https://github.com/naurissteins/Veila
Description: Secure, elegant, and fast Wayland screen locker
 Veila is a native Wayland screen locker focused on a modern look,
 low lock latency, and a small runtime footprint.
CONTROL

dpkg-deb --build --root-owner-group "$staging" "$deb_path"
echo "$deb_path"
