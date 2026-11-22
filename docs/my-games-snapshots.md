# My Games: fixed groups and preserved reports (4A)

Open **My games → Report card → Reports…** for the reports library, creation
controls and an in-app trend graph. Existing snapshots remain available here.

The live report has a **Games in live report** control: choose a count and click
**Apply**, or use **0** for all identified games. The choice is remembered. A new
analysis batch uses the same selection; resuming a paused batch retains its
original queue. The selection of games from an import still takes precedence.
Twenty is only the initial live-report default, not a required sample size.

## Workflow

1. Open **My games → Report card → Reports… → Choose games**.
2. Give the group a name, for example `October 2026`.
3. Select a user-chosen number of the most recent shown games, tick individual games, or enter an inclusive date range and click **Use shown
   games**. This replaces the selection with the displayed games. Changing the
   date filter alone does not change the selection; its total includes hidden
   selections. Clear the dates to include undated or partially dated records.
4. **Keep these games together →** preserves membership, names, your playing colour and copies of
   the main lines. New imports do not join the group. Unidentified or ambiguous
   games can be included, but are visibly excluded from personal statistics.
   Fix name mappings before saving; to change membership or identity details,
   create a new group. Overlapping groups are allowed.
5. Under **New report**, enter a report name such as `CPU baseline — 200
   visits`, then **Save new report**. This copies each game's latest available
   whole-game run. It does not run KataGo or replace an earlier snapshot.
6. **Reports library** lists every saved report, across all groups. Click one to
   open it. **View report / Print…** under New report also opens a self-contained local HTML report in the default
   browser. Its **Print / Save as PDF** button uses the browser's print dialog.
   The visual summary is printed by default: analysis coverage, move-cost bar
   charts with percentages and counts, and up to three positions from distinct
   games per compatible analysis set. **Detailed tables** reveals the longer
   membership/decision lists. **Game pages and engine details** reveals score
   graphs, individual-game decisions and provenance. These choices also control
   what is included in the printout.

The diagram ring marks the move actually played, not an engine recommendation.
SGF and complete analysis JSON downloads on the game pages provide the preserved
main line and evidence, including candidates and deeper investigations. The
browser report is read-only; it does not replace Bermuda's interactive live
review or start analysis. The **Saved comparisons** tab compares preserved move-cost distributions. Interpretation of
recurring patterns remains later work.

Run new/deeper analysis through the existing analysis workflow, then explicitly
save another snapshot for the same group. The new report can have different
coverage, engine or settings; the old report retains its original evidence.
Partial snapshots remain partial. Different engine/network/configuration
identities, visit budgets, rules, komi and board sizes are separate analysis sets.
A newer incomplete run is labelled partial; it is not silently substituted with
an older complete run.

## Storage and retention

`database/personal-snapshots.sqlite3` belongs to the personal project. It is not
in the cache or the professional database. Group and snapshot APIs only append;
SQL triggers also reject updates. Removal controls hide reports and empty groups from the library using persistent removal records. Immutable evidence remains internally stored; removal is not a disk purge. Exported HTML/PDF copies are unaffected.
Back up the personal project to preserve these records. They intentionally use
additional disk space: freezing a reference to a mutable checkpoint is not enough.

A group stores canonical game IDs for traceability plus its own SGF main-line
copies, display names and identified playing colours. These are sufficient to
replay the saved positions after source deletion; original source comments and
variations not retained by My Games are not reconstructed.

A snapshot contains the entire group, complete copies of the selected analysis
runs (history, results, candidates, refinements and provenance), a frozen summary
and frozen personal-decision rows. Runs are read from one SQLite read transaction
so an active writer cannot mix different versions during capture. Snapshots do
not depend on an installed engine, mutable latest-run lookup, later name mappings
or surviving source-game files when viewed or printed.

Removing source games or clearing live analysis does not remove old groups or
snapshots. A new snapshot can still find analysis by the saved game history after
a source is removed; if the analysis itself is gone, the new snapshot records it
as awaiting analysis. Old snapshots retain their copied results.

