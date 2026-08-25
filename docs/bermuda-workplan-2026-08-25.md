# Bermuda Workplan — 25 August 2026

## Current position

Bermuda now has a coherent principal workflow for professional-game study:

- browse large SGF game collections;
- select and replay games;
- open individual SGF files independently of the database;
- select a position or rectangular pattern directly on the board;
- search freely across board locations;
- match rotations and reflections where appropriate;
- distinguish matching games from multiple occurrences;
- examine professional continuations;
- filter and sort the catalogue and Search Results;
- filter by particular continuations;
- exclude handicap games from pattern searches by default while retaining them
  in the database;
- import additional SGF collections while detecting duplicate games and
  preserving their source information.

Pattern search uses the conservative geometric **Bermuda** heuristic for long,
shallow edge-oriented selections. It remains deliberately geometric: it is not
a model of Go strategy or move quality.

The heuristic should be treated as good enough unless ordinary Go study
produces a concrete counterexample. Do not tune its thresholds merely to make
result counts resemble another program.

## Player names and supplied catalogue

Player-name handling has moved beyond the original manually curated identity
system.

Bermuda now has a versioned supplied player catalogue with these principles:

- imported PB/PW text remains untouched;
- supplied identities use stable Bermuda catalogue keys rather than names as
  identifiers;
- supplied preferred names and alternative spellings are stored separately
  from local user corrections;
- local exact source-specific knowledge overrides supplied catalogue data;
- local exact global aliases override supplied catalogue data;
- ambiguous local aliases remain unresolved rather than being guessed;
- supplied catalogue resolution is exact and conservative;
- local corrections survive supplied catalogue updates;
- explicitly suppressed source-name interpretations are not silently restored
  by catalogue reconciliation;
- catalogue updates can retarget catalogue-derived links without disturbing
  genuinely local assignments.

The initial distributed catalogue is based on the **u-go.net Go Player List**,
maintained by Ulrich Görtz. Bermuda adopted the 10 August 2026 snapshot, made
available under CC0 1.0.

The initial catalogue is intentionally small rather than speculative. It
currently establishes the machinery and a curated first set of players; it is
not intended to be the final extent of Bermuda's player-name knowledge.

Player-name search is identity-aware. Searching a supplied alternative spelling
finds the same games as searching the player's preferred name.

### Player Names dialogue

The former **Player Identities** interface has been redesigned as **Player
Names**.

Its user-facing model is now:

- **Source names** are names as they appear in imported game collections that
  Bermuda has not grouped with a player;
- **Players with grouped names** are Bermuda's known player records;
- selecting a source name is inspection only and does not change the database;
- the dialogue explains what the selected source name means and that it may
  simply be left alone;
- a source name can be linked explicitly to an existing player;
- a new player can be created when appropriate;
- selecting a player shows the names Bermuda knows for that player;
- supplied catalogue names are read-only;
- locally added links remain explicitly removable;
- preferred display names can be changed;
- removing a player or a local link remains an explicit, confirmed action.

The dialogue should now be subjected to another independent usability test.
Do not coach the tester through it. Observe whether the current wording and
workflow are self-evident.

## Application presentation

A **Help** menu now exists with an initial **About Bermuda** dialogue.

About Bermuda currently provides:

- the GUI application version directly from the GUI crate version;
- a concise description of Bermuda;
- GNU General Public License version 3-or-later information;
- acknowledgement of the u-go.net Go Player List and Ulrich Görtz;
- the adopted player-list snapshot date and CC0 1.0 status;
- links to the Bermuda repository and u-go.net player list.

The About dialogue should remain concise. Additional technical diagnostics
belong in database information or support facilities rather than turning About
into a dump of implementation details.

## Immediate work

### 1. Use Bermuda for real Go study

Ordinary use is now an important part of development.

Use Bermuda for genuine professional-game research rather than constructing
synthetic examples unless a problem needs isolating.

In particular:

- exercise small tactical patterns as well as larger opening selections;
- record cases where Bermuda hides a useful professional precedent;
- record geometrically valid matches that are genuinely misleading;
- notice friction in pattern selection, Search Results, continuation
  exploration, replay and catalogue browsing;
- use player-name searches under alternative spellings;
- notice cases where a source player name should clearly have been grouped but
  was not;
- prefer concrete examples from actual Go study over speculative refinement.

Do not continue tuning the Bermuda geometric heuristic without such evidence.

### 2. Independent usability testing

Repeat independent usability testing of the application, particularly the new
**Player Names** dialogue.

Do not explain the intended workflow in advance.

Observe:

- what the tester believes the two player-name lists mean;
- whether selecting a source name has an obvious consequence;
- whether it is clear that most source names require no action;
- whether linking to an existing player is discoverable;
- whether creating a new player is understood as the exceptional alternative;
- whether the known-names display explains why several spellings belong
  together;
- terminology or controls that still cause hesitation.

Respond to observed problems rather than redesigning speculatively.

### 3. Expand the supplied player catalogue

Develop the initial supplied catalogue into useful practical coverage while
keeping provenance and correctness more important than raw size.

The principal distributed source is the redistributable u-go.net player list.

For catalogue expansion:

- retain stable Bermuda-owned catalogue keys;
- preserve source provenance;
- keep the catalogue-generation pipeline deterministic;
- increment the catalogue data version whenever its substance changes;
- use corpus comparisons as evidence and validation, not as an automatic source
  of redistributable mappings;
- do not invent equivalences;
- do not merge names merely because they look similar;
- examine external conflicts explicitly;
- retain local user corrections as higher-priority knowledge.

