# Bermuda — 1.0 Status and Workplan

_Last updated: 2026-09-22 · branch: `master` · base checkpoint: `553fdbd`_

## 1.0 objective

Bermuda is now close enough to a complete desktop application that the workplan
should be organised around a **1.0 release**, rather than around adding every
feature that might eventually be useful.

For 1.0, Bermuda should be a dependable Linux desktop application for:

- building and maintaining a user-owned professional 19×19 game corpus;
- browsing and replaying games;
- searching professional positions and patterns;
- examining professional continuations;
- creating persistent Studies without modifying source games;
- editing Study game trees safely;
- consulting the OGS Joseki Explorer inside Bermuda;
- keeping and replaying personal games in My Games;
- playing local games;
- optionally analysing positions with KataGo.

A 1.0 release does **not** need to contain every planned analytical, statistical
or online feature.

The guiding principle for the remaining work is therefore:

> finish and harden the workflows Bermuda already has before widening the
> application again.

## Current application

### Professional database

The professional database is a user-built corpus rather than data bundled with
Bermuda.

The intended corpus is professional 19×19 games. Bermuda already supports:

- project creation and opening;
- SGF import;
- duplicate detection;
- source/provenance preservation;
- catalogue browsing;
- replay;
- search and filtering;
- player-name normalisation without changing imported PB/PW text;
- a supplied player catalogue plus local corrections;
- identity-aware player search;
- import progress and error reporting.

Documentation should continue to explain how users can obtain and maintain
their own corpus rather than implying that Bermuda ships one.

### Pattern search

Pattern search is a mature core Bermuda workflow.

It supports:

- rectangular board selection;
- search across board locations;
- rotations and reflections where applicable;
- exact stone and empty-point context;
- professional continuation inspection;
- multiple occurrences within a game;
- continuation filtering;
- handicap-game handling;
- the conservative geometric Bermuda suppression heuristic.

The selected rectangle defines the search context. Stones and board edges
outside it are deliberately excluded.

The rubber-band/search-area semantics and physical board-edge indication have
now been clarified in the interface. These are no longer outstanding workplan
items.

Do not continue tuning Bermuda's geometric heuristic without a concrete failure
found during normal Go study.

### Study and Study Library

Study is now a permanent Bermuda workspace with:

- **Current study**;
- **Study Library**;
- **Joseki Library**.

Study uses copy-on-write ownership. Professional games, My Games and external
SGFs remain source material. Editing creates or updates a separate Study Library
copy; the original source is never modified.

Implemented Study functionality includes:

- replay of SGF variations;
- a literal structural SGF tree as well as replay positions;
- comments;
- SGF markup;
- Bermuda annotations;
- persistent Study Library storage;
- provenance metadata;
- opening and deleting Study documents;
- safe Study Library ownership rules;
- Add move;
- Pass;
- Branch;
- Insert node;
- Prune from here;
- Undo and Redo for the current editing session;
- automatic persistence of Study edits;
- Done / return to Study Library;
- a clearer **Annotate / Moves / Edit** tool organisation.

Branch now has a distinct meaning:

- **Branch** adds one or more sibling continuations from a fixed branch point;
- **Add move** extends the selected continuation.

There is therefore no planned **Insert move** command. It does not solve a
separate normal Study task.

### Native OGS Joseki Explorer

The Joseki Library reads the community-curated OGS Joseki Explorer natively.

Implemented behaviour includes:

- on-demand OGS loading;
- local caching;
- cached fallback when offline;
- OGS attribution and external View on OGS action;
- OGS commentary and source information;
- OGS board references and links;
- preservation of OGS move categories and labels;
- a lazy Bermuda-style joseki variation tree.

OGS remains an external reference source. Local editable material belongs in
Study.

### My Games

A separate personal-game corpus already exists.

It supports:

- creation on first use;
- catalogue browsing;
- replay;
- pattern search;
- local game play;
- legal moves and captures;
- pass, undo, resign and finish;
- retained review of completed games;
- SGF export;
- adding completed played games to My Games.

The larger longitudinal-analysis purpose of My Games is deliberately later
work.