HTML exports in `cache/snapshot-reports` are disposable derived files, regenerated
from the chosen snapshot. They require no network resources. Exported user text
is escaped, and game/analysis downloads are encoded. Keep an HTML/PDF copy for
sharing or reading independently of Bermuda; keep the project backup for the
original groups and evidence. Browser print options control paper size and PDF
output; the report supplies an A4 layout.

## Acceptance checks

Core regression tests cover fixed membership, duplicate/empty/missing selection,
unknown identities, overlapping groups, partial-to-complete analysis, changed
engine/visit settings, source deletion, analysis cleanup, restart, HTML escaping,
and SGF board orientation. The existing report and analysis tests cover the
shared summary and checkpoint reader.

On the real installation:

- Save a group containing an older game beyond the recent 20 and a pending game.
- Save a partial/baseline snapshot; check its counts, colours, dates and settings.
- Import a new game and finish or deepen analysis. Save a second snapshot.
- Restart Bermuda. Check that the first snapshot and group are unchanged and the
  second report is separately selectable.
- Print the summary to PDF, then include the game pages and check their boards
  against interactive review. Try a name containing an apostrophe or ampersand.

Do not remove real games or analysis merely to test retention; automated tests
exercise those cases on temporary projects.

## Understanding the visual report

The group answers “which games am I keeping together?” The snapshot answers
“what analysis am I preserving from now?” The numbered steps explain these
separately. Saving either one does not run KataGo.

Coverage uses all games in the fixed group as its denominator. Move-cost bars
use assessed personal decisions in that compatible settings set. They use a
fixed 0–100% scale, retain estimated gains in the below-one-point category, and
show counts alongside percentages. Unidentified games are visible in coverage
but excluded from personal statistics; zero assessed moves produces an explicit
empty state rather than invented percentages.

Position cards favour distinct games, choosing from the largest saved drops in
winning chance. When no such drops were recorded, saved point-loss examples are
shown instead and labelled as such. Cards identify the game, date, colour,
recorded result, move played, and before/after estimates. They are prompts for
investigation, not a diagnosis of recurring weaknesses.

Reopen an existing snapshot to get the visual presentation; its stored data is
unchanged. Compare different groups under comparable analysis to investigate
changes in play. A deeper new snapshot of the same games changes the evidence,
not the player's historical performance. The first distribution trend view is described below; richer comparisons remain 4D.


## Distribution trends (18 November 2026)

In **Reports → Saved comparisons**, select saved reports from different groups. Initially
one report per group is selected (the newest). Selecting another report for the
same group replaces its selection: a deeper analysis of the same games is not
another period of play. Older reports are not changed or deleted.

Choose a loss category and an analysis settings set. The graph plots the
percentage of assessed personal moves in that category, using the exact frozen
counts from each report's bars. It does not average percentages per game or
rescore the archived decisions. It works with reports saved before this update.
Points use each set's last game date, ordered chronologically; the full date
range, games, complete games and assessed moves appear below. The horizontal
axis is elapsed calendar time, not report creation order.

Only matching engine, network, configuration, scoring policy, score perspective,
visits, rules, komi and board size are combined. Thus CPU and graphics-card
baselines may be separate sets. Missing/unidentified games do not contribute
personal moves; partial games contribute only assessed moves. Reports with
unknown/partial game dates, no assessed moves or no matching settings do not
produce points. The underlying report can always be opened from the library.
Shared games are flagged and the connecting line is suppressed. Every plotted
point has an Open this report button for the underlying evidence.

More moves below one estimated point of loss, or fewer at five-plus, can be a
useful improvement signal. Changes in opponent strength, handicap, colour and
coverage can also alter those proportions; this is not a playing-strength grade
or proof of improvement. There is no claim that twenty, fifty or any other fixed
number guarantees a reliable trend. Use meaningful dated groups and inspect
coverage. Creating another snapshot does not run KataGo.

