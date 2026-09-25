#!/usr/bin/env bash
# Prepare a local test bundle. No app installation or publication is performed.
set -euo pipefail
repo_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
cd "$repo_dir"
for tool in python3 cargo git flatpak flatpak-builder; do
    command -v "$tool" >/dev/null || { printf 'Missing build tool: %s\n' "$tool" >&2; exit 1; }
done
if [[ $(flatpak --default-arch) != x86_64 ]]; then
    printf 'This first Flatpak candidate targets x86_64.\n' >&2
    exit 1
fi
flatpak_dir=$(mktemp -d "${TMPDIR:-/tmp}/bermuda-flatpak.XXXXXX")
printf 'Build directory: %s\n' "$flatpak_dir"
python3 - "$repo_dir" "$flatpak_dir" <<'PY'
from pathlib import Path
import shutil, subprocess, sys, tomllib, xml.etree.ElementTree as ET
repo, out = map(Path, sys.argv[1:])
version = tomllib.loads((repo / 'Cargo.toml').read_text())['package']['version']
assert version == tomllib.loads((repo / 'bermuda-qt/Cargo.toml').read_text())['package']['version']
meta = ET.parse(repo / 'packaging/flatpak/org.bermuda.app.metainfo.xml')
assert meta.find('./releases/release').get('version') == version, 'Update AppStream release version'
paths = subprocess.check_output(['git', 'ls-files', '-z'], cwd=repo).decode().split('\0')
roots = {'src', 'tests', 'data', 'bermuda-qt'}
files = {'Cargo.toml', 'Cargo.lock', 'LICENCE', 'README.md'}
paths = [p for p in paths if p and (p in files or Path(p).parts[0] in roots)]
paths += ['packaging/org.bermuda.app.desktop', 'docs/katago-setup.md',
          'packaging/flatpak/org.bermuda.app.metainfo.xml',
          'packaging/flatpak/org.bermuda.app.svg']
for name in paths:
    source = repo / name
    if not source.is_file():
        raise SystemExit(f'Missing source file: {source}')
    dest = out / 'source' / name
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, dest)
assert 'qrc:/qt/qml/org/bermuda/app/src/qml/Main.qml' in (out / 'source/bermuda-qt/src/main.rs').read_text(), 'Apply the bundled-interface patch first'
shutil.copy2(repo / 'packaging/flatpak/org.bermuda.app.json', out / 'org.bermuda.app.json')
(out / 'version').write_text(version)
PY
version=$(cat "$flatpak_dir/version")
cd "$flatpak_dir/source"
mkdir -p .cargo
# Download on the host; the SDK compilation runs without network access.
cargo vendor --locked --versioned-dirs vendor > .cargo/config.toml
python3 - <<'PY'
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
PY
cd "$flatpak_dir"
flatpak-builder --user --install-deps-from=flathub \
    --repo="$flatpak_dir/repo" "$flatpak_dir/build" "$flatpak_dir/org.bermuda.app.json"
flatpak build-bundle "$flatpak_dir/repo" \
    "$flatpak_dir/bermuda-$version-x86_64.flatpak" org.bermuda.app test \
    --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo
printf '\nTest bundle: %s/bermuda-%s-x86_64.flatpak\n' "$flatpak_dir" "$version"
printf 'Bermuda has not been installed or published. Build runtimes may have been installed. Keep this directory until testing is complete.\n'
