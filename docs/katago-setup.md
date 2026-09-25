# Set up optional KataGo analysis

Open **Help → Set up KataGo**, or **Set up KataGo** in the Analyse → KataGo panel.
Bermuda's database and study features work without KataGo.

## Guided installation

Choose **Install KataGo for me**, then choose whether to keep the computer
responsive (recommended) or allow analysis to use more CPU. Select **Install and
set up**. No installation directory or file paths are required.

Bermuda detects 64-bit Intel/AMD Linux or Windows and whether AVX2/FMA instructions
are available. It downloads the appropriate official KataGo 1.18.1 CPU package
and the official b10c384h6nbttflrs network published with KataGo 1.17.0. Both are
pinned to their published SHA-256 digests. Download size is about 80 MB on Linux
and 44 MB on Windows. Allow 300 MB of disk space.

Files are kept below Bermuda's per-user data directory, in `katago/managed`.
On Linux this normally means `~/.local/share/bermuda/katago/managed`; an
`XDG_DATA_HOME` override is honoured. Paths on other platforms are selected by
the operating system's per-user application-data convention.

Bermuda extracts the Linux AppImage so FUSE is not required, creates an analysis
configuration, and automatically runs a short test. Only successful analysis
activates and saves the new setup. Choose **Done** when it reports ready.
No administrator privileges, compiler or graphics runtime is needed for this
CPU route. The Linux binary still requires a compatible system; failures are
reported without replacing an existing setup.

Partial downloads are removed after failure or cancellation. Verified complete
installations are retained, including when their analysis test fails, so the test
can be retried while the dialog remains open. Closing the dialog before success
leaves the previously saved setup unchanged. The installer does not remove an
older engine or overwrite an existing custom configuration.

The responsive profile uses one simultaneous position, with at most two search
workers and two Eigen evaluation workers. The faster profile permits up to six
of each, adjusted down for machines reporting fewer processors. These are worker
limits, not a fixed percentage of CPU use. They are conservative starting points,
not a benchmark-derived optimum.

## Use an existing or custom installation

Choose **Use my own installation**. Browse to the engine and a compatible network.
Under **Advanced: analysis configuration**, choose a tuned analysis configuration,
or leave blank for Bermuda's starting configuration. Run **Test setup**, then
**Save**. This route also supports graphics-accelerated engines once their runtime
requirements are installed. Custom networks are not replaced automatically.

Existing `BERMUDA_KATAGO_EXECUTABLE`, `BERMUDA_KATAGO_MODEL` and
`BERMUDA_KATAGO_CONFIG` environment overrides remain authoritative and are shown
read-only. Guided installation is disabled while these overrides are present;
remove them from the launch environment and restart to use managed installation.

Automatic installation currently supports x86-64 Linux and Windows. Other
platforms use the custom-installation route. GPU detection and automatic driver
installation are not part of the guided CPU installer.

## What the test means

The test analyses an empty 19×19 board with Black to play, Japanese rules, komi
6.5 and a 10-visit budget. It reports a candidate, evaluation and elapsed time
including startup. This confirms communication and model compatibility; it is
not a speed benchmark or playing-strength measurement.

## Release validation

The pinned Linux AVX2 package and network have completed a real analysis using
the conservative configuration. The integrated Qt wizard must also be checked
in a fresh profile, including restart and subsequent analysis of a loaded game.
Windows packages and other CPU variants require platform testing before claiming
those release packages are validated.
