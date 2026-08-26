# Bermuda

Bermuda is an open-source desktop application for studying Go games.

It combines an SGF game database with an interactive Go board and pattern
search. Positions can be selected directly on the board and searched across
the database, with results showing matching games and continuation
statistics.

Bermuda is intended for serious game study while keeping the normal workflow
simple: import a collection of SGF games, browse the catalogue, open a game,
select a pattern, and search.

> Bermuda is under active development. Linux is currently the tested and
> supported platform.

## Features

- Browse large collections of SGF games.
- Replay games on an interactive 19×19 Go board.
- Open individual SGF files independently of the database.
- Select rectangular patterns directly on the board.
- Search for matching positions across the game database.
- Match patterns under board rotations and reflections.
- Examine the moves played after matching positions.
- View continuation statistics on the board.
- Filter search results by game metadata.
- Filter results by particular continuations.
- Import additional SGF collections into an existing database.
- Detect duplicate games when collections overlap.
- Retain source information for imported game collections.

## Using Bermuda

Bermuda is intended to be installed and used as a normal desktop application,
launched from an application menu or icon.

Packaged releases are not available yet, so the current development version
must still be built from source. The `cargo` commands later in this README
are therefore development/build instructions, not the intended long-term
way of launching Bermuda.

### First launch

On its first normal launch, Bermuda offers to create a managed
**Games Database**.

Choose a folder containing SGF files, enter a name and version for the source,
and select **Create**. Bermuda imports the games and prepares the database for
searching.

On subsequent launches, the Games Database is opened automatically.

To add another collection or a later release of an existing collection, use:

**Database → Add Games…**

Bermuda automatically updates the search data needed after the import.

## Pattern search

Open a game from the catalogue, or open an external SGF file with
**File → Open SGF…**.

Move to the position you want to investigate and choose **Select Pattern**.
Drag on the board to select the rectangular area to search for, then start
the search.

Bermuda searches for equivalent positions across the database, including
rotations and reflections where appropriate.

The results can then be explored in several ways:

- open matching games at the matching position;
- examine the most common continuation moves;
- restrict the results by player, date and other game metadata;
- restrict the results to games containing a selected continuation;
- compare alternative continuations on the board.

Starting a new search restores the original source game and board orientation.

## Demonstration

A demonstration of the graphical application is available here:

[Watch the Bermuda demonstration](docs/bermuda-demo.webm)

## Building from source

Packaged releases are not available yet, so the current development version must still be built from source.

Bermuda is written in Rust and uses Qt 6, QML, CXX-Qt and KDE Kirigami.

For step-by-step instructions for openSUSE Tumbleweed, Fedora, Debian and Ubuntu, see:

**[Building and Running Bermuda on Linux](docs/building-on-linux.md)**

The same Bermuda build runs under KDE Plasma, GNOME, Cinnamon, XFCE and other Linux desktop environments. KDE Plasma itself is not required.

Once the dependencies are installed, Bermuda can be built and run from the repository root with:

```bash
cargo run --release -p bermuda-qt
```

These are development/build instructions. Once packaged releases are available, ordinary users will install Bermuda and launch it normally from their desktop environment.

## Game collections

Bermuda works with SGF collections supplied by the user.

Each import records a source name and source version. This makes it possible
to combine collections and later releases while retaining their provenance.
Games present in more than one imported source are detected as duplicates
rather than being stored as independent copies.

## Command-line and developer tools

The repository also contains the `bermuda` command-line application used for
database development, importing, inspection and search-engine work.

Show the currently available commands with:

```bash
cargo run -- --help
```

The command-line interface is primarily a development and advanced-user
interface. Normal use of Bermuda is through the graphical application.

## Development status

Bermuda is under active development.

Linux is the currently tested and supported target. The Rust, Qt and CXX-Qt
technology used by the project makes other desktop platforms possible, but
Windows and macOS should not yet be considered supported Bermuda platforms.

The database and search implementation continue to evolve, and file formats
and developer interfaces may change while the project is in development.

## License

Bermuda is free software licensed under the GNU General Public License,
version 3 or later.

See `LICENSE` for details.
