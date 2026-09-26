#!/usr/bin/env bash
# Stage already-built, signed candidates for the current Cargo version. No publication or Git mutation.
set -euo pipefail
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_dir"
for tool in git python3 gpg sha256sum; do
    command -v "$tool" >/dev/null || { printf 'Missing tool: %s\n' "$tool" >&2; exit 1; }
done
if ! git diff --quiet || ! git diff --cached --quiet; then
    printf 'Commit the source and packaging changes before staging the release.\n' >&2
    exit 1
fi
version=$(python3 -c 'import tomllib; print(tomllib.load(open("Cargo.toml", "rb"))["package"]["version"])')
rpm_release=$(python3 -c 'import re; from pathlib import Path; print(re.search(r"^Release:\s+(\d+)\s*$", Path("packaging/opensuse/bermuda.spec").read_text(), re.M)[1])')
release_dir="$HOME/bermuda-packages/$version"
if [[ ! -d "$release_dir" ]]; then
    printf 'Build, sign and save the packages in %s first.\n' "$release_dir" >&2
    exit 1
fi
destination="$release_dir/github-pre-release"
if [[ -e "$destination" ]]; then
    printf 'Already exists: %s. Preserve or move it before staging again.\n' "$destination" >&2
    exit 1
fi
stage=$(mktemp -d "$release_dir/.github-pre-release.XXXXXX")
# Keep partial output on failure for inspection; never overwrite old release assets.
printf 'Staging directory: %s\n' "$stage"
python3 - "$repo_dir" "$release_dir" "$stage" "$version" "$rpm_release" <<'PY'
from pathlib import Path
import shutil, subprocess, sys, tomllib
repo, release, stage = map(Path, sys.argv[1:4])
version, rpm_release = sys.argv[4:6]
assert tomllib.loads((repo/'Cargo.toml').read_text())['package']['version'] == version
assert tomllib.loads((repo/'bermuda-qt/Cargo.toml').read_text())['package']['version'] == version
key = release.parent/'bermuda-signing-key.asc'
listing = subprocess.check_output(['gpg','--show-keys','--with-colons',str(key)],text=True)
fingerprint = next(line.split(':')[9] for line in listing.splitlines() if line.startswith('fpr:'))
assert fingerprint == '671AE6477ACE75C95BC695A5EC2793D4263A81A4', 'Unexpected signing public key'
for source in [release/f'RPMS/x86_64/bermuda-{version}-{rpm_release}.x86_64.rpm',
               release/f'SRPMS/bermuda-{version}-{rpm_release}.src.rpm',
               release/f'bermuda-{version}-x86_64-signed.flatpak', key]:
    if not source.is_file() or source.stat().st_size == 0:
        raise SystemExit(f'Missing or empty release input: {source}')
    shutil.copy2(source,stage/source.name)
shutil.copy2(repo/f'docs/releases/{version}.md',stage/'RELEASE-NOTES.md')
commit = subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip()
(stage/'SOURCE-COMMIT.txt').write_text(
    f'Release preparation Git commit: {commit}\n'
    'This records the preparation checkout, not independently verified binary provenance.\n'
    'Confirm that its application source matches the builds before tagging.\n')
PY
cd "$stage"
sha256sum "bermuda-$version-$rpm_release.x86_64.rpm" "bermuda-$version-$rpm_release.src.rpm" \
    "bermuda-$version-x86_64-signed.flatpak" bermuda-signing-key.asc \
    RELEASE-NOTES.md SOURCE-COMMIT.txt > SHA256SUMS
if [[ -t 0 ]]; then export GPG_TTY=$(tty); fi
gpg --local-user 671AE6477ACE75C95BC695A5EC2793D4263A81A4 \
    --armor --detach-sign --output SHA256SUMS.asc SHA256SUMS
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum -c SHA256SUMS
mv -T "$stage" "$destination"
printf '\nRelease files ready for review: %s\nNothing has been published.\n' "$destination"
