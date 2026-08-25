# Bermuda Workplan — 21 August 2026

## Current position

Bermuda's principal professional-game study workflow is working:

- browse the professional-game catalogue;
- select and replay games;
- select a position or rectangular pattern;
- search freely across board locations;
- inspect matching games and multiple occurrences;
- examine professional continuations;
- filter and sort the catalogue and Search Results;
- exclude handicap games by default while retaining them in the database.

Pattern search uses the conservative geometric **Bermuda** heuristic for
long, shallow edge-oriented selections. It remains deliberately geometric:
it is not a model of fuseki, Go strategy or move quality.

The heuristic should now be treated as good enough unless normal use
produces a concrete counterexample. Do not tune thresholds merely to make
result counts look better.

Player identity support is also substantially implemented:

- schema 6 contains players, aliases and nullable player links;
- imported source spellings remain untouched;
- source-specific and global aliases can resolve known identities;
- ambiguous names remain unresolved rather than being guessed;
- catalogue and Search Results use preferred display names for known
  identities while preserving raw source names;
- player-name filtering is identity-aware;
- a Player Identities curation interface exists;
- schema 5 to 6 migration has been tested on the real corpus and the live
  database has migrated successfully;
- migration is protected against concurrent database opens.

The remaining Player Identities question is primarily one of usability.

## Immediate work

### 1. Independent Player Identities usability test

Wait for the independent usability test already under way.

Do not coach the tester through the intended workflow and do not redesign the
interface in anticipation of problems. Observe what is actually confusing:

- what the tester tries first;
- terminology that is not understood;
- hesitation between creating a new player and assigning an existing player;
- operations the tester expects but cannot find;
- mistakes the interface makes easy.

Respond to concrete evidence from the test.

Afterwards, decide whether currently unexposed identity operations, such as
more explicit alias removal or per-game linking controls, genuinely need GUI
treatment.

### 2. Use Bermuda for real Go study

Ordinary use is now part of development.

Use Bermuda for genuine professional-game research rather than constructing
synthetic tests unless a problem needs isolating.

In particular:

- exercise small tactical patterns as well as larger opening selections;
- record cases where Bermuda hides a useful professional precedent;
- record geometrically valid matches that are genuinely misleading;
- notice friction in pattern selection, Search Results, continuation
  exploration, replay and catalogue browsing;
- prefer concrete examples from actual Go study over speculative refinement.

Do not continue tuning the Bermuda heuristic without such evidence.

### 3. Application completeness and first-release UX

Review Bermuda as a standalone desktop application rather than only as a
working Go-search engine.

#### Help and About

Add a **Help** menu appropriate to a first public release.

At minimum consider:

- Bermuda Help / User Guide;
- About Bermuda;
- keyboard-shortcut information when there are enough shortcuts to justify it.

**About Bermuda** should provide useful support information as well as
credits, including:

- acknowledgement of the u-go.net Go Player List, maintained by Ulrich Görtz,
  when its player identity data is included in Bermuda;

- application version;
- a concise description of Bermuda;
- licence information;
- project/repository information;
- build or commit identification where practical.

#### First-run and empty state

Review what a new user sees when no database is available.

The application should explain useful next actions rather than merely appear
empty. Creating or opening a database and adding games should be discoverable
without prior knowledge of Bermuda.

#### Database information

Consider an **About this database**, **Database Information** or
**Properties** facility.

Useful information could include:

- database path;
- schema version;
- canonical game count;
- source names and releases;
- position-index version/status;
- concise diagnostic information useful when reporting a problem.

This may prove more useful in Bermuda than a large collection of preferences.

#### Settings

Keep Settings deliberately small.

A setting should exist because reasonable Bermuda users may genuinely want
different persistent behaviour, not merely because desktop applications
usually have settings dialogs.

Possible or existing settings to review include:

- inclusion of handicap games in pattern searches;
- board-coordinate display if real use demonstrates a need for a choice;
- appropriate persistence of window size, splitter positions and similar
  layout state.

Prefer sensible automatic behaviour over settings wherever possible.

Do **not** expose implementation details such as Bermuda thresholds, edge
bands or matching heuristics as user preferences.

#### Desktop usability

Review:

- keyboard navigation and useful shortcuts;
- restoration of appropriate window/layout state;
- high-DPI and display scaling;
- light and dark desktop themes;
- user-facing error messages;
- behaviour when opening SGFs or databases from the desktop where supported.

These are first-release quality issues rather than new Go features.

#### Splash and start-up presentation