Where useful, develop tooling that helps identify promising catalogue additions,
but do not turn evidence scores into automatic identity assertions.

### 4. First-release application completeness

Review Bermuda as a standalone desktop application rather than only as a
working Go-search engine.

#### Help

About Bermuda now exists.

The next useful Help work is a concise **Bermuda Help / User Guide** covering
the normal workflow:

1. create or open a Games Database;
2. browse and replay games;
3. select a pattern;
4. search the database;
5. interpret Search Results and continuations;
6. understand the optional Player Names facility.

Help should explain the user's task, not the database implementation.

Keyboard-shortcut documentation can wait until there are enough useful
shortcuts to justify it.

#### First-run and empty state

Review what a new user sees when no database is available.

The application should explain useful next actions rather than merely appear
empty. Creating or opening a database and adding games should be discoverable
without prior knowledge of Bermuda.

#### Database information

Add an **About this database**, **Database Information** or **Properties**
facility if practical.

Useful information includes:

- database path;
- schema version;
- canonical game count;
- source names and releases;
- position-index version and status;
- concise diagnostic information useful when reporting a problem.

This is likely to be more useful than a large general Settings dialogue.

#### Settings

Keep Settings deliberately small.

A setting should exist because reasonable Bermuda users may genuinely want
different persistent behaviour, not because desktop applications traditionally
have many preferences.

Review:

- handicap-game inclusion behaviour;
- persistence of window size and splitter positions;
- board-coordinate display only if real use demonstrates a need for a choice.

Prefer sensible automatic behaviour over settings.

Do **not** expose implementation details such as Bermuda thresholds, edge bands
or matching heuristics as preferences.

#### Desktop usability

Review:

- keyboard navigation and useful shortcuts;
- restoration of appropriate window and layout state;
- high-DPI and display scaling;
- light and dark desktop themes;
- user-facing error messages;
- behaviour when opening SGFs or databases from the desktop where supported.

These are first-release quality issues rather than new Go features.

### 5. Release-readiness pass

Release readiness now has higher priority than speculative new search features.

Review:

- README and documentation under the Bermuda name;
- compatibility notes for deliberately retained identifiers such as
  `MOYODB-*` and `moyodb-project.toml`;
- references to the former MoyoDB application-data path;
- backup documentation;
- experimental and development CLI commands, distinguishing them from normal
  user-facing functions;
- database and index versioning expectations;
- schema migration documentation;
- supplied player-catalogue provenance and update procedure;
- licence and acknowledgement material;
- version numbering between the core and GUI crates.

Then define what constitutes the first meaningful Bermuda release.

### 6. CI and Linux packaging

Establish automated verification of clean builds and tests.

Prepare a sensible Linux packaging and installation route and review:

- desktop integration;
- application metadata;
- icons and other application identity;
- clean installation;
- first-run behaviour;
- installation of QML/Kirigami dependencies;
- appropriate release versioning.

Linux remains the currently tested and supported target.

Windows and macOS remain possible future targets but should not yet be described
as supported Bermuda platforms.

## Subsequent major work

### 7. KataGo integration — future analytical layer

Investigate optional KataGo integration as a separate analytical layer
alongside Bermuda's professional precedent search.

Bermuda and KataGo answer different questions:

- **Bermuda:** What have strong human players actually done from positions like
  this?
- **KataGo:** What does a strong Go engine think about this position and its
  continuations?

Preserve that distinction in architecture and interface.

KataGo judgement must not silently alter, rank or suppress Bermuda's
professional-game evidence. A disagreement between professional precedent and
engine judgement is potentially useful information.

Possible future uses include:

- analysing the currently displayed position;
- examining KataGo candidate continuations alongside professional precedents;
- showing evaluation changes where they genuinely aid investigation;
- comparing a professional move with engine alternatives;
- providing an analytical layer for the future personal-game corpus.

Do not initially turn Bermuda into a general-purpose KataGo GUI.

### 8. Personal-game corpus and analysis architecture

Design a separate corpus or database for personal games.

Its long-term purpose is collective analysis rather than merely replay:

- pattern-search recurring situations across personal games;
- identify recurring mistakes or habits;
- compare personal continuations with professional-game precedents;
- use professional precedent as evidence about what strong players actually
  did;
- optionally combine this later with Bermuda's separate KataGo analytical
  layer.

Keep professional evidence and engine evaluation conceptually distinct.

Architecture and questions come before implementation.

## Deferred polish

### Catalogue-loading animation

The existing catalogue-loading animation is functional and can remain for now.

At some later point, replace it with a more authentic whimsical animation using
two small Go bowls:

- one bowl containing and spilling only white stones;
- one bowl containing and spilling only black stones.

No mixed-colour bowl.

This is the small animation shown while the catalogue is loading. It is
separate from any future application splash screen.

A polished splash or start-up presentation may also be considered later, but
must never introduce artificial delay.

Cosmetic work should not displace functional or release-readiness work.

## Deliberately deferred

Do not turn Bermuda into a cut-price KataGo.

For now, defer:

- automatic move ranking or verdicts;
- score estimation;
- whole-board strategic evaluation;
- automatic claims that recent professional play is better;
- automatic strategic classification based merely on pattern density, opening
  coordinates or similar heuristics;
- further Bermuda-threshold tuning without a concrete failure from normal use.

## Guiding principle

Bermuda is a professional precedent finder.

Its central question remains:

> **We got here somehow — who has got a map?**

The program should expose professional evidence clearly and let the user
investigate what the professionals did next.
