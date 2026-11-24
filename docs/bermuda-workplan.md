# Bermuda Workplan

**Status:** Canonical project-wide work plan.

**Last updated:** 23 November 2026.

## Latest workflow update

23 November 2026: Study → My games now checks the existing player-name aliases.
Recognised personal games offer Add to My games; existing entries show In My
games. This is my game… lets the user choose Black/White for one game, with an
optional remembered alias for future imports. Game-local links and remembered
names are saved atomically with the imported metadata; source SGF data and the
open study are preserved. Core identity/import regression tests pass; full
Rust/Qt desktop build and interactive acceptance remain to be checked.

QML parsing and an isolated dialog interaction check also pass.

Saved-study deletion follow-up: deleting an open saved copy now keeps the board,
notes and variations in memory as an unsaved study. Active, parked, search-source
and retained played-game references are detached only after the file is removed
successfully. Save study then creates a new file. Confirmation explains this;
success and errors are visible in a dialog. Original SGFs, My games and KataGo
analysis are untouched. Desktop acceptance remains pending.

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

## Played-move visibility — 24 November 2026

Implemented: single-position review now shows the recorded next move with the
same square outline as whole-game review, including when it coincides with a
KataGo candidate. The played move's colour, coordinate and move number stay in
the fixed heading. Removed the redundant “differs from recommendation” warning;
a matching best move still has a short confirmation. Passes remain textual, and
trial variations do not display a misleading recorded-move marker.

Validation: the actual QML review panel passes an isolated controller test for
single-position and whole-game review, candidate overlap, investigate/return,
preview suppression, stale reports, passes and game end. Full application build
and visual confirmation on Gerry's desktop remain the final local checks.

Next: Chamber of Horrors report design.

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
corrected before acceptance; committed in `51092cc`.

The latest 20 games is a convenient current view, not a retention limit or an
agreed statistical sample size. Saved per-game analysis is the foundation;
fixed groups and historical comparisons are not yet implemented.

#### 4A. Preserve named game groups and analysis snapshots — implemented and accepted

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
application use and graphical printing were accepted on 5 November 2026. Side-by-side group comparison is 4D.

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

18 November 2026: a first distribution-trend increment is implemented for
acceptance. Reports now has an explicit library, new-report workflow and in-app
Trends tab; the live report count and saved-group recent-game selection are user
selectable. Trends reuse frozen loss-bucket percentages, separate incompatible
settings, show coverage and dates, and flag shared games. Existing reports are
preserved. This brings forward the requested numerical trend view; theme-based
comparisons and interpretation still depend on 4B/4C and remain below.

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

4A was accepted and committed in `521aa04`: fixed named groups, frozen analysis
snapshots and graphical printable reports. The next main increment is **4B**,
keeping and labelling decisions for study.

The 7 November review polish adds recorded player ranks and a square board marker
for the actual next move, including when it overlaps an engine candidate. Verify
these in use before resuming 4B.

The Windows tester reports that resetting AppData permits startup, but creating
even a 30-game database causes the next launch to freeze/crash on an earlier
build. Obtain the exact version and preserve the failing profile/database; compare
a current-build relaunch with manual database opening. The cause is not yet known.

**8 November Study workflow repair:** keep the strapline in About only; group
Game/notes, Moves/variations, Pattern search, KataGo and Influence below the
comments. Expose branching directly and add explicit Save study with confirmation,
retaining automatic saving after edits and updating an existing saved study.
Interactive acceptance of this repair is pending before resuming 4B.

### Planned: remote/cloud KataGo analysis

Offer **This computer** and **Remote/cloud engine** in KataGo setup, using the
same review, whole-game and personal-batch workflows. This is an agreed work item;
provider selection and implementation are pending the user's service suggestions.
Investigate suitable APIs or a user-controlled remote GPU engine, including any
free/low-cost compute options; do not promise availability or pricing yet.

Before implementation, settle authentication and credential storage, explicit
consent to send game/position data, usage/cost visibility and limits, cancellation,
connection failure and checkpoint recovery. Preserve engine/network/settings and
provider provenance so saved reports remain interpretable and comparable. Remote
compute does not require cloud synchronisation of the user's library.

