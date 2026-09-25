# Building the first Bermuda Flatpak

This is an x86_64 test candidate, not yet a published Flatpak or Flathub
submission. On 25 September 2026 the maintainer confirmed installation, NHK
SGF import, database persistence, guided KataGo setup and analysis after a
restart on Tumbleweed. The final signed bundle was imported into a separate
OSTree repository and its signature verified successfully. Testing on another
Linux installation is still outstanding.

## Prerequisites on Tumbleweed

Use your normal user account. Install the host tooling:

```bash
sudo zypper install flatpak flatpak-builder
```

Add Flathub as a per-user source for the KDE runtime and build SDK:

```bash
flatpak remote-add --user --if-not-exists flathub \
    https://flathub.org/repo/flathub.flatpakrepo
```

This uses Flathub to obtain dependencies; it does not upload Bermuda to
Flathub. The manifest selects `org.kde.Platform` and `org.kde.Sdk` branch
`6.10`, plus the corresponding `rust-stable` SDK extension. The builder installs
these dependencies when required. Accept its runtime/SDK installation prompts.
Expect substantial downloads on the first build and several gigabytes of
working space. Ordinary users of a finished package need the runtime, not the
SDK, Rust, or Qt development packages.

## Build a test bundle

From your Bermuda source checkout:

```bash
mkdir -p ~/bermuda-profiles
set -o pipefail
bash packaging/flatpak/build-flatpak.sh 2>&1 | \
    tee ~/bermuda-profiles/flatpak-build-0.8.0.txt
```

The script prints a unique `/tmp/bermuda-flatpak.XXXXXX` directory. It stages
tracked application source with current edits, plus the new packaging assets;
add newly created application source files to Git before building. Untracked
profiles, game files and host build output are not copied.

Cargo dependencies are downloaded and vendored on the host from `Cargo.lock`.
Compilation then runs offline inside the KDE SDK, using `cargo --frozen`.
The script creates an OSTree repository and a single-file test bundle:

```text
/tmp/bermuda-flatpak.XXXXXX/bermuda-0.8.0-x86_64.flatpak
```

Keep the build directory for diagnostics. Copy the finished bundle to a
permanent directory before clearing temporary files. Bermuda is not installed
by the build script, though the build runtimes may be installed.

## Install and launch explicitly

Substitute the directory printed by your build:

```bash
flatpak install --user /tmp/bermuda-flatpak.XXXXXX/bermuda-0.8.0-x86_64.flatpak
flatpak run org.bermuda.app//test
```

The build script produces an unsigned bundle for local testing. Follow the
signing procedure below to prepare a release copy. RPM signatures do not sign
a Flatpak. A single-file Flatpak does not include the KDE runtime; the
installer downloads that separately.

The exported menu name has a ` (Flatpak test)` suffix. Its desktop ID is the
same as the RPM's, so desktop menu precedence may hide one of the launchers.
Use `flatpak run org.bermuda.app//test` for an unambiguous Flatpak test, or
`/usr/bin/bermuda-qt` for the RPM. No development/RPM launcher is moved or edited
by this procedure.

## Data and permissions

Flatpak supplies separate per-user settings, cache and data under:

```text
~/.var/app/org.bermuda.app/
```

Bermuda should therefore show its first-run setup. The Flatpak does not
implicitly migrate or open the RPM's database. Use a small SGF collection to
create its test database. Do not explicitly select your working database for
this first check; selecting it would grant access to that database.

The manifest permits graphics, Wayland with X11 fallback, and networking for
OGS and KataGo downloads. It does not grant blanket home-directory access or
permission to launch commands on the host. Files and directories outside the
sandbox are selected through the desktop portal's file/folder chooser.

