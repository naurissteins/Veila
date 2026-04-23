#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: build-rpm.sh <version> [output-dir] [source-tarball]}"
output_dir="${2:-dist}"
source_tarball="${3:-$output_dir/veila-${version}-x86_64-linux.tar.gz}"
release="${RPM_RELEASE:-1}"
architecture="${RPM_ARCHITECTURE:-x86_64}"
work_root="$(mktemp -d)"
rpmbuild_root="$work_root/rpmbuild"

cleanup() {
  rm -rf "$work_root"
}
trap cleanup EXIT

require_file() {
  local path="$1"
  if [ ! -f "$path" ]; then
    echo "missing required file: $path" >&2
    exit 1
  fi
}

require_file "$source_tarball"
require_file packaging/rpm/veila.pam

if ! command -v rpmbuild >/dev/null 2>&1; then
  echo "missing required command: rpmbuild" >&2
  exit 1
fi

source_basename="$(basename "$source_tarball")"
source_dirname="$(tar -tzf "$source_tarball" | awk -F/ 'NR == 1 { print $1 }')"

if [ -z "$source_dirname" ]; then
  echo "failed to determine source directory from $source_tarball" >&2
  exit 1
fi

rm -rf "$rpmbuild_root"
mkdir -p \
  "$rpmbuild_root/BUILD" \
  "$rpmbuild_root/BUILDROOT" \
  "$rpmbuild_root/RPMS" \
  "$rpmbuild_root/SOURCES" \
  "$rpmbuild_root/SPECS" \
  "$rpmbuild_root/SRPMS" \
  "$output_dir"

cp "$source_tarball" "$rpmbuild_root/SOURCES/$source_basename"
cp packaging/rpm/veila.pam "$rpmbuild_root/SOURCES/veila.pam"

cat >"$rpmbuild_root/SPECS/veila.spec" <<SPEC
%global debug_package %{nil}

Name:           veila
Version:        ${version}
Release:        ${release}%{?dist}
Summary:        Secure, elegant, and fast Wayland screen locker
License:        GPL-3.0-or-later
URL:            https://github.com/naurissteins/Veila
Source0:        ${source_basename}
Source1:        veila.pam
BuildArch:      ${architecture}

%description
Veila is a native Wayland screen locker focused on a modern look,
low lock latency, and a small runtime footprint.

%prep
%setup -q -n ${source_dirname}

%build

%install
install -d %{buildroot}%{_bindir}
install -m755 bin/veila %{buildroot}%{_bindir}/veila
install -m755 bin/veilad %{buildroot}%{_bindir}/veilad
install -m755 bin/veila-curtain %{buildroot}%{_bindir}/veila-curtain

install -d %{buildroot}%{_datadir}/veila
cp -a share/veila/bg %{buildroot}%{_datadir}/veila/
cp -a share/veila/fonts %{buildroot}%{_datadir}/veila/
cp -a share/veila/icons %{buildroot}%{_datadir}/veila/
cp -a share/veila/systemd %{buildroot}%{_datadir}/veila/
cp -a share/veila/themes %{buildroot}%{_datadir}/veila/

install -d %{buildroot}/usr/lib/systemd/user
install -m644 share/veila/systemd/veilad.service %{buildroot}/usr/lib/systemd/user/veilad.service

install -d %{buildroot}%{_sysconfdir}/pam.d
install -m644 %{SOURCE1} %{buildroot}%{_sysconfdir}/pam.d/veila

install -d %{buildroot}%{_docdir}/%{name}
install -m644 README.md %{buildroot}%{_docdir}/%{name}/README.md

install -d %{buildroot}%{_licensedir}/%{name}
install -m644 LICENSE %{buildroot}%{_licensedir}/%{name}/LICENSE

%files
%license %{_licensedir}/%{name}/LICENSE
%doc %{_docdir}/%{name}/README.md
%config(noreplace) %{_sysconfdir}/pam.d/veila
%{_bindir}/veila
%{_bindir}/veilad
%{_bindir}/veila-curtain
/usr/lib/systemd/user/veilad.service
%{_datadir}/veila/bg
%{_datadir}/veila/fonts
%{_datadir}/veila/icons
%{_datadir}/veila/systemd
%{_datadir}/veila/themes
SPEC

rpmbuild --define "_topdir $rpmbuild_root" -bb "$rpmbuild_root/SPECS/veila.spec"

find "$rpmbuild_root/RPMS" -type f -name '*.rpm' -exec cp {} "$output_dir/" \;
find "$output_dir" -maxdepth 1 -type f -name 'veila-*.rpm' -print