### KataGo

KataGo is an optional analytical layer, not part of Bermuda's professional-game
evidence.

Current-position analysis exists, including:

- the Rust protocol/process adapter;
- position conversion;
- setup stones, passes, history and side-to-move handling;
- visit-budget control;
- inline KataGo investigation.

Bermuda and KataGo answer different questions:

- **Bermuda:** what have strong human players actually done?
- **KataGo:** what does a strong engine think about this position?

Those forms of evidence should remain visibly distinct.

## Remaining work for 1.0

### 1. Finish the bounded SGF editor work

The editor is now functionally substantial. Do not reopen it into a general SGF
authoring project before 1.0.

The remaining small completeness item is:

- **Make main variation** — allow a selected side variation to become the first
  or principal continuation at its branch point, preserving all siblings.

This should live under **Tree** alongside Branch, Insert node and Prune from
here, participate in Undo/Redo, and autosave normally.

Before declaring the editor complete, exercise real SGFs containing comments,
markup, non-move nodes, setup stones and several levels of variation to confirm
that unrelated SGF properties and topology remain intact.

Editing arbitrary SGF properties and direct setup-stone authoring are not 1.0
requirements unless real use exposes a concrete need.

### 2. Complete OGS → Study transfer

**Study this position** should copy useful explored joseki material into an
ordinary local Study.

The desired result is:

- copy the explored OGS subtree, not merely a single current position;
- retain the explored current path;
- retain commentary and markup where available;
- retain meaningful OGS provenance;
- produce an ordinary editable Study Library document;
- leave the OGS source untouched.

This is the main remaining bridge between the Joseki reference workflow and
Bermuda's local Study workflow.

### 3. Make KataGo release-quality

KataGo is optional, but if it is presented as a normal Bermuda feature its
interactive behaviour should be release-quality.

The important engineering work is:

- keep the engine alive between related analyses where practical;
- do not block the GUI during analysis;
- allow obsolete analysis to be cancelled or superseded;
- recover cleanly from engine failure;
- shut the engine down cleanly;
- provide clear configuration and diagnostics;
- behave gracefully when KataGo is absent.

Whole-game analysis, evaluation graphs and corpus-wide stored analysis are not
required for 1.0.

If the lifecycle work proves disproportionately large, Bermuda 1.0 should still
remain fully usable without KataGo rather than delaying the core application
indefinitely.

### 4. Release hardening

This is now as important as new functionality.

Before 1.0:

- run the full automated test suite from a clean checkout;
- review database/index versioning and migration expectations;
- verify that user data is never overwritten by upgrades;
- verify Study copy-on-write behaviour with professional games, My Games and
  external SGFs;
- test large-corpus import and search using realistic data;
- test empty, first-run and missing-resource states;
- improve error messages where failures currently surface as developer
  diagnostics;
- check keyboard navigation and basic accessibility;
- make the principal windows and panels behave sensibly at realistic desktop
  sizes;
- remove or clearly mark developer-only commands and controls.

### 5. Documentation and onboarding

A 1.0 application needs enough documentation to be used without knowledge of
the development history.

Provide or update:

- a concise user guide;
- first-run guidance;
- professional-corpus creation and update instructions;
- database/project concepts;
- pattern-search semantics;
- Study and Study Library;
- SGF editing;
- My Games;
- OGS Joseki use and attribution;
- KataGo setup and its optional status;
- backup and recovery guidance;
- compatibility notes for any retained legacy MoyoDB storage identifiers.

The application should not imply that a professional-game database is bundled
with Bermuda.

### 6. Packaging, CI and release process

The first supported release target remains Linux.

For 1.0:

- produce a repeatable release build outside the development tree;
- package required Qt/KDE runtime components appropriately;
- keep user databases and settings outside package-controlled locations;
- establish CI for build and tests;
- create a repeatable release/versioning procedure;
- test installation, upgrade and removal without harming user data;
- prepare release notes and a simple issue-reporting path.

### 6.1 Native Windows 10/11 build

Before 1.0, produce a native x86-64 Windows build of Bermuda and prove that it
runs on a clean Windows 10/11 machine without requiring Rust, Qt development
tools, or a Bermuda source tree.

