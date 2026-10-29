# My games report card

Open **Library → My games → Report card**. Choose **Manage my names…** first if
Bermuda does not yet know who you are. After changing names, press **Refresh**.

The report selects at most 20 canonical games by descending game date, then game
ID. Undated games come last. Duplicate imports count once. Games with neither
colour mapped to Me, or both colours mapped to Me across their sources, are
excluded and counted separately.

Coverage shows complete, partial and awaiting-analysis counts. The newest
non-empty whole-game run for each exact history is used: a newer partial run
takes precedence over an older complete run. Focused position analysis does not
count as whole-game coverage. Refresh includes new imports and saved results
without launching KataGo or deleting older analysis.

Consistency counts the player's assessed moves in four estimated-loss bands:
less than 1, 1 to below 3, 3 to below 5, and 5 or more points. Estimated gains
remain in the first band. Loss is the change between consecutive position
estimates, signed for the mover. Partial games contribute only moves with both
estimates available. This is not an accuracy grade or strategic diagnosis.

Engine/network/configuration hashes, visit budget, rules, komi, board size, score
perspective and analysis policy must match within an analysis set. No aggregate
mixes sets. Engine filenames and network names stay out of this presentation.

**Worth revisiting** lists up to 20 highest-loss decisions per set. Click one to
open its exact saved run just before that move, with candidates and **Why not
this?** available. Existing study replacement and copy-on-write behaviour applies.
**Return to report card** refreshes the report. **Analysed games** includes partial
runs; **Awaiting analysis** opens a game in the existing whole-game review.
Analysis starts explicitly there or through **Analyse pending games**.

Generation runs on a worker thread, replays at most 20 games, scans saved headers
once and loads one game's saved results at a time. Header scanning is linear in
saved runs; an indexed catalogue can follow if measurements justify it.

## Validation

Added Rust tests cover duplicate imports, pending/partial/complete coverage,
player-colour filtering, White's loss sign, future imports, ambiguous self-play,
settings separation and whole-game discovery excluding focused runs. Run:

```sh
cargo test --release --workspace personal_report::tests
cargo test --release --workspace game_analysis::tests
```

Local checks cover Rust syntax, QML parsing and an offscreen fixture for rendering,
decision signals, empty results and missing identity. Full Rust compilation and
real-collection acceptance remain to be done on the user's machine.

Manually check coverage, open an example, verify the board is just before your
move, explore a candidate and return. Open a partial game and verify new imports
appear after Refresh. Opening the report must not launch KataGo.

## Batch analysis

**Analyse pending games** queues the unanalysed and partially analysed games in
this report (at most the latest 20 identified games). **Batch settings** exposes
visits per position, rules, and fallback komi. Recorded komi takes precedence;
missing komi uses the visible fallback. Completed games are skipped. Changing
settings does not trigger automatic reanalysis of completed games. Partial runs
with matching engine contents and settings resume; other runs remain saved.

The game list and requests are snapshotted before analysis into a separate
SQLite queue under the personal project's cache directory. New imports do not
silently join a running batch. One game is analysed at a time, with existing
per-position durable checkpoints. Progress shows games completed, current-game
positions, processing rate and estimated time left in the current game.

**Pause batch** finishes stopping the worker, then returns to the report.
**Resume batch** uses the saved queue and its original settings, including after
restarting Bermuda. Opening the report never automatically starts analysis.
Engine file contents are checked before resuming; changed files stop the batch
rather than mixing results. **Clear paused queue** removes only the queue, never
saved analysis, and allows a fresh selection/settings snapshot. Errors stop the
queue and preserve completed positions for retry. Closing Bermuda also leaves
committed checkpoints and the queue available on restart.

A modal progress panel prevents starting competing engine work or replacing the
active workspace while the batch runs. Pause before returning to other tasks.
The batch itself does not load games into or modify the current Study document.
**Back to Home** explicitly returns to the My games home view.

Additional test:

```sh
cargo test --release --workspace personal_batch::tests
```

The test covers queue persistence, source/fallback komi, pause, changed engine
files, a crash between analysis and queue commits, completed-game skipping and
clearing a queue without deleting analysis. It uses saved results and never
launches a real engine. Local QML fixture checks exercise start, modal progress,
pause/resume and a review panel with no controller; no warnings were emitted.
Rust tests and actual engine execution still require the user's build.

## Decision context (29 October follow-up)

Saved analysis is unchanged: new context is derived from existing consecutive
position estimates when the report opens. No repeat batch is required.

The report separates **Changes in winning chances** (drops of at least five
percentage points, ranked by that drop) from **Large point losses** (at least
one point, ranked by estimated score loss). Each list is capped at 20 per analysis
set. All values use the mover's perspective. Point losses made with a pre-move
win estimate at or below 5% are explicitly labelled; this is a display threshold,
not a proof that a human game was lost. There is no automatic monotonicity-based
cutoff and no claim to model the actual opponent's strength.

Review opened from a report identifies your colour, side to move, the recorded
result (when present), the actual played move, and before/after score and win
estimates. It distinguishes the recorded move from the current recommendation.
**Investigate played move** explicitly runs the existing deeper-search workflow
for the recorded move; this does not overwrite the original score graph.

Report move N is translated to position N-1 once. Navigation suppresses
intermediate controller notifications, checks the final board position, and
reports a mismatch instead of silently showing it as the selected decision.
The source entry's move number remains visible with **Return to this decision**.
The screenshot of After 170 does not establish how that position was reached;
the new UI makes both the entry and current position explicit.

The graph has intermediate move-number ticks, shades unanalysed positions and
states the last analysed move. Score remains explicitly Black-positive, while
the decision context uses the mover's perspective. Review's Decisions tab has
separate win-chance and point-loss filters and defaults to your colour when known.

Validation: added Rust assertions for White score/win sign, actual move,
already-low win chances, separate report rankings and colour propagation. An
offscreen QML regression opens move 172 at position 171 with White to play,
exercises intermediate move notifications and return navigation, and checks the
played-move coordinate conversion. Rust compilation remains a user-side check.

## Review navigation and temporary trials (29 October 2026)

On first entry, the report loads the most recent analysed game and, when
available, a decision with a large winning-chance drop. Within the session it
can return to the last reviewed game and position. The board context identifies
the game, date and your colour; the report remains a collection-wide summary.
Continue reviewing opens that game's analysis. If no analysed game is available,
the report does not invent a default analysis.

Why not this shows the accepted trial move immediately. Back to game position
clears the trial preview without leaving review and requests a stop if candidate
analysis is still running. Return to report card and Continue reviewing provide
the route between collection and game review.

Saved user trials are hidden by default. Only the active trial is displayed,
labelled Your trial rather than assigned an engine-candidate letter. Leaving it
removes it from the display, not from saved evidence. Repeating a trial reuses
saved analysis when its visits meet the requested budget.

The user accepted the revised workflow at the end of the session. Offscreen
checks covered trial visibility, cancellation, coordinate conversion, reuse and
default game selection. Full overnight batch completion remains pending.
New imports do not join an existing queue: finish that queue, refresh the report,
then start Analyse pending games for newly added games.
