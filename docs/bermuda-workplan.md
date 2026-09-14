# Bermuda Workplan

**Status:** Canonical project-wide work plan.

**Last updated:** 14 September 2026.

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

The Qt/Kirigami application has a task-oriented structure around:

- Game;
- My Games;
- Game Database;
- Pattern Search;
- View;
- Help.

The goban is the central working surface.

Pattern Search and KataGo are peer investigation modes. Influence is an
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

The current limitation is architectural rather than conceptual: each analysis
starts a fresh KataGo process and blocks the GUI while it runs.

## Development order

### 1. KataGo lifecycle and responsiveness

This is the immediate next engineering task.

Make interactive engine use suitable for ordinary study:

- keep KataGo alive between analyses;
- avoid reloading the neural network for every request;
- move analysis off the GUI thread;
- show a clear analysis-in-progress state;
- allow obsolete work to be cancelled or superseded;
- recover cleanly from engine failure;
- shut KataGo down cleanly with Bermuda;
- preserve deterministic tests without requiring a real engine.

Only after process-start overhead is removed should the default visit budget be
judged again.

**Completion criterion:** several positions can be analysed during one Bermuda
session without repeated model loading or a frozen interface.

### 2. Session continuity

Restore the durable study document and current move after restarting Bermuda.

The initial implementation should handle separately:

- an external SGF;
- a professional-database game;
- a My Games game.

Restore the document by provenance rather than merely serialising a board
position.

Do not silently turn this into autosaving of arbitrary editable positions or
unfinished live games. Those require a separate decision.

**Completion criterion:** close Bermuda while reviewing a durable game, reopen
it, and return to the same game and move.

### 3. Unified investigation interface

Develop KataGo using the useful interaction grammar already established by
Pattern Search.

Work includes:

- candidate moves shown on the goban;
- a concise candidate list;
- a clear distinction between root evaluation and evaluation after a candidate;
- principal variations;
- stepping through a variation;
- easy return to the source position;
- useful candidate comparison;
- preservation of Pattern Search state while temporarily using KataGo, and vice
  versa.

Professional-search evidence and KataGo evidence must remain visually and
conceptually distinct.

A later useful bridge is to allow a KataGo candidate or variation position to
become the source of an independent professional search.

**Completion criterion:** KataGo feels like a Bermuda investigation tool rather
than a separate engine-result dialogue.

### 4. Occurrence context and search evolution

The current search system is good at answering what professionals played next.
The next search-development layer should expose more of the context in which a
matching occurrence arose.

Useful information includes:

- first and last matching positions;
- pattern duration;
- forming move or capture;
- preceding local moves;
- next local move;
- tenuki and delayed return;
- local activity and board occupancy;
- grouping repeated matches into useful appearances or local episodes.

Possible aggregate views can then include formation maps, later local activity,
move-number distributions and representative examples chosen by explicit,
explainable criteria.

This work must remain inspectable. Do not replace transparent evidence with an
opaque relevance score.

**Completion criterion:** the user can readily investigate how a matching
professional situation arose, persisted and continued, not merely that its
stone geometry matched.

### 5. Personal-study workflow

The distinctive long-term question for My Games is:

> **What do I repeatedly do across my games?**

The intended workflow is:

    personal games
          |
          v
    interesting moments
          |
          v
    recurring shapes or decisions
          |
          v
    repeated personal behaviour
          |
          v
    independent professional search
          |
          v
    comparison with professional continuations

KataGo may identify candidate moments worth examining. It should not become an
automatic authority that defines every departure from its preference as a
mistake.

Initial work should favour Bermuda's existing machinery:

- analyse selected personal games;
- retain derived engine analysis with clear model/configuration provenance;
- identify potentially interesting moments;
- find repeated personal positions or patterns;
- group recurring decisions across games;
- launch independent searches against the professional corpus;
- expose the supporting professional games and continuations.

Later questions may include whether recurring problems diminish over time,
whether particular behaviours differ by colour, and recurring strengths as well
as weaknesses.

**Completion criterion:** Bermuda can give at least one evidence-backed answer
to "what do I repeatedly do, and what did professionals do in comparable
situations?"

### 6. Release hardening

Release work should proceed alongside research development rather than waiting
for every long-term idea.

A useful first packaged Bermuda release does not require the complete
longitudinal My Games vision. It does require the existing professional-study
workflow to be dependable and understandable.

Work includes:

- concise user help;
- useful first-run and empty states;
- database information and diagnostics;
- appropriate restoration of desktop state;
- clear user-facing errors;
- high-DPI and theme checks;
- accessibility and keyboard review;
- database/index compatibility documentation;
- migration expectations;
- licence and acknowledgement review;
- CI;
- reproducible Linux builds;
- Linux packaging and desktop integration.

Linux remains the primary platform during this work. Native Windows support
comes afterwards.

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

> **Make KataGo long-lived and asynchronous.**

Before changing code, inspect the current process ownership and the Rust/Qt
threading boundary.

Keep the tested Rust KataGo adapter as the protocol boundary. QML and other GUI
code should not parse KataGo JSON.

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
