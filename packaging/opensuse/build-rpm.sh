#!/usr/bin/env bash
# Run with bash; build as your normal user, never as root.
set -euo pipefail
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_dir"
for tool in python3 cargo rpmbuild git; do
    command -v "$tool" >/dev/null || { printf 'Missing build tool: %s\n' "$tool" >&2; exit 1; }
done
rpm_dir=$(mktemp -d "${TMPDIR:-/tmp}/bermuda-rpm.XXXXXX")
printf 'Build files and resulting packages: %s\n' "$rpm_dir"
mkdir -p "$rpm_dir"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS,stage}
# Copy the working source, including uncommitted edits. Do not archive target,
# local profiles, downloaded engines, or arbitrary untracked files.
python3 - "$repo_dir" "$rpm_dir" <<'PY'
import pathlib, shutil, subprocess, sys, tomllib
repo, out = map(pathlib.Path, sys.argv[1:])
version = tomllib.loads((repo / 'Cargo.toml').read_text())['package']['version']
assert version == tomllib.loads((repo / 'bermuda-qt/Cargo.toml').read_text())['package']['version']
spec = (repo / 'packaging/opensuse/bermuda.spec').read_text()
assert f'Version:        {version}\n' in spec, 'Update the spec version to match Cargo.toml'
stage = out / 'stage' / f'bermuda-{version}'
paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=repo).decode().split('\0')
allowed_roots = {'src', 'tests', 'data', 'bermuda-qt'}
allowed_files = {'Cargo.toml', 'Cargo.lock', 'LICENCE', 'README.md'}
paths = [p for p in paths if p and (p in allowed_files or pathlib.Path(p).parts[0] in allowed_roots)]
paths += ['packaging/org.bermuda.app.desktop', 'packaging/opensuse/bermuda.spec',
          'packaging/opensuse/build-rpm.sh', 'docs/katago-setup.md', 'docs/building-on-linux.md']
for name in paths:
    source = repo / name
    if not source.is_file():
        raise SystemExit(f'Missing source file: {source}')
    dest = stage / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, dest)
# Fail early if the previously tested bundled-interface patch is absent.
assert 'qrc:/qt/qml/org/bermuda/app/src/qml/Main.qml' in (stage / 'bermuda-qt/src/main.rs').read_text(), 'Apply the bundled-interface patch first'
assert 'AboutDialog.qml' in (stage / 'bermuda-qt/build.rs').read_text()
(out / 'version').write_text(version)
PY
version=$(cat "$rpm_dir/version")
source_dir="$rpm_dir/stage/bermuda-$version"
# Fetch dependencies now; the RPM build itself is locked and offline.
cd "$source_dir"
mkdir -p .cargo
cargo vendor --locked --versioned-dirs vendor > .cargo/config.toml
python3 - <<'PYLICENSE'
from pathlib import Path
import shutil
root = Path('third-party-licenses')
root.mkdir()
for crate in Path('vendor').iterdir():
    for source in crate.rglob('*'):
        if source.is_file() and source.name.upper().startswith(('LICENSE', 'LICENCE', 'COPYING', 'NOTICE', 'COPYRIGHT')):
            dest = root / source.relative_to('vendor')
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, dest)
PYLICENSE
tar -C "$rpm_dir/stage" -czf "$rpm_dir/SOURCES/bermuda-$version.tar.gz" "bermuda-$version"
cp packaging/opensuse/bermuda.spec "$rpm_dir/SPECS/bermuda.spec"
rpmbuild --define "_topdir $rpm_dir" -ba "$rpm_dir/SPECS/bermuda.spec"
printf '\nPackages are in %s/RPMS and %s/SRPMS\n' "$rpm_dir" "$rpm_dir"