Acceptance: reopen an old report; save a second named report without replacing
the first; create two disjoint dated groups at a chosen size; compare their bars
and trend percentages; check separate settings, partial evidence and shared-game
warnings. Automated tests cover preserved counts, settings identity, game-count
selection and the graph's normalisation/ordering/overlap logic.


## Live trends and removal (18 November 2026)

**Reports → Trends** reads the latest saved whole-game analysis for all identified
My games, independently of the report-card game limit and saved reports. Opening
Reports refreshes this data; use **Refresh analysis** to pick up further analysis
while the dialog stays open. This never launches KataGo or schedules reports.

Select 1, 3 or 6 months, 1 year, All, or an inclusive custom date range. Relative
periods end today. Show each contributing game, or group by calendar week (Monday start),
calendar month, or a chosen number of consecutive contributing games. Blocks
restart at the selected range and may end in a smaller final block.
Weekly/monthly points mark the period start; game blocks use the last game date.
The game list under each point makes its membership inspectable.

Percentages divide the selected loss bucket's move count by assessed personal
moves, not by games. Partial games contribute only assessed moves. The engine,
network, configuration, visits, rules and board size remain separate. Komi
values are combined in the live view; each loss retains its original analysis
at that game's komi. Other
settings and pending games are counted but do not enter the selected denominator.
Undated games are omitted explicitly. Empty periods are not zero; lines break
across missing evidence or skipped calendar periods. These are descriptive
trends, affected by opponents, colour and handicap, not a playing-strength grade.
Later analysis or identity edits can change this live view; preserved reports
and **Saved comparisons** remain fixed reference points.

The New report tab labels the **Game group** and its saved-report count. An
unwanted group such as Q4 2026 with no saved reports can be removed there.
Remove reports individually from Reports library or the group's report list;
remove all its reports before removing a group. Both operations require an
explicit confirmation, persist across restarts, and retain source games and
saved KataGo analysis. There is currently no restore control.

Validation: core personal-analysis tests include per-game bucket accounting,
removal guards, persistence and non-reused IDs. JavaScript tests cover weighted
percentages, date ranges, mixed settings, missing evidence and game blocks.
All five tabs instantiate in an offscreen Qt harness; desktop acceptance remains
required with the user's actual collection.

### All categories

Both Trends and Saved comparisons offer **All categories** alongside the four
individual loss categories. The chart draws four lines with the same teal, blue,
amber and red colours as the printed bar charts, plus distinct line styles and
a legend. Period/report entries show all four percentages. Matching settings,
move-weighted percentages and missing-evidence gaps are unchanged.

### Clearer live timeline and komi pooling

Live Trends now defaults to Each game and the highest available visit budget.
The analysis selector shows depth, rules, board size and the number of dated
games in the selected range that have assessed personal moves. Compatible
settings differing only in komi are combined automatically; no reanalysis is
performed. Older preserved reports and their Saved comparisons retain their
original grouping.

Game dates preserves elapsed calendar time. Game order spaces contributing
games equally; same-date ties use canonical game ID rather than inventing
unknown playing times. Weekly and monthly views retain gaps in evidence, while
the timeline trims leading/trailing bins with no contributing moves. Counters
separate contributing games from all games in range, pending analysis, other
settings, unassessed personal moves and undated games.

Verified by JavaScript tests for komi pooling, provenance separation, weighted
percentages, date filtering, same-date game ordering and exclusion counts, plus
offscreen UI switching through grouping and axis choices.

### Weekly report library (20 November 2026)

In **Reports library**, choose **Create weekly reports…**, then **Preview weekly
reports**. The preview lists consecutive seven-day periods from the earliest
fully dated game in My games through today, using your local calendar date.
The first period includes the starting date and the following six days. There
is no twenty-game limit: every game in each interval is included.

**Replace library with weekly reports…** asks for confirmation showing the
number of reports and groups being replaced. It removes all existing reports
and groups from the visible library, then creates one preserved report per
period, including empty weeks. Games and saved KataGo analysis are read only;
no engine run is started. Replacement is transactional: any error restores the
original library. Removed entries remain internal immutable evidence, as with
individual report removal; previously exported HTML/PDF files are not deleted.

