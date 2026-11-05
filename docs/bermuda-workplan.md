# Bermuda Workplan

**Status:** Canonical project-wide work plan.

**Last updated:** 5 November 2026.

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

My Games now has personal identity mapping, saved-analysis report cards and
resumable batch analysis. Recurring themes and changes over time remain future work.

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

## Agreed development order (updated November 2026)

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

**Accepted checkpoint:** The user completed the item 3 workflow review and
accepted the current-position and whole-game candidate workflow, saved results,
deeper investigation and saving continuations into studies. Item 3 is complete
for the agreed scope. Version 0.8.3 release preparation followed on 26 October.
Presentation improvements continued during the 29 October My Games work.

Items 2 and 3 are complete for their agreed scope. Item 4 is in progress.
Release regression checks remain ongoing maintenance.

### 4. Find recurring strengths and weaknesses in My Games

Use saved analysis to identify decisions worth investigating; find repeated
patterns and behaviours across personal games; connect examples to professional
searches; eventually show changes over time.

**29 October first increment:** My games reuses the existing player-name editor
through **Manage names…**. Explicit source-name links share a preferred display
name without rewriting SGFs; one identity can be marked **Me**. The selection
is stored in the personal database, survives rename/restart, and clears on
identity deletion. This is identity groundwork only, not longitudinal analysis.
The user has accepted the simpler name-selection workflow and the default label
**Me**. See `my-games-names.md` for usage, compatibility and validation.

**29 October report-card increment (built and exercised by the user):**
My games gains a report for the most recent 20 unambiguously identified canonical
games. It reuses saved whole-game results, shows complete/partial/pending coverage,
groups comparable settings, charts estimated loss for the player's moves, and
opens decisions in the existing candidate/Why not this workflow. It does not
launch analysis on import. See `my-games-report-card.md`.

**29 October batch increment (running on the real personal collection):**
**Analyse pending games** snapshots the report's pending/partial selection,
processes games sequentially, saves queue and position checkpoints, and offers
explicit pause/resume across restart. Source komi is used with a visible fallback;
engine/settings identities protect resumed results. The report now says **Back
to Home** and unassigned analysis-controller QML warnings are addressed.

**29 October review usability checkpoint:** Decision review identifies game, date,
player colour, actual move and result; separates winning-chance drops from point
losses; adds graph ticks and already-low-win-chance context. The report opens with
an identified analysed game and provides Continue reviewing. Why not this shows
the trial immediately, provides a prominent return action, and keeps temporary
trials separate from lettered engine candidates. Saved trial results are retained
for reuse. The user reports that the revised flow seems good.

**30 October checkpoint:** The report's analysis controls are grouped together,
with Refresh beside the action and pause available even if the progress dialog
is hidden. These changes were tested, committed and pushed. The historical
29 October overnight test is not treated as proof of every recovery scenario.

**3 November import checkpoint (accepted in use):** **My games → Add games…**
accepts several SGFs without replacing the current study, reports duplicates
and file issues, reuses name mappings, and offers analysis options for the
imported selection, including older games outside the latest-20 report.
The user verified a four-file import with three existing games, then the
analysis-options workflow. The initial Qt Dialog property-name conflict was
corrected before acceptance. These changes still need their own commit.

The latest 20 games is a convenient current view, not a retention limit or an
agreed statistical sample size. Saved per-game analysis is the foundation;
fixed groups and historical comparisons are not yet implemented.

#### 4A. Preserve named game groups and analysis snapshots — first implementation, awaiting user acceptance

- Select games explicitly or by a date range, then save a named group with
  fixed membership. Later imports must not change it. Show dates, game count,
  player colour and identity coverage; allow overlapping groups but identify
  overlap when comparing them.
- Keep the group of games distinct from an analysis snapshot. A group may have
  several snapshots; each records the actual analysis runs and their coverage,
  engine/network/configuration identities, visits, rules and komi.
- Freeze the evidence underlying a saved report, not merely a pointer to a
  mutable latest result. Completing partial analysis or analysing more deeply
  creates an explicit new snapshot; it must not silently rewrite the old one.
- Preserve the source game/position references needed to inspect a snapshot.
  Game deletion, identity edits and analysis cleanup must explicitly handle
  these references rather than quietly invalidate history.
- Show complete/partial/pending/excluded coverage and snapshot creation date.
  Saved partial snapshots remain visibly partial.

**First implementation (5 November):** The report card now offers **Groups and
snapshots…**. Select games explicitly or by an inclusive date filter, save a
fixed group, then capture separately named reports. Group membership, names and
your playing colour are fixed at group creation. Main-line SGF copies and full
analysis evidence live in the personal project's `personal-snapshots.sqlite3`,
separate from mutable checkpoints. Source deletion, identity edits and subsequent
analysis do not rewrite a saved report. Unknown/ambiguous identities are retained
as explicitly excluded members; partial and missing analysis remain visible.

**Print / Save as PDF** is included through a self-contained browser report,
with optional individual game pages, score graphs and position diagrams. It
prints the chosen frozen snapshot. This is a read-only archive view; interactive
live review remains unchanged. See `my-games-snapshots.md` for the workflow,
storage/retention policy and checks. Automated preservation tests pass; local
application acceptance remains outstanding. Side-by-side group comparison is 4D.