A polished Bermuda splash/start-up presentation may be useful, particularly
once the application is installed and launched independently of a terminal.

It must not introduce artificial delay.

Visual ideas can be explored later; presentation should remain subordinate to
fast and clear application start-up.

### 4. Release-readiness pass

Release-readiness work now has a higher priority than speculative new search
features.

Review:

- documentation under the Bermuda name;
- compatibility notes for deliberately retained identifiers such as
  `MOYODB-*` and `moyodb-project.toml`;
- references to former MoyoDB data paths and commands;
- backup documentation;
- experimental and development CLI commands, distinguishing them from
  ordinary user-facing functions;
- database and index versioning expectations;
- schema migration documentation.

Then define what constitutes the first meaningful Bermuda release.

### 5. CI and Linux packaging

Establish automated verification of clean builds and tests.

Prepare a sensible Linux packaging/install route and review:

- desktop integration;
- application metadata;
- icons and other application identity;
- clean installation and first-run behaviour.

Linux remains the currently supported target.

Windows and macOS remain possible future targets but should not yet be
described as supported Bermuda platforms.

## Subsequent major work

### 6. KataGo integration — future analytical layer

Investigate optional KataGo integration as a separate analytical layer
alongside Bermuda's professional precedent search.

Bermuda and KataGo answer different questions:

- **Bermuda:** What have strong human players actually done from positions
  like this?
- **KataGo:** What does a strong Go engine think about this position and its
  continuations?

Preserve that distinction in both architecture and interface.

KataGo judgement must not silently alter, rank or suppress Bermuda's
professional-game evidence. A disagreement between professional precedent and
engine judgement is potentially useful information in its own right.

Possible future uses include:

- analysing the currently displayed position;
- examining KataGo candidate continuations alongside professional precedents;
- showing evaluation changes where they genuinely aid investigation;
- comparing a professional move with engine alternatives;
- providing an analytical layer for the future personal-game corpus.

Do not initially turn Bermuda into a general-purpose KataGo GUI.

Integration should remain optional and should be driven by useful Bermuda
workflows. Engine analysis and professional-game evidence should remain
visibly distinct.

### 7. Personal-game corpus and analysis architecture

Design a separate corpus/database for personal games.

The long-term purpose is collective analysis rather than merely replay:

- pattern-search recurring situations across personal games;
- identify recurring mistakes or habits;
- compare personal continuations with professional-game precedents;
- use professional precedent as evidence about what strong players actually
  did;
- optionally combine this later with Bermuda's separate KataGo analytical
  layer.

Keep professional evidence and engine evaluation conceptually distinct.

Architecture and questions come before implementation.

## Possible future research — not currently scheduled

### Fuseki filtering

Do not currently treat a Fuseki filter as an active development requirement.

MoyoGo Studio contained an apparently related feature, but its exact purpose
and behaviour are insufficiently understood. More importantly, ordinary use
of Bermuda has not yet exposed a concrete problem requiring an opening-aware
filter.

Do not reproduce an MGS feature merely because it existed.

If real use produces a question such as:

> Show me cases where this formation genuinely arose as part of the opening,
> rather than appearing accidentally later in the game.

collect real examples first and design from that requirement.

Possible solutions might involve move history, opening phase, stone history or
other information. The appropriate solution may or may not resemble the MGS
Fuseki filter.

Do not put speculative fuseki knowledge into the Bermuda geometric heuristic.

## Deferred polish

### Catalogue-loading animation

The existing catalogue-loading animation is functional and can remain for now.

At some later point, replace it with a more authentic whimsical animation
using two small Go bowls:

- one bowl containing and spilling only white stones;
- one bowl containing and spilling only black stones.

No mixed-colour bowl.

This is the small animation shown while the catalogue is loading. It is
separate from any future application splash screen.

This cosmetic work should not displace functional or release-readiness work.

Other cosmetic changes should likewise be driven by things noticed during
real use.

## Deliberately deferred

Do not turn Bermuda into a cut-price KataGo.

For now, defer:

- automatic move ranking or verdicts;
- score estimation;
- whole-board strategic evaluation;
- automatic claims that recent professional play is better;
- automatic strategic classification based merely on pattern density,
  opening coordinates or similar heuristics;
- further Bermuda-threshold tuning without a concrete failure from normal use.

## Guiding principle

Bermuda is a professional precedent finder.

Its central question remains:

> **We got here somehow — who has got a map?**

The program should expose professional evidence clearly and let the user
investigate what the professionals did next.