The current period is labelled unfinished, including its last day. Empty weeks
are labelled as gaps, never as zero move-quality scores. Partial and missing
analysis remain visible in coverage counts. Incomplete or invalid dates and
future-dated games are omitted and counted in the preview. Games whose personal
colour is unknown remain in their period but are excluded from personal statistics.

Once created, the starting date is remembered across restarts. Importing an
older game does not shift existing boundaries: the preview counts games before
the anchor. Select **Reset the starting date to my earliest game** to explicitly
rebuild from that older date. Preview again after imports or library changes.
To refresh the weekly reports after new games or analysis, repeat this workflow;
it replaces the library again rather than silently changing saved reports.

Weekly report charts combine different komi values, using each game's own
analysis. Engine, network, configuration, rules, board size and visits still
separate incompatible evidence. The existing custom-report workflow retains
its original grouping.

Live Trends defaults to Weekly once a weekly starting date exists and uses the
same seven-day boundaries. It reads current analysis, so a newly analysed game
can change the live graph without changing a previously saved report. A selected
date range may show only part of a boundary week. Other trend groupings remain
available.

Validation: Rust tests cover boundaries and leap days, empty/unfinished periods,
source-analysis preservation, stale previews, rollback, anchor persistence and
explicit reset preview, plus komi pooling. JavaScript tests cover anchored trend
boundaries and gaps. All six tabs load in an offscreen QML harness with a mock
model. Full Qt application compilation and desktop acceptance remain to be done
on the user's system.

### Weekly trend axes

Saved comparisons now place weekly reports at their fixed period start, matching
Live Trends. Custom reports continue to use their last game date. Date ticks and
vertical guides are shown throughout the axis; labels thin out on long ranges.
Missing or unassessed weekly evidence creates a gap, never a zero value or a
line bridging missing weeks. Live weekly views include empty intervening weeks.

Hollow circles identify the current unfinished live week, or a saved report
whose period was unfinished when preserved. Saved status remains unchanged
until reports are rebuilt. Entries below both charts show date ranges, game
counts and analysis coverage; saved entries also distinguish total period games
from games contributing to the chosen analysis settings.

Validated with calculation tests and offscreen QML rendering of both graphs
with analysed, empty and unfinished weeks, including All categories.

### All analysis

Both trend views now offer **All** in the analysis selector alongside the
individual settings. This opt-in view combines the older CPU/200-visit evidence
and newer GPU/1,000-visit evidence without reanalysis or changing reports. Live
Trends uses the latest saved run per game; Saved comparisons uses the selected
preserved reports. Percentages use summed loss-bucket counts divided by assessed
moves, rather than averaging percentages from differently sized groups.
The same weekly boundaries, gaps and unfinished markers apply. The option is
labelled simply All, without an additional mixed-analysis warning.

Calculation tests cover mixed depths/engines, move weighting, pending games and
invalid/duplicate snapshot evidence. Both graphs render offscreen with All
selected; desktop acceptance remains pending.

### PDF filenames

Exported report pages now have descriptive browser titles. Weekly reports use
Bermuda - Weekly report - YYYY-MM-DD to YYYY-MM-DD, with an unfinished suffix when
appropriate. Custom reports use the group and report names. This supplies a
meaningful suggested filename when the browser prints to PDF.

Reopen an existing report using Bermuda's **Open report…** button to regenerate
its HTML with the new title. Saved analysis and report evidence are unchanged;
already exported PDFs are not renamed.

### Direct PDF export

Each report in the library has **Save PDF…**, with its date range or custom
report name suggested as the filename. Choose a destination to save an A4 PDF
summary directly from Bermuda, including coverage, loss-category charts and the
same selected board positions as the browser summary. This uses the preserved
snapshot; it does not run KataGo or alter saved analysis. The destination is
replaced only after a successful export.

**Open report…** still opens the browser presentation, where detailed tables and
individual game pages can be included before printing or saving a PDF.
