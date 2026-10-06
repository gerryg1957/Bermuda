Bermuda for Windows 11 (64-bit Intel/AMD) - first Windows test package

Extract the ENTIRE ZIP into a folder you can write to, then open Bermuda.exe.
Keep the qml, platforms and other folders alongside the executable.
Do not run from inside the ZIP. No Qt, Rust or administrator installation is required.
To add a desktop shortcut, right-click Bermuda.exe and use Windows' shortcut options.
The shortcut uses the same Go-board icon as the Linux packages.

Bermuda stores your games and settings separately in your Windows user profile.
Removing the extracted application folder does not remove your game collections.
No professional game database is included. Use Bermuda's database setup guidance.

KataGo is optional. Use the guided KataGo setup inside Bermuda to download a CPU
engine and network. An internet connection is required for setup. The installer
checks download hashes and selects an AVX2 engine only when the CPU supports it.
Existing engine and custom network paths can also be selected through setup.
Automatic GPU-specific engine installation is not included in this release.

This first Windows package is not Authenticode-signed. Windows may display an
unknown-publisher warning. Obtain packages only from the project's GitHub release.
The .sha256 file checks download integrity; it is not a publisher signature.

Build and automated check details: BUILD-INFO.txt and the GitHub Actions run.
A GitHub Windows Server build check is not a substitute for Windows 11 user testing.
Please report startup, import, study, file-dialog and KataGo setup problems at:
https://github.com/gerryg1957/Bermuda/issues

Source, build instructions and licences:
https://github.com/gerryg1957/Bermuda
See LICENCE and licenses/ for Bermuda and bundled components.
Qt 6.9.3 source: https://download.qt.io/archive/qt/6.9/6.9.3/single/
Kirigami 6.18.0 source: https://invent.kde.org/frameworks/kirigami/-/tree/v6.18.0
The libraries are dynamically linked; replacements must be binary-compatible.