Keep existing CPU snapshots as a baseline when testing GPU or remote analysis.
A change in analysis quality is not evidence of improvement in the player's past
decisions.

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


### Reports follow-up — 18 November 2026

Implemented for desktop acceptance: live Trends from all identified games' saved
analysis, selectable periods and weekly/monthly/user-sized game blocks. No
scheduled report creation is required. Preserved report comparisons remain a
separate tab. Added report removal and empty-group removal with confirmation,
clear group/report labels and counts; source games and analysis are retained.
Core and offscreen UI checks pass. Next check: Gerry's real collection, including
removing the unused Q4 2026 group and comparing periods with matching settings.

Reports presentation follow-up: added All categories to live trends and saved
comparisons, with four coloured/styled lines, a legend and per-period percentages.
Offscreen category-switching and existing trend calculation checks pass.

Study import fix: support SGF compressed point lists in triangle, square, circle,
cross and selected-point markup, including variation diagrams. A supplied
commented game reproduced the failure; its full Study tree now loads with 246
main-line positions matching replay. Eight Study-tree tests pass. Opening
failures now show a visible dialog instead of only logging to the terminal.
The supplied commentary file is not included in the repository or patch.

Study comment panel: larger 240-pixel default, draggable lower handle and
remembered height. Double-click the handle to reset. Height is constrained to
the available panel space so Study controls remain accessible.

Live trend usability follow-up: combined komi values automatically, retained
engine/depth/rules separation, replaced numbered sets with analysis labels and
contributing counts, added Each game with date or equal game-order spacing,
and trimmed empty timeline ends. Game blocks use contributing games only.
Preserved report contents and comparisons stay unchanged. Calculation and
offscreen UI checks pass; desktop acceptance pending.

### Item 4A — weekly reports, 20 November 2026

Implemented for desktop acceptance: a preview-and-confirm workflow replaces the
reports library with consecutive seven-day reports anchored to the first dated
My games entry. Empty weeks and the unfinished current period are explicit.
There is no per-report game limit. Source games and KataGo analyses are retained;
no new analysis runs. Replacement is atomic and stale previews are rejected.

The anchor persists; importing older games requires an explicit reset to change
boundaries. Live weekly trends share the anchor. Weekly report charts pool komi
values while retaining other analysis compatibility checks. Individual report
removal and custom reports remain available.

Next acceptance: preview Gerry's collection, confirm expected dates/counts,
replace the library, open/print a weekly report, and compare its boundaries with
Live Trends. The graph uses current analysis; reports are preserved snapshots.

Weekly trends follow-up: saved comparisons use report-period starts, live and
saved graphs have intermediate date ticks, empty weeks break connecting lines,
and unfinished periods have hollow markers and explicit labels. Game counts and
coverage remain visible below each graph. Calculation and offscreen rendering
checks pass; desktop acceptance pending.

Trend overview follow-up: added an optional **All** analysis selection to live
Trends and Saved comparisons, combining visit counts and engine settings while
retaining individual selections. Counts are weighted by assessed moves. No
reanalysis or report rewriting occurs. Label kept simply All at Gerry's request;
calculation and offscreen UI checks pass.

22 November 2026: descriptive exported report titles identify weekly date ranges
(and unfinished periods), or custom group/report names, providing useful
suggested PDF filenames. Reopening a report updates its HTML presentation without
changing preserved evidence. Browser print-dialog acceptance remains pending.

22 November 2026: added **Save PDF…** to each saved report. Native Qt PDF export
retains summary charts and selected board diagrams, suggests a descriptive
filename and writes transactionally. Browser export remains available for
optional detailed tables and individual game pages. Snapshot tests, standalone
Qt PDF rendering (including multiple pages), and QML loading checked; full
Bermuda desktop build remains to be checked on the user's installation.

22 November 2026 follow-up: manual KataGo setup detects Linux AppImages,
including extensionless packages, and offers explicit background preparation.
A private extracted copy avoids FUSE mounting in Flatpak; custom model/config
choices remain unchanged. Test setup and Save are still required. Guided setup
continues to verify and extract its pinned downloads. Custom preparation needs
an interactive Flatpak check on the tester's installation before packaging.
