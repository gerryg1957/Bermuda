# Run in an x64 MSVC developer environment with Qt 6.9.3 and Rust installed.
# Produces a portable ZIP only after the deployed application passes its checks.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $true
Set-Location (Resolve-Path "$PSScriptRoot/../..")
$repo = (Get-Location).Path
$work = Join-Path $repo 'windows-build'
$logs = Join-Path $work 'logs'
$prefix = Join-Path $work 'kde'
New-Item -ItemType Directory -Force $work, $logs, $prefix | Out-Null
$env:RUSTFLAGS = '' # Generic x64 build: no target-cpu=native or mandatory AVX2.
$env:CARGO_TARGET_DIR = Join-Path $repo 'target'
$qt = (& qmake -query QT_INSTALL_PREFIX).Trim()
$env:QMAKE = Join-Path $qt 'bin/qmake.exe'
$env:CMAKE_PREFIX_PATH = "$prefix;$qt"
$env:PATH = "$prefix/bin;$env:PATH"

# Kirigami needs ECM and Qt, but not a KDE desktop installation.
foreach ($module in @('extra-cmake-modules', 'kirigami')) {
    $source = Join-Path $work $module
    if (-not (Test-Path $source)) {
        git clone --depth 1 --branch v6.18.0 "https://github.com/KDE/$module.git" $source
    }
    $moduleBuild = Join-Path $work "$module-build"
    cmake -S $source -B $moduleBuild -G Ninja `
        '-DCMAKE_BUILD_TYPE=Release' "-DCMAKE_INSTALL_PREFIX=$prefix" `
        "-DCMAKE_PREFIX_PATH=$prefix;$qt" `
        '-DBUILD_TESTING=OFF' '-DBUILD_DOC=OFF' '-DBUILD_EXAMPLES=OFF' `
        '-DUSE_DBUS=OFF' '-DCMAKE_DISABLE_FIND_PACKAGE_OpenMP=ON' `
        '-DKDE_INSTALL_QMLDIR=qml' '-DKDE_INSTALL_LIBDIR=lib' '-DKDE_INSTALL_BINDIR=bin'
    cmake --build $moduleBuild --parallel 2
    cmake --install $moduleBuild
}

$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
$version = ($metadata.packages | Where-Object name -eq 'bermuda-qt').version
$revision = (Get-Content "$PSScriptRoot/package-revision.txt" -Raw).Trim()
if ($revision -notmatch '^[1-9][0-9]*$') { throw 'Invalid Windows package revision' }
$env:BERMUDA_PACKAGE_VERSION = "$version-$revision"
node tests/file-paths.cjs
cargo test --release --locked -p bermuda --lib --target x86_64-pc-windows-msvc
cargo build --release --locked --workspace --target x86_64-pc-windows-msvc
$name = "bermuda-$version-$revision-windows-x64"
$stage = Join-Path $work $name
if (Test-Path $stage) { throw "Package directory already exists: $stage. Use a fresh build directory." }
New-Item -ItemType Directory $stage | Out-Null
$bin = Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc/release'
Copy-Item "$bin/bermuda-qt.exe" "$stage/Bermuda.exe"
Copy-Item "$bin/bermuda.exe" "$stage/bermuda-cli.exe"
Copy-Item "$prefix/bin/*.dll" $stage
Copy-Item "$prefix/qml" "$stage/qml" -Recurse

# Discover Qt imports in both Bermuda and the installed Kirigami modules.
& "$qt/bin/windeployqt.exe" --release --compiler-runtime `
    --qmldir "$repo/bermuda-qt/src/qml" --qmlimport "$prefix/qml" `
    --dir $stage "$stage/Bermuda.exe"
# Walk every KDE DLL as well so indirect Qt DLL dependencies are deployed.
$kdeDlls = Get-ChildItem "$prefix/bin" -Filter '*.dll'
foreach ($dll in $kdeDlls) {
    & "$qt/bin/windeployqt.exe" --release --no-translations --dir $stage (Join-Path $stage $dll.Name)
}
# App-local MSVC runtime means no administrator installation is required.
if (-not $env:VCToolsRedistDir) { throw 'MSVC redistributable directory not available' }
$crt = Get-ChildItem (Join-Path $env:VCToolsRedistDir 'x64') -Directory -Filter 'Microsoft.VC*.CRT' |
    Select-Object -First 1
if (-not $crt) { throw 'MSVC x64 redistributable DLLs not found' }
Copy-Item (Join-Path $crt.FullName '*.dll') $stage
@'
[Paths]
Prefix=.
Plugins=.
QmlImports=qml
Qml2Imports=qml
'@ | Set-Content "$stage/qt.conf" -Encoding utf8
Copy-Item "$PSScriptRoot/bermuda.ico" $stage
Copy-Item "$PSScriptRoot/README.txt" $stage
Copy-Item "$repo/LICENCE" $stage
New-Item -ItemType Directory "$stage/licenses" | Out-Null
Copy-Item "$work/kirigami/LICENSES" "$stage/licenses/Kirigami" -Recurse
# Include Qt's license texts from its source distribution (same version as binaries).
$qtLicenses = Join-Path $work 'qtbase-license-source'
git clone --depth 1 --filter=blob:none --sparse --branch v6.9.3 https://github.com/qt/qtbase.git $qtLicenses
git -C $qtLicenses sparse-checkout set LICENSES
Copy-Item "$qtLicenses/LICENSES" "$stage/licenses/Qt" -Recurse
# Cargo-vendor output is used only to collect dependency notices, never shipped as binaries.
$vendor = Join-Path $work 'vendor'
cargo vendor --locked --versioned-dirs $vendor | Out-Null
Get-ChildItem $vendor -Recurse -File | Where-Object {
    $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT)'
} | ForEach-Object {
    $relative = [IO.Path]::GetRelativePath($vendor, $_.FullName)
    $destination = Join-Path "$stage/licenses/Rust" $relative
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item $_.FullName $destination
}
$commit = (git rev-parse HEAD).Trim()
@("Bermuda $version", "Windows package revision: $revision", "Source: https://github.com/gerryg1957/Bermuda", "Commit: $commit",
  "Rust: $(rustc --version)", "Qt: $(& qmake -query QT_VERSION)", 'KDE Frameworks: 6.18.0',
  "Kirigami commit: $(git -C $work/kirigami rev-parse HEAD)") |
    Set-Content "$stage/BUILD-INFO.txt" -Encoding utf8

# Exercise the shipped files from a path containing spaces, without SDK search paths.
$check = Join-Path $work 'Clean user check'
New-Item -ItemType Directory -Force $check | Out-Null
Copy-Item $stage "$check/Bermuda" -Recurse
$deployed = Join-Path $check 'Bermuda'
$oldPath = $env:PATH
$oldAppData = $env:APPDATA
$oldLocal = $env:LOCALAPPDATA
try {
    $env:PATH = "$deployed;$env:SystemRoot/System32;$env:SystemRoot"
    foreach ($variable in @('QML_IMPORT_PATH', 'QML2_IMPORT_PATH', 'QT_PLUGIN_PATH',
        'QT_QPA_PLATFORM_PLUGIN_PATH', 'QT_QUICK_CONTROLS_STYLE')) {
        Remove-Item "Env:$variable" -ErrorAction SilentlyContinue
    }
    $env:APPDATA = Join-Path $check 'Roaming'
    $env:LOCALAPPDATA = Join-Path $check 'Local'
    New-Item -ItemType Directory -Force $env:APPDATA, $env:LOCALAPPDATA | Out-Null
    $env:QT_FORCE_STDERR_LOGGING = '1'
    $env:QT_QUICK_BACKEND = 'software'
    # Exercise the same URL conversion used by Open SGF, with spaces, Unicode,
    # a literal percent and a hash in the filename (all require URL decoding).
    $sgfDirectory = Join-Path $check 'SGF files'
    New-Item -ItemType Directory -Force $sgfDirectory | Out-Null
    $sgf = Join-Path $sgfDirectory 'étude 100% #1.sgf'
    '(;FF[4]GM[1]SZ[19];B[pd];W[dd])' | Set-Content $sgf -Encoding utf8
    $sgfUrl = ([Uri]::new($sgf)).AbsoluteUri
    $app = Start-Process "$deployed/Bermuda.exe" `
        -ArgumentList "--smoke-test --smoke-sgf-url `"$sgfUrl`"" `
        -WorkingDirectory $deployed -PassThru `
        -RedirectStandardOutput "$logs/startup.out" -RedirectStandardError "$logs/startup.err"
    if (-not $app.WaitForExit(60000)) {
        Stop-Process -Id $app.Id -Force
        Get-Content "$logs/startup.out", "$logs/startup.err" -ErrorAction SilentlyContinue
        throw 'Packaged GUI did not finish its startup check within 60 seconds'
    }
    $app.Refresh()
    $output = (Get-Content "$logs/startup.out", "$logs/startup.err" -Raw) -join "`n"
    Write-Host $output
    if ($app.ExitCode -ne 0 -or $output -notmatch 'BERMUDA_STARTUP_OK' -or
        $output -notmatch 'BERMUDA_SGF_OPEN_OK' -or
        $output -match 'failed to load component|is not installed|Type .* unavailable') {
        throw 'Packaged QML startup check failed'
    }
    $installRoot = Join-Path $check 'KataGo installation'
    $app = Start-Process "$deployed/Bermuda.exe" `
        -ArgumentList "--check-katago-install `"$installRoot`"" `
        -WorkingDirectory $deployed -PassThru `
        -RedirectStandardOutput "$logs/katago.out" -RedirectStandardError "$logs/katago.err"
    if (-not $app.WaitForExit(600000)) {
        & "$env:SystemRoot/System32/taskkill.exe" /PID $app.Id /T /F
        throw 'Guided KataGo installation/analysis exceeded ten minutes'
    }
    $app.Refresh()
    if ($app.ExitCode -ne 0 -or -not (Test-Path "$installRoot/result.json")) {
        Get-Content "$logs/katago.err"
        throw 'Guided KataGo installation/analysis failed'
    }
    Copy-Item "$installRoot/result.json" "$logs/katago-result.json"
    $result = Get-Content "$installRoot/result.json" -Raw | ConvertFrom-Json
    if (-not $result.success) { throw 'KataGo did not report successful analysis' }
} finally {
    $env:PATH = $oldPath
    $env:APPDATA = $oldAppData
    $env:LOCALAPPDATA = $oldLocal
}
New-Item -ItemType Directory -Force "$repo/dist" | Out-Null
$zip = "$repo/dist/$name.zip"
Compress-Archive -Path $stage -DestinationPath $zip
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $name.zip" | Set-Content "$repo/dist/$name.sha256" -Encoding ascii
Write-Host "Checked Windows package: $zip"