The first Windows milestone is deliberately modest:

- build a normal native `Bermuda.exe`;
- deploy the Qt/KDE runtime DLLs, plugins and other required resources beside it;
- package the result as a self-contained test directory or archive;
- confirm that Bermuda starts and its principal local workflows function on a
  clean Windows 10/11 installation;
- verify Windows path handling, per-user data locations, process launching and
  optional KataGo discovery/configuration;
- record any portability defects exposed by the Windows build before 1.0.

A single-file executable is not required. The goal is a conventional Windows
application that can be unpacked and run without a development environment.

This is a portability and release-readiness milestone. Windows does not need to
be declared a fully supported 1.0 platform until the resulting build has had
enough real testing. macOS remains later.

## 1.0 acceptance criteria

Bermuda is ready to call 1.0 when a fresh user can, without development tools
or project knowledge:

1. install and start Bermuda on the supported Linux target;
2. create/open a professional-game project and import SGFs;
3. browse, search and replay that corpus;
4. understand the selected pattern/search-area semantics;
5. create a Study from source material without modifying the source;
6. annotate and structurally edit that Study, including variations;
7. close and reopen the Study from the Study Library;
8. use the native OGS Joseki reference and transfer useful explored material to
   Study;
9. create/use My Games and save a locally played game;
10. use Bermuda normally when KataGo is not installed;
11. use current-position KataGo analysis without freezing or destabilising the
    application when KataGo is configured.

There should be no known data-loss bug, source-SGF mutation bug, reproducible
crash in a principal workflow, or database migration problem.

## After 1.0

The following remain worthwhile, but should not hold up the first stable
release.

### My Games analysis

Develop My Games into a longitudinal study tool:

- whole-game KataGo-assisted review;
- identify interesting moments;
- pattern-search recurring situations across personal games;
- identify recurring mistakes, habits and difficult shapes;
- compare personal continuations with professional precedent;
- support long-term review across many games.

### Richer KataGo analysis

Possible post-1.0 work:

- whole-game/background analysis;
- persistent analysis results;
- evaluation graph;
- richer candidate variations;
- analysis provenance and reanalysis policy;
- possible play-against-KataGo mode.

### Statistics and specialised explorers

Possible database-facing additions:

- player statistics;
- win rates;
- tournament summaries;
- opening/fuseki statistics;
- richer joseki and fuseki exploration;
- tighter links between statistics, source games, Study and pattern search.

### Further pattern-search ideas

Reassess rather than automatically implement older ideas such as:

- colour-independent or friendly/enemy matching;
- wildcard / don't-care points;
- specialised fuseki-oriented filtering;
- additional result ranking or sorting.

The existing geometric Bermuda search should remain simple unless ordinary use
demonstrates a real deficiency.

### Online play

Online play remains deliberately post-1.0.

Two possible directions remain:

- Bermuda as an OGS playing client;
- Bermuda-to-Bermuda peer-to-peer play.

Both require substantial networking, state synchronisation and fair-play work
and should not distract from completing a stable local application.

### Other platforms

After a stable Linux release:

- investigate a native Windows build and self-contained test distribution;
- consider macOS only after the core cross-platform deployment model is proven.

## Immediate sequence

The preferred route from the current tree to 1.0 is now:

1. implement **Make main variation**;
2. exercise and close the SGF-editor milestone;
3. implement explored OGS subtree → Study transfer;
4. finish KataGo lifecycle/responsiveness needed for ordinary interactive use;
5. perform documentation, first-run and error-handling review;
6. establish Linux packaging, CI and release procedure;
6.1. produce a native Windows 10/11 `Bermuda.exe` test distribution that runs
     without development tools;
7. run a real-use release-candidate pass against professional corpus, Study,
   My Games, OGS and optional KataGo, including a Windows portability pass;
8. fix release-blocking defects only;
9. release Bermuda 1.0.

Do not add speculative features merely because 1.0 is approaching. The shortest
path to 1.0 is now to make the existing application dependable, understandable
and distributable.
