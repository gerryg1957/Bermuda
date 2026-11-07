# My games: player names and Me

29 October 2026 — development increment after the 0.8.3 release.

## Using it

Open Home → My games → Manage names…, or Database → Manage My Games Names….

The main screen asks **Which player is you?**

1. Leave **Show my names as** set to **Me**, or choose another label. An existing
   saved label is retained until you explicitly change it.
2. Tick your usernames. Each row shows the number of distinct games and its
   import source. Already-linked names are checked and marked **Already yours**.
3. Click **Review names**, check the list and display label, then **Save my names**.

Saving creates or updates the personal identity and marks it as Me automatically.
The label, new aliases and Me selection are saved together in one transaction.
A conflict rolls back the whole batch. Existing links are kept, including names
not currently appearing in the list. No changes are written during review.

**Advanced name management…** opens the previous editor for opponent identities,
removing links, switching Me, or dealing with a name already linked elsewhere.
Selecting a new source name there still requires a fresh player selection before
linking. **Back to my names** returns to the simple form with saved settings.
Unsaved simple-form selections are discarded when opening the advanced editor.

The catalogue refreshes after edits. Searching a linked username or preferred
name uses the existing identity-aware search. Source spellings remain intact.
Links apply to exact names within the selected import source and version,
including future imports. They do not guess identities or distinguish two
people using the same spelling in that same source. The single-SGF import path
currently groups imports under Imported personal games; it does not infer an
online server from a username.

The professional database remains separate. Its existing Player Names editor
keeps its current behaviour and does not show Me controls. This increment does
not rewrite names displayed in an already-open external SGF or saved study.

## Storage and compatibility

Existing player and alias tables hold the mappings. Setting Me adds an optional
`personal_player` singleton table in that database, in a transaction. It refers
to `players.id`, with ON DELETE CASCADE. Rename preserves the identity; deleting
it clears Me, including when deletion occurs through an older application.
Absent settings mean no Me has been selected. Clearing Me preserves the aliases.

This is an additive extension to schema 8, with no alteration to existing
columns, indices, game records, or schema version. Bermuda 0.8.3 can still open
and use the database, but does not expose the Me setting. No cross-database or
cross-machine settings are used. Standard database backups include Me.

## Validation

Added Rust regression coverage for persistence, replacement, clearing, rename,
identity deletion, invalid IDs, database isolation, compatibility with schema 8,
both colours, future import resolution, identity-aware search and raw-name
preservation when unlinking.

QML parsing and an offscreen dialog test with a mock model check selection,
Mark as me, Clear Me, professional mode and preventing stale edits after an
unsuccessful project open. Rust syntax was checked locally; the full Rust/Qt
build and regression tests require the maintainer's toolchain. Acceptance on a
real personal collection remains pending.

The simplified-form follow-up adds atomic-save and stale-selection regression
coverage. An offscreen mock-model check exercises the Me default, checkbox
payload, review-before-save, saved state and advanced/professional routes. Rust
tests still require execution on the maintainer's toolchain.

## Adding SGF files directly (3 November 2026)

Use **Home → My games → Add games…** and select one or several SGF files.
The library imports their recorded main lines without replacing the current
study. Existing canonical games are counted as already present, including files
previously added through the study workspace. Existing personal-name mappings
are reused. Unknown or ambiguous playing identities are reported with a
**Manage names…** action; these games remain in the library.

The result shows added/duplicate counts and individual file issues. A malformed
file does not discard other successful imports. Importing runs off the UI thread;
**Stop after this file** keeps completed imports. Position-only SGFs and
multi-game SGF collections are rejected explicitly. An indexing failure is
reported even if the game was successfully saved; reimporting retries indexing.

**Done** stays in the library. Imported games are highlighted; catalogue filters
are cleared and games sorted newest first. **Analysis options…** opens a report
restricted to these imported game IDs, including older games outside the normal
latest-20 window. Nothing starts automatically. **Analyse imported games** then
analyses only identified pending/partial games in that selection, using the
visible settings. Completed analysis is skipped.

An existing saved queue is never overwritten or silently extended. Resume runs
that original queue. To analyse the import instead, explicitly clear the paused
queue (saved results remain), then start the selected import batch. Returning to
the ordinary Report card restores its usual latest-20 scope.

**Open → SGF** remains the route for opening a file for study without importing it.

Verification: core tests cover duplicates, mixed success/failure, cancellation,
identity reuse, explicit older-game selection, and selected queue creation.
Local delivery checks include QML parsing and a mocked report-panel selection/
start/pause check; a full Rust/Qt build must be run on the development machine.

## Recorded ranks (7 November 2026)

The library shows each player's recorded SGF rank beside their name, including
a mapped name such as Me. The review heading shows both ranks. These are BR/WR
values from that game, not a current rating or a strength inferred from KataGo.
Missing values stay blank; rank systems from different servers are not normalised.

New imports retain ranks as optional source metadata. Older imports recover them
from readable original SGFs only when their canonical game hash still matches.
For personal imports whose source locator is a hash, use Add games again with the
original SGFs: missing ranks are filled without duplicating games or rerunning
analysis. Previously preserved analysis snapshots remain unchanged.

The optional database columns are added transactionally without changing schema
version 8 or the compact move-file format. Old 0.8.3 builds can still open it.
