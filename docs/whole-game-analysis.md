# Whole-game analysis: first implementation

Open a personal game, a selected professional game, or a Study. In Study, choose **KataGo → Whole game…**. Select visits per position, komi and rules,
then **Start analysis** (or **Resume analysis** for a partial report). The review workspace covers the area left of the board, including the game tree;
the board and divider keep their sizes. **Return to database** or **Return to
Study** reveals the previous view without resetting its search. Returning does
not pause an active analysis; use **Pause** first when desired. Japanese rules are an explicit initial assumption,
not inferred from SGF. Analysis covers position zero through the last position
of the loaded line; for branched Studies this is the selected replay line.

The source database/SGF is never rewritten. The engine runs on a background
worker, one position at a time, with full prior move history and setup stones.
A separate worker owns the game pass; interactive analysis on that controller
is released before starting and is unavailable until the pass stops.

## Saved results and recovery

`game-analysis.sqlite3` is stored in Bermuda's per-user KataGo working directory.
Each completed position is committed before progress is shown. Pause cancels
the current request; a partial current position is discarded. Reopen the same
game/line and use **Start / resume** with the same settings after a pause or
restart. Complete saved runs reload without launching an analysis process.
The configured engine/model/configuration files must still be accessible to
identify the matching run. The initial version has no separate saved-run browser.

The run key includes the complete ordered move history, resolved setup,
initial player, board size, rules, komi, visit budget, policy version, and
SHA-256 hashes of the executable, network and configuration contents. Changed
settings/files create a separate run and retain old results. Executable hash
identifies the exact engine build (a human-readable KataGo version query is
not yet implemented). File paths, creation time, Bermuda version, the explicit
Black perspective override, warnings, actual visits, scores, win rates and
candidate principal variations are retained.

Do not replace engine/network/configuration files while a run is active.
Configurations that include other files are not fully fingerprinted in this
first version: changes in included files require a change to the primary config
(e.g. a comment) to create a new analysis identity.

## Reading the report

The title identifies the currently loaded game. A stored report is displayed
only when its complete replay line matches that game. Opening another game
hides the previous report; starting analysis reuses matching checkpoints. If
another game is still being analysed, pause it before starting the new game.

Switch the graph between Black-perspective score and Black win probability. Click a saved point to visit
that position; the decision list takes you to the position **before** the
chosen move. Navigation verifies the currently loaded replay line against the
analysed snapshot so a report cannot silently navigate another game. The **Candidate moves** tab shows the top five candidates by visits, with
Black-perspective estimates and first-move coordinates. Lettered markers
identify their first moves on the board; selecting a row highlights its marker.
Pass candidates have no board marker. The AI checkbox hides these suggestions.
The **Summary** tab counts estimated loss bands for each colour, treating
negative losses as zero for grouping. Engine and network provenance is available under **Analysis details**.

Estimated loss is the difference between adjacent position scores, signed for
the player who moved. Negative values are retained in storage. The list shows
the largest 20 estimates of at least one point. These are noisy comparisons of
two independent searches, particularly at low visits, not definitive mistake
labels. Final positions (including passes) are analysed, but resignation is
not treated as a move. Visits shown beside a position are actual engine visits.
Rate and ETA are rough position-throughput estimates for newly completed work.

## Acceptance checks

1. Analyse a short game at low visits with known rules and komi; check both
   colours' score signs and the final position.
2. Pause midway; resume and confirm completed positions are reused.
3. Close and restart Bermuda after pausing; reopen the game and resume.
4. Restart a completed run: it should load saved results promptly.
5. Change visits or komi: a separate run should begin. Restore the original
   settings and confirm the prior results reload.
6. Try a selected professional game, a handicap/setup game and a game with
   passes; check board navigation and suggested coordinates.
7. Switch to a different game while a report is open: graph navigation must
   refuse to move the unrelated board.
8. Resize the window and divider: the panel should fit the left pane and scroll
   at smaller sizes. Return should reveal the original view and preserve its search.
9. After pausing/completing, leave the review and use the existing interactive
   Analyse control for a deeper investigation of a selected position.

Automated core tests cover checkpoint reopening, run separation, score signs,
and a synthetic engine protocol run with pause/resume and cached completion.
Real-engine and full-application checks are separate acceptance work.

## Preview and save suggested continuations

The default is a small set of candidate next moves, with matching lettered
board markers and Black-perspective estimates. Click a board letter or a row
to preview that move immediately; **Return to position** restores the source position.
Other candidate letters remain available during a single-move preview.
Selecting another candidate lets the player compare alternatives. Preview
uses a temporary board and does not edit the game or move its slider.

After pausing/completing analysis, **Save move to study** adds only the chosen
next move with an analysis/provenance comment, as one undoable edit. It follows
an existing matching move rather than duplicating it and preserves existing
comments and sibling variations. The normal Study Library copy-on-write path
protects imported games and external SGFs.

