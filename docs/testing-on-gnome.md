Testing MoyoDB on GNOME
There is no need to install the KDE Plasma desktop.
Kirigami is a library of Qt Quick components, not a requirement to run the Plasma desktop. MoyoDB needs the Qt 6 and Kirigami development/runtime libraries; installing those may bring in several KDE Framework libraries, but not the complete KDE desktop session. Kirigami is built on QML, Qt Quick and Qt Quick Controls and is intended to run on desktop Linux generally.
CXX-Qt requires a C/C++ compiler, Rust, CMake 3.24 or newer, Qt and a discoverable Qt qmake executable.
Fedora GNOME
sudo dnf install \
    gcc-c++ \
    cmake \
    ninja-build \
    pkgconf-pkg-config \
    git \
    curl \
    qt6-qtbase-devel \
    qt6-qtdeclarative-devel \
    kf6-kirigami-devel
Fedora provides the Qt 6 base, Qt Declarative and KF6 Kirigami development packages separately, so installing these does not require selecting the KDE Plasma desktop environment.
Optional native Wayland and desktop styling packages are:
sudo dnf install qt6-qtwayland kf6-qqc2-desktop-style
Debian 13 GNOME
sudo apt update

sudo apt install \
    build-essential \
    cmake \
    ninja-build \
    pkg-config \
    git \
    curl \
    qt6-base-dev \
    qt6-declarative-dev \
    libkirigami-dev
On Debian 13, libkirigami-dev depends on the Kirigami QML module and the necessary Qt 6 development packages. qt6-base-dev also supplies the Qt platform plugins and depends on qmake6.
Optional native Wayland support:
sudo apt install qt6-wayland
openSUSE Tumbleweed with GNOME
sudo zypper install \
    gcc-c++ \
    cmake \
    ninja \
    pkgconf-pkg-config \
    git \
    curl \
    qt6-base-devel \
    qt6-declarative-devel \
    kf6-kirigami-devel
Optional:
sudo zypper install qt6-wayland kf6-qqc2-desktop-style
openSUSE Tumbleweed carries Qt 6 Declarative and KF6 Kirigami as ordinary distribution packages; KDE’s own Kirigami setup documentation uses kf6-kirigami-devel and optionally the desktop style package.
Install Rust
Using rustup is the simplest way to install the stable Rust toolchain and Cargo:
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"
rustup update stable
Rust’s official Cargo documentation recommends rustup, which installs both Rust and Cargo.
Confirm that Qt can be located
Run:
qmake6 -query QT_VERSION 2>/dev/null || qmake -query QT_VERSION
It should print a Qt 6 version.
When qmake6 exists but there is no command named simply qmake, build with:
export QMAKE="$(command -v qmake6)"
Then confirm the essential tools:
rustc --version
cargo --version
c++ --version
cmake --version
"$QMAKE" -query QT_VERSION
Build and test the source
From the repository root:
cargo fmt --all --check
cargo test
cargo check -p moyodb-qt
If those pass, run the GUI with an existing MoyoDB project:
cargo run -p moyodb-qt -- ~/pattern-test
Create a small test project
Your friend does not need GoGoD or the full go4go collection. A directory containing perhaps 10–100 ordinary 19×19 SGF files is sufficient.
cargo run -- init ~/pattern-test

cargo run -- import-dir \
    ~/pattern-test \
    TestCorpus \
    2026 \
    /path/to/test-sgfs

cargo run -- build-position-index ~/pattern-test
Then:
cargo run -p moyodb-qt -- ~/pattern-test
GNOME-specific troubleshooting
When the program reports that org.kde.kirigami is not installed, the Kirigami runtime/QML package is missing. Install the distribution’s Kirigami package rather than the complete KDE desktop.
When controls have a styling problem, test with Qt’s built-in Fusion style:
QT_QUICK_CONTROLS_STYLE=Fusion \
cargo run -p moyodb-qt -- ~/pattern-test
When CXX-Qt cannot find Qt:
QMAKE="$(command -v qmake6)" \
cargo check -p moyodb-qt
When native Wayland causes a graphics problem, compare it with XWayland:
QT_QPA_PLATFORM=xcb \
cargo run -p moyodb-qt -- ~/pattern-test
