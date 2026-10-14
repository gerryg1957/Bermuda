# Database operation progress and recovery

The database operation dialog reports SGF discovery, import, game indexing and
search-index finalisation separately. Discovery has no known total yet.

Import and game-indexing estimates use recent processing speed, after at least
five seconds of observations in that phase. They are approximate and apply only
to the current phase. Import discovery time is excluded from the estimate.
Finalisation has an activity indicator and no invented time estimate: finishing
the game count does not mean that the search index is ready.

## Identifying and opening databases

Beside the Study tab, the workspace header shows the open professional database's name and total
unique-game count, independent of filters and paging. Hover over the name to
see the folder path. The count is read in the catalogue worker, without loading
all games into memory. Loading and opening failures are labelled explicitly.

**Database → Open default database** opens the managed Bermuda database, falling
back to the legacy MoyoDB location. If neither exists it explains how to select
or create a database; it does not silently create one.

**Database → Open database…** lists valid databases beside the managed and legacy
default databases, plus up to 30 recently opened locations. It shows each name,
folder and unfinished-setup status. Select a database with one click; use
**Browse…** for a database elsewhere. Deleted folders are omitted. Discovery
checks only these locations, without recursively scanning SGF collections.
The chooser does not change which database is opened at startup.

## Setup and continuation

The progress dialog names the database and shows its location. Import and
search preparation are separate steps, followed by finalisation. Each estimate
covers only the current phase.

New operations save their source folder, source name, version and indexing
choice in `database-task.sqlite3` in the project folder. After cancellation,
**Continue setup** uses those settings. Reopening that database after restarting
Bermuda shows an unfinished-setup notice with the same action beside the database
name. Completing import
advances the saved task to search preparation; completing the operation clears
it. A new operation cannot silently overwrite an unfinished task.

Import continuation re-scans the source folder and recognises previously
imported files; index continuation skips committed games. This is continuation
of durable work, not an in-memory pause. Keep the source folder available at
its saved location. Tasks interrupted before this feature was installed have no
saved instructions and must still be continued manually.

**Create Another Database** defaults to the parent of Bermuda's managed games
database. **Choose another location…** remains available. Existing databases
are not moved.

## Source files and interrupted imports

Discovery and import exclude macOS archive metadata: `__MACOSX` directories
and names beginning with `._`. These are not counted as source SGFs, duplicates
or import errors. Other malformed SGFs still produce errors.

After cancelling an import, use **Database → Add Games** in the same database
with the same source folder, source name and version. Already imported files
are recognised; this re-scans the folder rather than resuming a saved cursor.
Select **Update the position index after importing** to index the games after
the import finishes. Update Position Index alone does not import missing games.

## Cancelling a position-index build

Cancel stops between games, or interrupts SQLite while it builds the final
search indexes. Completed game transactions remain committed. A cancelled
CREATE INDEX is rolled back by SQLite; a secondary index already completed by
an earlier statement remains valid.

To continue, open the database and choose **Database → Update Position Index**.
Previously indexed games are skipped. If all games were committed but
finalisation was interrupted, only the remaining search-index creation is
needed. Keep the database and source SGFs; do not delete them to resume work.

This stores one unfinished task per database, not a background task queue or
a Pause/Resume button. There is no new safe-start mode in this change, and the
reported Windows startup memory problem has not been diagnosed or fixed by it.
Do not use a large affected database as the first test of this change.

## Remaining work

- Inspect and bound memory use during indexing and startup.
- Offer safe startup and recovery when the previous database cannot open.
- Add a Pause/Resume control and coordinated safe application exit.
- Preserve checkpoints in the separate binary pattern-index builder (it currently
  discards cancelled temporary builds).
- Add download rate/ETA and validated partial-download resumption.

## Development checks

Core tests exercise cancellation between games, cancellation at finalisation,
actual SQLite interruption during CREATE INDEX, successful reuse of the
connection after interruption, and equivalence of resumed and uninterrupted
position indexes. Estimate tests cover warm-up, phase timing, recent-rate
adaptation and resets.
