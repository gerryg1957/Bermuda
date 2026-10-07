# Windows 11 package

The first Windows package targets Intel/AMD x86-64. Windows on ARM and 32-bit
Windows are not currently build targets. Bermuda itself uses the generic Rust
x86-64 MSVC target rather than the build machine's CPU instructions.

## Build through GitHub

Commit and push the Windows packaging changes. The `Windows package` workflow
runs when its workflow or packaging files change. Later builds can be started
from **Actions > Windows package > Run workflow**, selecting `master`.

The Windows 2022 GitHub runner supplies a Windows Server/Visual Studio environment.
It installs Qt 6.9.3, builds KDE ECM and Kirigami 6.18.0, tests Bermuda's core and
builds the release executables. `windeployqt` bundles the runtime QML modules,
plugins and Qt DLLs. The package includes app-local Microsoft runtime DLLs,
third-party licence texts and the Go-board icon used by the Linux packages.
The Rust compiler version and source commit are recorded in BUILD-INFO.txt.

The workflow then runs the deployed app with SDK search paths removed, in a path
containing spaces and with temporary application-data paths. It requires a QML
startup success marker and opens an SGF through the same URL-to-path converter as
the file dialogs, using spaces, Unicode, percent and hash characters in its path.
It also invokes Bermuda's real guided CPU installer,
verifies its downloads, and requires an actual KataGo analysis result. Downloads
are used only for that check and are not included in the distributed ZIP.

Only successful checks produce the **Bermuda-Windows-x64** artifact. Download
that artifact from the completed workflow run; extract the GitHub artifact wrapper
to obtain the release ZIP and its SHA-256 checksum. Upload those files as release
assets after review. The workflow does not publish a GitHub release or use signing
secrets. Diagnostic logs are available in **Windows-check-logs** on failure.

## Windows 11 testing before declaring support

The automated runner has developer tools installed, even though the smoke check
removes their search paths. It does not prove all clean-machine dependencies or
interactive behavior. Ask a Windows 11 tester without Qt or Visual Studio to:

- Extract the complete ZIP and launch Bermuda.exe, checking version and icon.
- Import a small SGF folder, open a game and perform a pattern search.
- Create a study, branch with either colour, add a comment and reopen it.
- Exercise the native open/save dialogs and a non-ASCII user/folder name.
- Complete guided KataGo setup, obtain analysis, restart and analyse again.

The current automated engine test covers the CPU capability route selected by the
runner. It does not test every supported CPU or a GPU backend. Windows hardware
advice remains basic; automatic GPU selection is not promised.

## Local build for contributors with Windows

Use an x64 MSVC developer shell with Rust, Git, Python, Ninja, CMake and the
Qt 6.9.3 MSVC 2022 x64 kit (including ShaderTools) on PATH. Run:

```powershell
./packaging/windows/build-windows.ps1
```

Use a fresh checkout or remove the previous `windows-build` staging directory
before rerunning locally. Output is `dist/bermuda-VERSION-REVISION-windows-x64.zip`.

The Windows ICO is derived from `packaging/flatpak/org.bermuda.app.svg`.
Regenerate it with `python packaging/windows/make-icon.py` after changing the SVG;
the script requires CairoSVG and Pillow. The committed ICO avoids adding Python
image dependencies to the normal build.

Windows package revisions are recorded in `packaging/windows/package-revision.txt`.
Revision 1 corrects file-dialog paths in the original 0.8.2 Windows preview. The
application's Cargo version stays 0.8.2; the Windows package and About dialog
identify the corrected build as 0.8.2-1. Linux packages retain their own revision
numbering. This does not require rebuilding the existing Linux packages.
