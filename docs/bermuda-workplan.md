# Bermuda Workplan

**Status:** Canonical project-wide work plan.

**Last updated:** 26 October 2026.

## Purpose

This document records Bermuda's current development order.

The other documents under `docs/` describe architecture, strategy or individual
subsystems. They may discuss future possibilities, but this workplan is the
source of truth for project-wide priorities.

Bermuda is now well beyond the original sequence of database core, importer,
position index, pattern search and future graphical interface. Those pieces
exist together in a substantial desktop application, alongside My Games,
local play and working KataGo analysis.

The question is therefore no longer simply which isolated feature comes next.
The task is to make these parts form one coherent study workflow.

## Guiding principle

Bermuda's central question remains:

> **We got here somehow — who has got a map?**

**Bermuda organises evidence; KataGo evaluates; the player investigates.**

Bermuda should expose evidence that the user can investigate rather than
replace that evidence with an unexplained judgement.

Three sources of evidence must remain distinct:

- **Professional evidence:** what strong human players actually played.
- **Personal evidence:** what the user repeatedly does in their own games.
- **KataGo evidence:** what a strong Go model estimates about a position.

They may meet on the same goban, but they do not answer the same question.

## Current baseline

The present application already provides:

### Professional corpus

- large SGF collection import;
- canonicalisation and duplicate detection;
- source provenance;
- replay including setup stones, captures, passes and ko;
- position indexing;
- catalogue browsing and filtering;
- exact-position search;
- rectangular pattern search;
- rotations, reflections and colour reversal;
- distinct occurrence and game counts;
- continuation analysis;
- candidate filtering and comparison;
- navigation into supporting professional games.

Pattern search remains deliberately geometric. It should not acquire hidden
strategic judgement merely to make its results look more plausible.

### Desktop application

The permanent Qt/Kirigami layout separates Library (Professional games / My
Games) from Study (new patterns, SGFs, joseki and saved studies). The old UI
switch has been removed. Pattern controls are grouped below compact SGF
comments; matching games appear directly below the continuation summary and
filter in place. Board orientation controls sit beside Return to joseki above
the move slider. See `navigation-trial.md` for the accepted layout.

Pattern Search and KataGo are peer investigation tools. Influence is an
independent overlay rather than a third evidence source.

### My Games

A separate personal corpus exists and already supports:

- creation on first use;
- catalogue browsing;
- replay;
- pattern search;
- local game play;
- legal moves and captures;
- pass, undo, resign and finish;
- retained review of a finished played game;
- SGF export;
- adding a played game to My Games.

The larger longitudinal-analysis purpose of My Games remains future work.

### KataGo

The first interactive KataGo milestone is implemented:

- Rust protocol types;
- process adapter;
- deterministic tests;
- optional real-engine integration tests;
- conversion from Bermuda positions to KataGo positions;
- setup stones, passes, history and side-to-move support;
- analysis of the displayed position;
- visit-budget control;
- inline KataGo investigation mode.

Interactive analysis now runs on a background worker and reuses KataGo across
requests. Whole-game analysis, persistent reports and continuation preview/save
are implemented. The user accepted graphical candidate exploration and deeper
candidate analysis on 15 October 2026.

## Agreed development order (October 2026)

This order supersedes the September roadmap and the older release sequencing in
`workplan-status.md`.

### 1. Fix Windows startup and memory — awaiting tester information

Investigate excessive memory use after indexing and relaunch. Preserve the
affected database, test with an isolated profile, provide startup without
reopening the database, and detect interrupted indexing with recovery/rebuild.
The issue remains open pending further information from a Windows user.

### 2. Improve long-running tasks — current work complete

The 14 October checkpoint (`1aa24f8`) contains task progress, recovery and
continuation, database identity and selection improvements. The user verified
the final default-database usability changes. See `database-operation-progress.md`
for delivered behaviour and limitations. Continue monitoring rates/ETA,
checkpoints, pause/resume, restart recovery, memory and responsiveness as other
long-running operations are introduced; this is not a claim that every task
already implements every facility.

### 3. Complete KataGo game analysis — current work complete

- Analyse whole games, including individually selected professional games.
- Save results with engine/network details and explicit analysis settings.
- Provide a clickable score graph, significant-decisions list and comparison
  of suggested continuations.
- Allow deeper analysis of selected positions and saving variations/comments
  into a study.

Whole-game analysis, per-position SQLite checkpoints, exact-settings resume,
clickable score/win-rate graphs, ranked estimated score changes and candidate
continuations are implemented. Results retain engine, network and configuration
SHA-256 identities, rules, komi and visits. Deeper single-position analysis is
available after pausing a run.

The completed workflow leads with candidate next moves and their estimates.
Clicking a candidate on the board or in the list previews its stone immediately.
Optional engine continuations are shown as numbered stones, with Previous/Next
and a slider. Return restores the source position. A chosen move or full line
can be saved with comments/provenance as one undoable Study edit.

**Analyse this candidate more deeply** searches the position after the chosen
move with a separate visit budget. Completed refinements persist alongside the
matching game analysis, update the candidate's estimate and continuation, and
retain stable candidate letters. The whole-game score graph remains separate.
Stopping retains previous results. The Analysis details button was removed;
engine/network/configuration provenance is still retained internally.

