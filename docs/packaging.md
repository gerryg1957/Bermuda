# Bermuda Build and Packaging Strategy

## Status

Design document.

## Goals

Bermuda should be straightforward to:

- build from source;
- install properly on Linux;
- package for major Linux distribution families;
- test on Windows;
- maintain without several contradictory build systems.

The existing Bermuda build instructions should be reused and developed rather than replaced unnecessarily.

## Principle: one build, several packages

Distribution packages should be thin descriptions of how to build and install Bermuda using the project's normal build process.

We should avoid having:

- one unofficial build procedure for developers;
- another embedded in an RPM;
- another embedded in Debian packaging;
- another set of commands in documentation.

The source build is authoritative. Packaging adapts dependency names, installation paths and distribution policy around it.

## Build documentation

The source-build instructions have now been inspected and consolidated as:

    docs/building-on-linux.md

That document covers the common Bermuda build and the distribution-specific prerequisites for openSUSE Tumbleweed, Fedora, Debian and Ubuntu.

The consolidated guide should remain the authoritative source-build documentation unless experience shows that separate distribution documents would genuinely be clearer.

## Packaging order

Recommended order:

1. openSUSE RPM;
2. Fedora RPM;
3. Debian/Ubuntu `.deb`;
4. additional distributions through source-build instructions;
5. Windows deployment/package work in parallel as the Windows port matures.

openSUSE is the natural first RPM target because it is Bermuda's principal development environment.

## Linux installation layout

A packaged Bermuda should follow standard Linux filesystem conventions.

Likely installed components include:

- Bermuda GUI executable;
- any Bermuda CLI executable(s);
- `.desktop` launcher;
- AppStream metadata;
- application icons;
- MIME information if required;
- licences/documentation as required by the distribution.

Exact paths should follow the packaging policy of the target distribution rather than being hard-coded unnecessarily into application logic.

## openSUSE RPM

The first packaging target should be a native openSUSE RPM.

A source package should contain or reference:

- Bermuda source;
- Cargo metadata;
- Qt/KDE build dependencies;
- build commands;
- install commands;
- desktop integration;
- package metadata;
- licence information.

A likely repository structure is:

    packaging/
        opensuse/
            bermuda.spec

Once this works locally, suitability for the Open Build Service can be considered.

The RPM build should exercise the same underlying build procedure documented for source builds.

## Fedora / Red Hat family

Fedora and openSUSE both use RPM, but they should not be assumed to accept an identical spec file unchanged.

Differences may include:

- dependency package names;
- Qt/KDE versions;
- Rust packaging conventions;
- macros;
- filesystem paths/policy;
- AppStream validation requirements.

We should reuse as much as practical while allowing a Fedora-specific spec or conditional sections where that is cleaner.

"Red Hat support" needs to be defined carefully. Current Fedora is likely to have newer Rust/Qt/KDE dependencies than enterprise RHEL releases.

The first Red Hat-family target should therefore probably be Fedora. RHEL-compatible distributions can be evaluated once Bermuda's minimum dependency versions are known.

## Debian and Ubuntu

Debian-family packaging should live in a conventional `debian/` packaging directory or an equivalent packaging source arrangement.

It should produce normal `.deb` packages using the same Bermuda build.

Again, the package should not contain a separately maintained application build recipe unless Debian policy requires a specific adaptation.

Ubuntu compatibility can normally derive from Debian packaging, subject to available dependency versions.

## Other Linux distributions

We do not need native packages for every distribution.

A good `building.md` plus a clear dependency list should allow users of distributions such as Arch, Gentoo or Mageia to build Bermuda themselves or create native community packages.

The project should document:

- minimum Rust version;
- minimum Qt version;
- KDE Framework/Kirigami requirements if adopted;
- SQLite requirements;
- C/C++ build tools required by dependencies;
- CMake or other build-tool requirements;
- optional KataGo integration requirements.

## Cargo and workspace structure

As Bermuda grows, it may become useful to use a Cargo workspace containing reusable components such as:

    crates/
        bermuda-core/
        bermuda-corpus/
        bermuda-katago/

and the GUI application.

This is a possible destination, not an immediate packaging prerequisite.

Code should be split into crates when stable interfaces make the separation useful, not simply to make the directory tree look architectural.

## KataGo packaging

KataGo should be treated as an external program rather than bundled blindly into Bermuda.

Reasons include:

- large neural-network models;
- CPU/GPU variants;
- independent release cadence;
- distribution packaging availability;
- licensing and redistribution considerations;
- users may already have a configured KataGo installation.

A Bermuda package may recommend or optionally depend on a distribution KataGo package where appropriate, but Bermuda should still start and function as a professional database without KataGo.

The application should explain clearly when AI functionality is unavailable because KataGo has not been configured.

## Personal corpus packaging

Personal-corpus support is part of Bermuda and does not require a separate application package merely because its database is separate from the professional corpus.

User databases and configuration must never be installed into system package directories.

Package upgrades must not overwrite personal games or analysis.

## Application data

Distribution packages should install immutable application resources into normal system locations.

Runtime/user data should use appropriate per-user data/configuration/cache locations.

We should explicitly classify data before implementation:

- configuration;
- persistent databases;
- personal corpus;
- KataGo analysis cache/persistent analysis;
- disposable cache;
- logs.

This avoids later migration problems.

## Windows

The first Windows target should be a native Windows 11 x86-64 build.

Initial development should favour building Bermuda natively on a Windows test machine so that genuine portability issues become visible.

Once a native build works, the next objective is a self-contained test distribution that does not require the tester to install Rust, Qt or development tools.

Windows deployment will need to account for:

- Qt runtime libraries/plugins;
- any KDE runtime components used by the GUI;
- Microsoft runtime requirements;
- application resources;
- optional KataGo discovery/configuration.

An installer can come later. A reliable self-contained test directory is a more useful first milestone.

## Continuous builds

Once Linux and Windows builds are repeatable, automated builds should be considered.

Useful automated checks include:

- `cargo test`;
- `cargo clippy`;
- formatting checks;
- Linux GUI build;
- RPM build;
- Debian package build;
- Windows GUI build.

Packaging should not be introduced into continuous integration until the ordinary build is stable enough that packaging failures are meaningful.

## Versioning

Application packages should derive their version from one authoritative Bermuda version.

Database-format and index-format versions remain separate concepts and must not be confused with the application package version.

## Repository layout

A possible eventual layout is:

    bermuda/
        Cargo.toml
        crates/
        bermuda-qt/
        data/
            icons/
            desktop/
            metainfo/
        packaging/
            opensuse/
            fedora/
        debian/
        docs/
            building-on-linux.md
            personal-corpus.md
            katago-integration.md
            application-design.md
            packaging.md

This is illustrative. Existing working project structure should not be reorganised merely to conform to this diagram.

## Immediate next steps

The source-build instructions and principal Rust, Qt, CXX-Qt and Kirigami dependencies have now been inspected and documented.

Before writing the first package:

1. establish the installation/runtime resource requirements;
2. identify the files and Qt/Kirigami resources that must be installed with the application;
3. make any source-tree assumptions compatible with an installed application;
4. write the first openSUSE spec using the normal Bermuda build.

The first packaging milestone remains:

> Build a Bermuda RPM on openSUSE, install it with the package manager, launch Bermuda from the Plasma application menu, and remove it cleanly without touching user data.

That gives us a sound base from which Fedora and Debian-family packaging can follow.
