# Building and Running Bermuda on Linux

**Quick-start guide for testers**  
**Updated: 26 August 2026**

Bermuda is a desktop application for studying professional Go games. It is written in Rust and uses Qt 6, QML, CXX-Qt and KDE Kirigami.

You do **not** need to use the KDE Plasma desktop to run Bermuda. The same Bermuda source builds and runs under KDE Plasma, GNOME, Cinnamon, XFCE and other Linux desktop environments. The packages you install depend mainly on your **Linux distribution**, not on your desktop environment.

This guide is intended to get a tester from a normal Linux installation to a running Bermuda application with as little developer knowledge as possible.

> **Current status:** Bermuda does not yet have packaged releases, so testers currently build it from source. Linux is the tested target. Windows is not covered by this guide and should not yet be regarded as a supported Bermuda platform.

## Quick overview

For most testers the process is:

1. Install the build packages for your distribution.
2. Install Rust with `rustup`.
3. Clone Bermuda from GitHub.
4. Run `cargo run --release -p bermuda-qt`.
5. On first launch, let the graphical application create the Games Database.

No command-line database creation or indexing is required for normal use.

## 1. Install the build requirements

Choose the section for your Linux distribution.

### openSUSE Tumbleweed

Bermuda is principally developed and tested on openSUSE Tumbleweed.

```bash
sudo zypper install \
    gcc-c++ \
    cmake \
    git \
    curl \
    pkgconf-pkg-config \
    qt6-base-devel \
    qt6-declarative-devel \
    qt6-quickcontrols2-devel \
    qt6-wayland \
    kf6-kirigami-devel \
    kf6-qqc2-desktop-style-devel
```

A known-working Bermuda development machine currently has Rust 1.97.0, CMake 4.4.2, Qt 6.11.2 and KDE Frameworks/Kirigami 6.29.0. These are reference versions, not declarations of Bermuda's minimum supported versions.

### Fedora

The same instructions apply whether Fedora is running GNOME or KDE Plasma.

```bash
sudo dnf install \
    gcc-c++ \
    cmake \
    git \
    curl \
    pkgconf-pkg-config \
    qt6-qtbase-devel \
    qt6-qtdeclarative-devel \
    qt6-qtwayland \
    kf6-kirigami-devel \
    kf6-qqc2-desktop-style
```

Installing the Kirigami and KDE Qt Quick Controls packages does **not** install or require the complete KDE Plasma desktop.

### Debian 13

```bash
sudo apt update

sudo apt install \
    build-essential \
    cmake \
    git \
    curl \
    pkg-config \
    qt6-base-dev \
    qt6-declarative-dev \
    qt6-wayland \
    libkirigami-dev \
    qml6-module-qtcore \
    qml6-module-qtquick-dialogs \
    qml6-module-org-kde-desktop
```

### Ubuntu 26.04 LTS or later

```bash
sudo apt update

sudo apt install \
    build-essential \
    cmake \
    git \
    curl \
    pkg-config \
    qt6-base-dev \
    qt6-declarative-dev \
    qt6-wayland \
    libkirigami-dev \
    qml6-module-qtcore \
    qml6-module-qtquick-dialogs \
    qml6-module-org-kde-desktop
```

Ubuntu 24.04 LTS predates the straightforward Qt 6 / KDE Frameworks 6 package combination used by current Bermuda development. For early Bermuda testing, Ubuntu 26.04 LTS or later is recommended.

## 2. Install Rust

The simplest method is Rust's standard `rustup` installer:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Accept the default installation when prompted.

Then load the Rust environment into the current terminal:

```bash
source "$HOME/.cargo/env"
```

You can check that Rust and Cargo are available with:

```bash
rustc --version
cargo --version
```

## 3. Download Bermuda

Use the HTTPS GitHub address so that no SSH key setup is required:

```bash
git clone https://github.com/gerryg1957/Bermuda.git
cd Bermuda
```

If you already have a Bermuda checkout, skip this step and use your existing directory.

## 4. Build and run Bermuda

From the Bermuda repository directory:

```bash
cargo run --release -p bermuda-qt
```

The first build takes longer because Rust and CXX-Qt must compile Bermuda and its Rust dependencies. Later builds are much quicker because Cargo reuses what it has already built.

If the build succeeds, the Bermuda graphical application opens automatically.

## 5. First launch

On its first normal launch, Bermuda offers to create a managed **Games Database**.

1. Choose a folder containing SGF game files.
2. Enter a name for the source.
3. Enter a source version or release identifier.
4. Select **Create**.

Bermuda imports the games and prepares the search data from the graphical application. Normal testers do **not** need to run command-line database creation, import or indexing commands.

On subsequent launches, Bermuda opens the Games Database automatically.

To add another collection or a later release of an existing collection, use:

**Database -> Add Games...**

## Updating Bermuda

From the Bermuda source directory:

```bash
git pull
cargo run --release -p bermuda-qt
```

Cargo recompiles only what is needed after the update.

## Troubleshooting

Most testers should not need this section. Use it only if the normal build does not work.

### CXX-Qt cannot find Qt

First check whether a Qt 6 `qmake` is available:

```bash
qmake6 -query QT_VERSION 2>/dev/null || qmake -query QT_VERSION
```

It should print a Qt 6 version.

Some distributions provide `qmake6` but no command named simply `qmake`. Bermuda builds successfully in that arrangement. If CXX-Qt nevertheless reports that it cannot locate Qt, try:

```bash
export QMAKE="$(command -v qmake6)"
cargo run --release -p bermuda-qt
```

### Kirigami cannot be loaded

If Bermuda reports that `org.kde.kirigami` is not installed, the Kirigami runtime/QML module is missing. Install the Kirigami package for your distribution; you do not need to install KDE Plasma.

### KDE-style controls cannot be loaded

Bermuda normally uses KDE's Qt Quick Controls desktop style. Check that the relevant package is installed:

- openSUSE: `kf6-qqc2-desktop-style-devel`
- Fedora: `kf6-qqc2-desktop-style`
- Debian/Ubuntu: `qml6-module-org-kde-desktop`

As a diagnostic fallback, Bermuda can be started using Qt's built-in Fusion style:

```bash
QT_QUICK_CONTROLS_STYLE=Fusion \
cargo run --release -p bermuda-qt
```

If Bermuda works with Fusion but not with the normal style, please mention that when reporting the problem.

### Wayland graphics problem

Bermuda is developed on a modern Wayland desktop, but if a tester encounters an apparent platform or rendering problem, comparing with XWayland can help identify it:

```bash
QT_QPA_PLATFORM=xcb \
cargo run --release -p bermuda-qt
```

This is a diagnostic test, not the preferred normal way to run Bermuda.

## Reporting a problem

When reporting a build or start-up problem, please include the **complete terminal output** from:

```bash
cargo run --release -p bermuda-qt
```

Also include the output of:

```bash
rustc --version
cargo --version
c++ --version | head -1
cmake --version | head -1
qmake6 -query QT_VERSION 2>/dev/null || qmake -query QT_VERSION
```

It is also useful to say which Linux distribution and desktop environment you are using.

## For developers

The normal tester workflow above deliberately avoids Bermuda's command-line database tools. Developers can additionally check the whole workspace with:

```bash
cargo fmt --all --check
cargo test
cargo check -p bermuda-qt
```

The `bermuda` command-line program remains available for database development, importing, inspection and search-engine work:

```bash
cargo run -- --help
```

## Package-information note

The package names in this guide were checked against current KDE, Fedora, Debian and Ubuntu package information on 26 August 2026. Distribution package names can change, so this guide should be reviewed periodically while Bermuda remains a source-built development application.