On 15 October the user confirmed that the graphical continuation and deeper
candidate workflow work, and found the latter useful for investigating “why is
this move bad?”. This is player-led investigation, not automatic verbal Go
coaching. The permanent navigation and inline pattern-results layout are also
accepted. See `whole-game-analysis.md` for implementation and validation scope.

**Presentation follow-up:** revisit explanatory text and control arrangement
after the user has reflected. The functionality is accepted; no further layout
change is agreed or required for this checkpoint. Candidate visits show search
effort, not a human explanation of why a move succeeds or fails.

Item 4 is the next major feature. Items 2 and 3 are complete for the agreed
current scope; wider release regression checks remain ongoing maintenance.

**26 October correction:** current-position analysis still used display-only
candidate circles, whereas whole-game review had the accepted interactive
workflow. The correction routes current-position analysis through the same
candidate panel, searching and saving only the selected position. Local QML
checks pass; the patch awaits the user's Rust/Qt build and real-engine check.
Resolve this parity gap before starting item 4.

The user subsequently confirmed that current-position candidate review works.
Item 3 is now undergoing an end-to-end workflow review before presentation
polish and item 4. The first follow-up adds **Open latest saved**, loading
matching saved results without engine files or a new search. This increment
awaits the user's build and walkthrough: reopen, navigate graph/decisions,
compare candidates, deepen a candidate, save a variation and reopen the study.

### 4. Find recurring strengths and weaknesses in My Games

Use saved analysis to identify decisions worth investigating; find repeated
patterns and behaviours across personal games; connect examples to professional
searches; eventually show changes over time.

### 5. Play against KataGo at a chosen strength

Offer tested strength settings, colour, handicap and time controls. Save played
games into My Games for subsequent analysis.

### 6. Later: play other people

Peer-to-peer play and/or Bermuda as an OGS playing client.

Alongside this sequence, continue release maintenance, session continuity,
measured search-speed improvements, search correctness checks and better
presentation of match context.

**Boundary:** Bermuda finds and organises evidence; KataGo evaluates positions;
the player investigates their significance. Move-number filtering and “fuseki
harvesting” are open ideas, not agreed features.

## Parallel maintenance

These activities do not need their own major stage.

### Real-use testing

Use Bermuda for genuine Go study.

Concrete failures found during normal study take priority over speculative
refinement.

Continue watching for:

- useful precedents hidden by search geometry;
- misleading geometrically valid matches;
- friction in search and continuation investigation;
- confusing provenance or corpus behaviour;
- UI instability during long-running work;
- ambiguity between professional, personal and KataGo evidence.

Do not tune search thresholds merely to imitate another program or obtain a
preferred number of matches.

### Professional data

Bermuda should not ship a professional game database.

Keep the import path robust enough that users can build and maintain their own
professional 19x19 corpus while retaining provenance.

Continue to protect:

- reproducible import;
- duplicate detection;
- metadata integrity;
- source provenance;
- index rebuildability;
- clear reporting of failed SGFs.

### Player catalogue

Continue expanding the supplied player catalogue conservatively.

Preserve:

- source provenance;
- stable Bermuda-owned identities;
- exact conservative mappings;
- local corrections taking precedence;
- deterministic catalogue generation.

Do not infer identity merely because two names look similar.

### Documentation

Keep surviving design documents aligned with the implemented architecture, but
do not duplicate the global schedule in each of them.

## Deliberately deferred

Unless real use supplies a compelling reason, defer:

- a special-purpose fuseki subsystem;
- opaque strategic ranking heuristics;
- automatic natural-language Go coaching;
- machine-learning clustering before existing pattern machinery has been
  exploited;
- routine KataGo analysis of the entire professional corpus;
- cloud accounts or cloud synchronisation;
- training a neural network on the user's games;
- speculative search-threshold tuning;
- decorative animation work;
- macOS packaging.

Bermuda should also resist becoming a general-purpose KataGo GUI. Engine
features belong when they improve investigation of professional precedent,
personal experience or the relationship between them.

## Immediate next task

Walk through item 3 end to end, beginning with **Open latest saved**.
Validate saved-result navigation, candidate exploration and Study persistence.
Then hear the user's reflections on the analysis wording and arrangement;
do not redesign it in advance. The next major feature is item 4: recurring strengths and weaknesses
across My Games, using saved analysis and links to professional evidence.
Windows investigation resumes when the tester supplies more information.

## Documentation map

The surviving documents have distinct responsibilities:

- `bermuda-workplan.md` — project-wide ordering and current priorities;
- `bermuda-strategy.md` — Bermuda's research philosophy and interpretation of
  evidence;
- `application-design.md` — application behaviour and interaction model;
- `architecture.md` — overall software architecture;
- `database-design.md` — database and storage design;
- `pattern-search-design.md` — pattern-search semantics and evolution;
- `katago-integration.md` — KataGo integration architecture;
- `personal-corpus.md` — longitudinal My Games design;
- `player-catalogue-design.md` — player identity/catalogue design;
- `building-on-linux.md` — current build and run instructions;
- `packaging.md` — distribution and packaging strategy.

Git history is the archive for superseded design documents.

## Definition of progress

A Bermuda milestone is useful when it improves one or more of:

- correctness of evidence;
- access to the underlying supporting games;
- understanding of the context around a match;
- comparison of independent evidence sources;
- responsiveness and reliability during genuine study;
- ability for another user to install and understand Bermuda.

Feature count by itself is not progress.
