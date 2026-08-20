# Changelog

All notable changes to Bermuda will be documented in this file.

The project follows Semantic Versioning (MAJOR.MINOR.PATCH).

---

## [0.3.0] - 2026-07-23

### Added

- Whole-database pattern search.
- Pattern extraction from arbitrary board positions.
- Brute-force pattern matching engine.
- `PatternSearcher` API for searching individual games and complete databases.
- `PositionIndexer::game_ids()` for enumerating all games in a project.
- New `pattern` and `pattern_search` modules.
- Expanded developer documentation, including:
  - Developer Handbook
  - Algorithms
  - Architecture
  - Roadmap
  - Position Explorer
  - Position Index

### Improved

- Refactored pattern searching into reusable components.
- Simplified search architecture by separating board, game, and database searching.
- Continued improvements to command-line tooling and project structure.

---

## [0.2.0] - 2026-07-16

### Added

- SQLite-backed project database.
- Canonical game hashing.
- Exact position indexing.
- Exact position search.
- Game replay engine.
- SGF import improvements.
- Support for setup stones and captures.
- Move replay by move number and move range.

### Improved

- Replay performance.
- Metadata handling.
- Internal indexing architecture.

---

## [0.1.0]

Initial public development release.

### Added

- SGF parser.
- Main variation extraction.
- Compact move file format.
- Board representation.
- Move legality checking.
- Capture handling.
- Ko detection.
- Pass handling.
- Command-line import, inspect and replay tools.

---

## Future

Planned for future releases:

- Pattern wildcards.
- Pattern symmetry handling.
- Indexed pattern search.
- Joseki and fuseki exploration tools.
- Performance optimisation.
- Native graphical interface.