**Acceptance:** Save an October group and report, import November games, and
reanalyze an October game. After restart, the original membership and report
remain reproducible and the new analysis is separately selectable. Check summary
and individual-game printouts against the preserved evidence.

#### 4B. Collect decisions with the player's interpretation

- Save a decision from review with game, move, player colour, actual move,
  analysis reference, notes and optional variations. Include good decisions
  and uncertain cases as well as estimated mistakes.
- Let the player create and amend labels such as missed threat, unnecessary
  defence or wrong direction. These are interpretations, not engine diagnoses.
- Keep measurable context separate: estimated point loss, change in winning
  chances, already-low winning chances, board region and analysis coverage.
  Phase filters may be useful, but their definition must be visible and
  provisional rather than presented as strategic understanding.
- Reuse the existing candidate, Why not this and save-to-study workflows.

**Acceptance:** A labelled example survives restart and reanalysis, opens at the
correct pre-decision board, and keeps the player's note separate from changes
in engine estimates. Labels and example selection remain editable.

#### 4C. Find recurrence and connect professional evidence

- Group examples by player labels and explicit position/pattern searches.
  Show occurrences, distinct games and the underlying examples.
- Allow related decisions in one fight to be linked as one episode; do not
  count a sequence of consequences as several independent recurrences without
  making that distinction visible. Automatic fight recognition is not required.
- Provide a direct route from a personal example to the existing professional
  pattern search, retaining the chosen area and orientation. Save useful
  professional examples back into the study with provenance.
- Keep professional continuation frequency distinct from KataGo evaluation.
  Geometric similarity is evidence to investigate, not proof of a shared cause.

**Acceptance:** A theme can be inspected across several separate games and
followed into supporting professional positions without losing the original
personal decision or notes.

#### 4D. Compare fixed groups over time

- Compare two named groups using explicitly selected snapshots. Show changes
  in theme frequency, severity and supported strengths, with clickable examples
  on both sides.
- Display denominators: games, assessed own moves, labelled examples and missing
  analysis. Offer rates per assessed move or distinct game where appropriate;
  hand-labelled counts alone must not be presented as unbiased error rates.
- Keep compatible analysis settings distinguishable. Changed engines, networks,
  budgets or coverage may explain a difference; offer a deliberate matched
  reanalysis rather than silently mixing evidence or replacing the baseline.
- Surface relevant context, including handicap/komi, player colour, known
  opponent ranks and game conditions. Different opponents and game types can
  affect apparent progress. Sparse evidence should be described as insufficient.
- Prefer readable comparisons and example lists initially. A radar chart is
  optional presentation, not a reason to invent skill scores. Final measures,
  minimum sample requirements and group sizes will be chosen using real games.

**Acceptance:** Compare two unequal-sized groups and trace every displayed
change to its games and measurement basis. Missing or incompatible evidence and
shared games are visible. No overall playing-strength grade is inferred.

#### 4E. Turn chosen examples into practice — later within item 4

- Let the player turn a reviewed decision into an exercise, hiding suggestions
  until an attempt and then exploring alternatives and continuations.
- Preserve its source position, acceptable alternatives, evidence depth and
  player notes. Practice history is separate from evidence of improvement in
  subsequently played games.
- Reanalysis may change an answer or show that an exercise is unsuitable.
  Offer revision or retirement while preserving previous attempts and notes.
- Repetition scheduling and export can follow demonstrated need. This does not
  bring forward the separate professional-game next-move quiz proposal.

**Acceptance:** An exercise can be attempted, reviewed in its original game,
revised after deeper analysis and revisited without losing its history.

**Scope and inspiration:** The useful documented SWHub sequence is report →
position → investigation → practice. Its Game Focus features informed this
review; fixed named groups and frozen report evidence are Bermuda's explicit
requirements, not an assertion that SWHub implements them. The 3 November
review used public product documentation, not copied implementation code:

- [WeiqiHub overview](https://github.com/ale64bit/WeiqiHub/blob/main/README.md)
  and [BSD licence](https://github.com/ale64bit/WeiqiHub/blob/main/LICENSE.md).
- [SWHub overview](https://github.com/SoumyaK4/SWHub/blob/SWHub/README.md),
  [Game Focus release notes](https://github.com/SoumyaK4/SWHub/blob/SWHub/CHANGELOG.md)
  and [proprietary licence](https://github.com/SoumyaK4/SWHub/blob/SWHub/LICENSE.md).

HumanSL rank/style estimates, inferred human reading depth, automatic strategic
labels and elaborate skill radars remain exploratory, not agreed features.
No proprietary code, assets or wording are to be incorporated. Any future
reuse of BSD-licensed material requires a separate provenance/licence review.
Item 4 remains in progress. Each increment is to be reviewed with real games
before moving to the next; this is not authorization to build every stage at once.

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

The direct-import changes and their workplan update were committed in `51092cc`.
Exercise the first **4A** increment: named fixed game groups, reproducible saved
reports and printing. Verify game selection, retention across reanalysis and
restart, and the report layout before implementing comparisons.

Continue using the existing reports to collect concrete questions and possible
labels. GPU setup and benchmarking can proceed when the card arrives, keeping
existing CPU analysis as a baseline; new hardware is not a prerequisite for
preserving groups. Do not interpret a change in engine/settings as an improvement
in the player's decisions. Windows investigation resumes when tester information
arrives.

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
