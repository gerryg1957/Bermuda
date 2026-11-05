# My Games: fixed groups and preserved reports (4A)

The first 4A implementation adds **Groups and snapshots…** to the report card.
The existing recent-20 report remains a live view. Twenty is not a limit on
saved groups, or a prescribed sample size for measuring improvement.

## Workflow

1. Open **My games → Report card → Groups and snapshots… → 1. Choose games**.
2. Give the group a name, for example `October 2026`.
3. Tick individual games, or enter an inclusive date range and click **Use shown
   games**. This replaces the selection with the displayed games. Changing the
   date filter alone does not change the selection; its total includes hidden
   selections. Clear the dates to include undated or partially dated records.
4. **Keep these games together →** preserves membership, names, your playing colour and copies of
   the main lines. New imports do not join the group. Unidentified or ambiguous
   games can be included, but are visibly excluded from personal statistics.
   Fix name mappings before saving; to change membership or identity details,
   create a new group. Overlapping groups are allowed.
5. Under **Saved groups and reports**, enter a report name such as `CPU baseline — 200
   visits`, then **Preserve analysis**. This copies each game's latest available
   whole-game run. It does not run KataGo or replace an earlier snapshot.
6. **View report / Print…** opens a self-contained local HTML report in the default
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
review or start analysis. Automatic group comparisons and interpretation of
recurring patterns remain later work.

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
SQL triggers also reject updates. No deletion UI is supplied in this increment.
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
not the player's historical performance. Automatic comparisons remain 4D.
