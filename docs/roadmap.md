# MoyoDB Roadmap

## Vision

MoyoDB is a modern open-source database and research environment for professional Go games.

The project aims to provide a reliable, maintainable replacement for MoyoGo Studio with a modern architecture, fast searching, and a Qt-based graphical interface.

The database is built around canonical game identities rather than SGF files, allowing the same game to be imported from multiple sources while storing only one canonical game record.

---

# Version 0.2 — Foundation ✅

Completed

* SGF parser
* Main variation extraction
* Setup stone support
* Captures
* Passes
* Simple ko
* Compact binary move format
* Board replay engine
* Canonical game identity (SHA-256)
* SQLite metadata database
* Single-game importer
* Recursive directory importer
* Duplicate detection
* Source-aware metadata

---

# Version 0.3 — Production Importer

Goal: reliably build large databases from professional SGF collections.

Planned work

* Hash-based storage directories for move files
* Batch SQLite transactions
* Import progress reporting
* Import statistics
* Improved error logging
* Performance tuning
* Full import of GoGoD
* Full import of go4go

Deliverable

A complete professional game database built from existing SGF collections.

---

# Version 0.4 — Position Index

Goal: convert game records into a searchable position database.

Planned work

* Replay every imported game
* Store every board position
* Canonical board representation
* Position indexing
* Position statistics
* Fast position lookup
* Incremental position indexing
* Position fingerprints
* Index rebuild command

Deliverable

A complete position database suitable for large-scale searching.

---

# Version 0.5 — Pattern Search

Goal: provide fast pattern searching across the complete database.

Planned work

* Whole-board exact position search
* Corner patterns
* Side patterns
* Centre patterns
* Colour-independent matching
* Rotation and reflection support
* Candidate filtering
* Search statistics

Deliverable

Professional-strength pattern searching.

---

# Version 0.6 — Qt Desktop Application

Goal: modern graphical user interface.

Planned work

* Database browser
* Game viewer
* Board display
* Pattern search interface
* Game information panels
* Import management
* Preferences

Deliverable

Daily-use desktop application.

---

# Version 0.7 — Fuseki Explorer

Goal: explore professional opening play interactively.

Planned work

* Opening tree built from indexed positions
* Move frequency statistics
* Win/loss percentages
* Symmetry-aware opening merging
* Opening filters

  * Player
  * Date range
  * Event
  * Source database
* Jump directly from opening positions to matching games
* Export opening sequences as SGF

Deliverable

A professional opening explorer capable of analysing the first 30–50 moves of hundreds of thousands of games.

---

# Version 0.8 — Analysis

Goal: support advanced Go research.

Planned work

* Joseki explorer
* Corner statistics
* Player reports
* Tournament reports
* Date-range filtering
* Opening popularity graphs
* Position frequency analysis
* Pattern statistics
* AI integration hooks

Deliverable

A comprehensive professional Go analysis environment.

---

# Version 1.0 — Public Release

Goals

* Stable file format
* Stable database schema
* Complete documentation
* Linux packages
* Windows packages
* Open-source release
* GPL licensing
* User manual

Deliverable

A complete professional Go database system suitable for long-term maintenance.

---

# Design Principles

* Correctness before optimisation.
* Small, testable components.
* Canonical game identity independent of SGF formatting.
* Original implementations based on published specifications.
* Clear separation between core library, importer, search engine, and GUI.
* Open architecture suitable for long-term maintenance.

## Project Milestones

### Completed

* Public Git repository established on GitHub
* GPL v3 licensed
* Source code under version control
* Documentation published
* Development roadmap published

Future project management improvements

* GitHub Issues
* GitHub Releases
* Continuous Integration (GitHub Actions)
* Contributor guidelines
* Code of Conduct