Long engine sequences are supporting detail, hidden by default. **Explore engine
continuation (optional)** shows the line as numbered stones on the existing board.
Previous/Next and a slider replay it from the source position, including captures
and passes. Captured stones lose their labels; passes appear in the move counter.
Candidate letters are hidden beyond the first move; selecting another row returns
to a single-move preview. A separate **Save engine
line to study** action. That explicitly saves the whole line. Neither candidate
estimates nor a principal variation are presented as instructions for the player.

**Bermuda organises evidence; KataGo evaluates; the player investigates.**

### Deeper candidate investigation

Select a candidate and use **Analyse this candidate more deeply**. The visit
budget defaults to at least 1,000 (or five times its current visits), up to
100,000. Pause whole-game analysis first. **Stop** cancels the focused search
and retains previous results.

The request includes the original game history followed by the selected move.
KataGo therefore searches the replies to that move, even if it originally gave
the move little attention. Completed results update that candidate's estimate,
visit count and graphical continuation. Letters remain stable. These focused
visits describe a separate search after the move; they are not added to the
whole-game visit total or used to alter its score graph or decisions list.

Completed refinements are stored alongside the matching game analysis and
restored with it. Engine/network/configuration hashes must match the original
run. Saving a refined move or line into a study includes its focused-search
provenance. The technical **Analysis details** dialog is no longer shown.

Preview/save verify that the complete loaded replay line matches the report.
An invalid continuation is rejected without changing the in-memory collection.
Saving is disabled while whole-game analysis is running.

## Acceptance checkpoint — 15 October 2026

The user confirmed that the graphical continuation works and subsequently that
the deeper-candidate patch works, finding it useful to investigate why a
plausible move falls short. Functionality is accepted for the current milestone;
explanatory wording and control arrangement remain a follow-up after reflection.

Local checks passed for QML syntax, mocked-controller interactions, candidate
letter stability and incremental patch applicability. Rust tests were supplied
for request history/side-to-move, Black score perspective, persistence and
retention of prior results on cancellation or changed engine files. This
development environment has no Rust compiler; no separate test-output transcript
was supplied with the user's acceptance.

Retain pass/capture previews, save/undo/redo/reopen and source-game preservation
in release regression checks; acceptance here does not claim exhaustive coverage.

## Current-position parity — 26 October 2026

The ordinary **Analyse** button now opens the shared candidate review panel in
position-only mode and searches only the displayed position. Whole-game review
remains a separate action. The earlier current-position circles were display-only;
that split was an implementation gap, not the intended interaction.

Both routes now provide lettered board candidates, per-candidate Black score and
win estimates, single-move previews, numbered continuation stepping, deeper
candidate investigation and saving into Study. Position-only mode hides the
whole-game graph, decisions and summary tabs. Navigating away clears the preview
and hides candidates that do not belong to the displayed position.

Position runs preserve move history and have separate cache identities from
whole-game runs. They save only the requested turn, including its original move
number, and can restore completed deeper-candidate investigations. Existing
whole-game checkpoints remain compatible.

QML interaction and syntax checks passed locally. A Rust regression test covers
single-turn dispatch, cache separation, restore and deeper analysis at a nonzero
turn. Rust build/test execution and real-engine acceptance remain to be checked
on the user's machine.

## Follow-up refinements

### Recommendation, alternatives and “Why not this?”

KataGo markers no longer share the red pattern-search circles or encode visits
through their size. A filled teal marker identifies the unrestricted search's
recommendation; quieter blue markers identify alternatives. Explicit visit
counts remain in the candidate rows. A focused search's forced visit count
does not promote that move to the unrestricted search recommendation.

**Why not this?** restores the source position and lets the player click a
legal move, or choose **Try pass**, for investigation at the deeper-search visit
budget. Occupied points, suicide and ko are rejected before starting the worker.
No study move is inserted. On completion, an additional move not in the original
candidate list is shown in purple as **Your idea**, automatically selected and
previewed. It supports the same graphical continuation, deeper analysis and
explicit saving into Study. Completed investigations are retained in saved
reports; comments distinguish player-proposed moves from engine suggestions.

The continuation is evidence to investigate, not an automatic verbal explanation
or proof that an alternative is inferior. A future quiz can reuse the chosen-move
analysis path, but quiz selection, scoring and hidden answers are not implemented
by this change.

### Reopening saved evidence

**Open latest saved** loads the newest non-empty report for the exact loaded
game history and review scope. Whole-game mode finds a whole-game report;
position mode finds a report for the displayed turn. It restores the saved
rules, komi, visit budget and deeper-candidate refinements without starting
KataGo or requiring the original engine/network/configuration files.

The newest report can be partial. Whole-game navigation moves to a saved
position if the current move is beyond its progress. Further analysis still
requires a working engine and the appropriate settings. Selecting among older
reports is separate future work.

- Saved-run browsing and comparison of old settings, including runs whose
  engine files are no longer installed.
- Human-readable engine version and full configuration-include provenance.
- Review explanatory wording and control arrangement after user reflection.
- Broader saved-position comparisons beyond the implemented per-candidate
  refinements; keep focused searches distinct from the original score graph.

Recurring-pattern analysis across My Games remains item 4.
