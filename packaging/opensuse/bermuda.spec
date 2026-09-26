# Local Tumbleweed release candidate. OBS policy review is a later step.
Name:           bermuda
Version:        0.8.1
Release:        1
Summary:        Go game database and study application
License:        GPL-3.0-or-later
URL:            https://github.com/gerryg1957/Bermuda
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  cargo
BuildRequires:  rust
BuildRequires:  gcc-c++
BuildRequires:  cmake
BuildRequires:  pkgconf-pkg-config
BuildRequires:  qt6-base-devel
BuildRequires:  qt6-declarative-devel
BuildRequires:  desktop-file-utils
# Embedded QML is not visible to RPM's ordinary import-file scanner.
Requires:       qt6-declarative-imports
Requires:       kf6-kirigami-imports
Requires:       kf6-qqc2-desktop-style
Recommends:     qt6-wayland

%description
Bermuda imports SGF collections, replays games, and searches local board
patterns with continuation statistics. Optional KataGo analysis is configured
inside the application. Game collections and KataGo are supplied separately.

%prep
%setup -q

%build
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_TARGET_DIR="$PWD/target"
cargo build --release --frozen --workspace

%check
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_TARGET_DIR="$PWD/target"
cargo test --release --frozen -p bermuda --lib
desktop-file-validate packaging/org.bermuda.app.desktop

%install
install -D -m 0755 target/release/bermuda-qt %{buildroot}%{_bindir}/bermuda-qt
install -D -m 0755 target/release/bermuda %{buildroot}%{_bindir}/bermuda
install -D -m 0644 packaging/org.bermuda.app.desktop %{buildroot}%{_datadir}/applications/org.bermuda.app.desktop

%files
%license LICENCE third-party-licenses
%doc README.md docs/katago-setup.md docs/building-on-linux.md
%{_bindir}/bermuda
%{_bindir}/bermuda-qt
%{_datadir}/applications/org.bermuda.app.desktop

%changelog
* Sat Sep 26 2026 Bermuda contributors
- Keep game and joseki trees visible when resizing panes.

* Fri Sep 25 2026 Bermuda contributors
- Prepare initial 0.8.0 Tumbleweed package.
