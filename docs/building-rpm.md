# Building the first Tumbleweed RPM

The 0.8.0 RPM was built and installed successfully on the development
Tumbleweed machine on 25 September 2026. Both binary and source RPM signatures
were verified successfully with RPM 4.20.1. Clean-machine installation testing
and the remaining release checks are still required before publication.

## Build

Use the source-build prerequisites in [Building on Linux](building-on-linux.md),
then install the RPM tooling and runtime modules:

```bash
sudo zypper install rpm-build cargo rust python3 desktop-file-utils \
    qt6-declarative-imports kf6-kirigami-imports kf6-qqc2-desktop-style
```

Apply the bundled-interface change before packaging. The build script checks
for it. Run from the repository as your normal user:

```bash
mkdir -p ~/bermuda-profiles
set -o pipefail
bash packaging/opensuse/build-rpm.sh 2>&1 | tee ~/bermuda-profiles/rpm-build-0.8.0.txt
```

The script prints its temporary build directory immediately and the RPM paths
on success. It preserves the directory after failure for diagnosis. Allow
several gigabytes for dependencies, compiled files and source archives.

The script copies tracked source files with their current edits, plus the new
packaging files. Commit or `git add` newly created application source files
before building so they are included. It does not archive arbitrary untracked
files or your user profile. `cargo vendor --locked` downloads dependencies;
`rpmbuild` then compiles with `--frozen` (offline, unchanged lockfile).

The RPM includes the GUI, command-line tool, desktop launcher and documentation.
Qt/KDE runtime packages are dependencies. It includes no game collection or
KataGo engine. Its generic games icon can be replaced later.

## Inspect and install

Use the actual RPM path printed by the script:

```bash
rpm -qpl /tmp/bermuda-rpm.XXXXXX/RPMS/x86_64/bermuda-0.8.0-1.x86_64.rpm
rpm -qpR /tmp/bermuda-rpm.XXXXXX/RPMS/x86_64/bermuda-0.8.0-1.x86_64.rpm
sudo zypper install /tmp/bermuda-rpm.XXXXXX/RPMS/x86_64/bermuda-0.8.0-1.x86_64.rpm
```

The build produces unsigned packages. For release copies, follow the signing
steps below and install the signed copy. A local test of your own unsigned
build may show a signature warning; do not disable signature checks globally.

A user-created `~/.local/share/applications/org.bermuda.app.desktop` overrides
the installed launcher. If yours still points at `target/release/bermuda-qt`,
back it up outside the applications directory before testing the packaged menu
entry. Preserve any icon customisation you want to reuse. Run `kbuildsycoca6`
after changing the launcher, or sign out and back in.

Close the development copy before opening the installed one. First confirm
that `/usr/bin/bermuda-qt` starts, then launch Bermuda from KDE and check that
About reports 0.8.0. Briefly check database access and the KataGo setup dialog.
The installed program must work without the source checkout. An existing
profile is reused; a fresh profile should still offer the normal first-run flow.

For release validation also test on a clean Tumbleweed installation: the
development machine may have runtime packages which conceal missing dependencies.

## Remove or update

```bash
sudo zypper remove bermuda
```

Only package-owned system files are removed. Per-user databases, settings and
managed KataGo installations are not RPM-owned. Installing an updated RPM uses
the same paths. Keep source RPMs alongside binary releases so corresponding
source and vendored dependencies are available.

Once the build has been inspected and the RPMs copied somewhere permanent,
the exact temporary directory printed by the script can be removed. Do not
use a broad wildcard to delete temporary profiles or build directories.

## Sign release packages (maintainer)

Signing happens after building. It changes the RPM files, not an already
installed application. Repeat it for every new build, including rebuilds of
the same version. These commands were tested with RPM 4.20.1 and GnuPG.

The Bermuda release-signing key already exists. Do not create another key
for each release. Its public fingerprint is:

```text
671AE6477ACE75C95BC695A5EC2793D4263A81A4
```

The current key expires on 24 September 2028. Arrange renewal before expiry.
The private key and its passphrase remain with the maintainer; neither belongs
in Git, a release download directory, or a support log. Keep a secure offline
backup of the private key and revocation certificate before publication.

First copy the binary and source RPMs out of the temporary build directory
into `~/bermuda-packages/0.8.0/RPMS/` and `~/bermuda-packages/0.8.0/SRPMS/`,
preserving their subdirectories. Sign those permanent copies as your normal
user, not with sudo:

```bash
export GPG_TTY=$(tty)
rpmsign --define "_gpg_name 671AE6477ACE75C95BC695A5EC2793D4263A81A4" \
  --addsign \
  ~/bermuda-packages/0.8.0/RPMS/x86_64/bermuda-0.8.0-1.x86_64.rpm \
  ~/bermuda-packages/0.8.0/SRPMS/bermuda-0.8.0-1.src.rpm
```

Enter the signing passphrase when prompted. Export the public key to accompany
the releases (GnuPG may ask before overwriting an existing export):

```bash
gpg --armor \
  --output ~/bermuda-packages/bermuda-signing-key.asc \
  --export 671AE6477ACE75C95BC695A5EC2793D4263A81A4
```

Import that public key into the local RPM trust database, then verify both
packages:

```bash
sudo rpm --import ~/bermuda-packages/bermuda-signing-key.asc
rpm -Kv \
  ~/bermuda-packages/0.8.0/RPMS/x86_64/bermuda-0.8.0-1.x86_64.rpm \
  ~/bermuda-packages/0.8.0/SRPMS/bermuda-0.8.0-1.src.rpm
```

Both should report the RSA/SHA256 signature with key ID `263a81a4` as `OK`,
as well as successful digest checks. Publish these signed copies and their
corresponding source RPM. The original copies under `/tmp` remain unsigned.
Update the version and release paths in these examples for subsequent builds.

## Install a signed release (user)

Download the public key and RPM from the project's published release location.
Inspect the key before importing it:

```bash
gpg --show-keys --with-fingerprint ./bermuda-signing-key.asc
```

Compare the full fingerprint with the project's independently obtained release
information. Once satisfied that this is the Bermuda key:

```bash
sudo rpm --import ./bermuda-signing-key.asc
rpm -Kv ./bermuda-0.8.0-1.x86_64.rpm
sudo zypper install ./bermuda-0.8.0-1.x86_64.rpm
```

Importing the public key is normally needed only once per signing key. A
signature establishes package origin and integrity; clean-machine testing
still establishes whether the application and its dependencies work.

Reference: [RPM 4.20 signing manual](https://rpm.org/docs/4.20.x/man/rpmsign.8).