The host must have a working desktop portal backend (KDE's on Plasma).
Opening an SGF, importing a folder of SGFs, saving an SGF, and reopening the
application must be checked: these verify portal access and persistent paths.

## KataGo inside the sandbox

Try **Set up KataGo → Install KataGo for me** within the Flatpak. The current
installer downloads a CPU engine, verifies it and extracts the AppImage
without mounting it. Its process and managed files remain inside the
Flatpak's environment. No FUSE mount or host-command permission is intended.

This guided CPU route passed the maintainer's Flatpak test, including analysis
after restarting Bermuda. Repeat that focused check when changing the
installer, engine package or Flatpak runtime.

Custom networks and configurations can be chosen through the file dialog.
Custom executables also need to work with the Flatpak runtime. A host path
such as `/usr/bin/katago` refers to the sandbox's `/usr`, and a host binary may
require libraries absent from the runtime. Do not promise arbitrary host
GPU-engine compatibility on this first candidate.

## Focused acceptance check

1. Launch and confirm About reports 0.8.0.
2. Import a small SGF directory into a new managed database; open and search a game.
3. Open and save an SGF through the file chooser.
4. Restart; confirm the database and saved settings are retained.
5. Open the OGS joseki library to check network access.
6. Complete guided KataGo setup and get an analysis result.

After these pass, arrange Flatpak signing, repeat the installation on another
Linux machine, and decide between downloadable bundles and a repository with
updates. Flathub submission also requires its own metadata, identity and
policy review; this recipe is not a claim of Flathub acceptance.

## Sign and verify the bundle

Use the existing Bermuda signing key. Replace the example build directory
with the directory printed by your build. These steps operate on build and
release files, not the installed application.

```bash
bermuda_flatpak_build=/tmp/bermuda-flatpak.8y45RJ
export GPG_TTY=$(tty)
flatpak build-sign \
  --gpg-sign=671AE6477ACE75C95BC695A5EC2793D4263A81A4 \
  "$bermuda_flatpak_build/repo" org.bermuda.app test

gpg --output ~/bermuda-packages/bermuda-signing-key.gpg \
  --export 671AE6477ACE75C95BC695A5EC2793D4263A81A4

flatpak build-bundle \
  --gpg-keys="$HOME/bermuda-packages/bermuda-signing-key.gpg" \
  --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
  "$bermuda_flatpak_build/repo" \
  "$HOME/bermuda-packages/0.8.0/bermuda-0.8.0-x86_64-signed.flatpak" \
  org.bermuda.app test
```

On Tumbleweed the `ostree` command is supplied by **libostree**:

```bash
sudo zypper install libostree
bermuda_verify_dir=$(mktemp -d /tmp/bermuda-verify.XXXXXX)
ostree --repo="$bermuda_verify_dir/repo" init --mode=archive-z2
flatpak build-import-bundle "$bermuda_verify_dir/repo" \
  "$HOME/bermuda-packages/0.8.0/bermuda-0.8.0-x86_64-signed.flatpak"
mkdir -m 700 "$bermuda_verify_dir/keyring"
cp ~/bermuda-packages/bermuda-signing-key.gpg \
  "$bermuda_verify_dir/keyring/pubring.gpg"
ostree --repo="$bermuda_verify_dir/repo" show \
  --gpg-homedir="$bermuda_verify_dir/keyring" \
  app/org.bermuda.app/x86_64/test
```

Require `Good signature` from `Bermuda Release Signing` and the expected key
ID `EC2793D4263A81A4`. The verified 0.8.0 test build had OSTree commit:

```text
28b05883a7b75cb95ae26d9203b87a647fc6ba94f0128eb18c1ff0961fa299c6
```

That is the Flatpak commit, not a Git source commit. A later build will have
its own identity. Preserve the build directory until release preparation is
finished. The verification directory can be removed separately afterwards.

## Remove the test application

```bash
flatpak uninstall --user org.bermuda.app//test
```

Do not add `--delete-data` if you want to retain the Flatpak profile. Removing
the Flatpak does not uninstall the RPM. Keep the test profile until any
problems are resolved; it can be removed separately once no longer needed.

References:

- [KDE Rust Flatpak guide](https://develop.kde.org/docs/getting-started/rust/rust-flatpak/)
- [Flatpak sandbox permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)
- [Flatpak single-file bundles](https://docs.flatpak.org/en/latest/single-file-bundles.html)
