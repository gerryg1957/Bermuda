# Bermuda supplied player catalogue

## Purpose

Bermuda should normally know established player identities before the user
starts research.

A user searching for a player by a name or romanisation that they know should
not also have to know the other spellings used by the imported game sources.

The manual Player Identities interface is therefore an exception and
correction mechanism, not a catalogue-building task for each user.

## Principles

1. Imported PB/PW text remains unchanged and authoritative as source evidence.
2. Bermuda may associate that text with a separate player identity.
3. Bermuda ships with a versioned curated player catalogue.
4. User additions and corrections are stored separately from supplied data.
5. Explicit user mappings take precedence over Bermuda-supplied mappings.
6. Ambiguous names remain unresolved rather than being guessed.
7. Catalogue updates may reconsider Bermuda's own automatic assignments but
   must not silently overwrite user-curated assignments.
8. A database does not need to materialise every player known to the supplied
   catalogue.

## Storage model

The supplied catalogue is maintained as versioned data in the Bermuda source
tree and compiled into the application. A likely source representation is:

    data/player_catalogue.json

On opening a project, Bermuda synchronises that data into dedicated tables in
the project's existing metadata.sqlite3 database.

Conceptually:

    player_catalogue_state
    player_catalogue_players
    player_catalogue_aliases

contain Bermuda-supplied knowledge.

The existing:

    players
    player_aliases

remain the local/materialised identity layer and the user's curated knowledge.

The supplied catalogue is not a second runtime SQLite database.

## Stable identities

Every supplied player has a Bermuda-owned stable catalogue key independent of
the player's preferred display name.

Names are data, not identifiers: preferred spellings and romanisations may
change while the catalogue identity remains stable.

A materialised row in `players` may therefore refer to a supplied catalogue
key. User-only identities have no catalogue key.

## Resolution precedence

Import-time resolution should use this order:

1. exact user source-specific mapping;
2. exact user global mapping;
3. exact Bermuda catalogue mapping;
4. unresolved.

At every stage an ambiguous mapping remains unresolved.

This extends the existing PlayerDirectory rule that source-specific local
aliases outrank global aliases.

## Assignment provenance

Once Bermuda can assign identities automatically from supplied catalogue data,
`game_metadata` must distinguish the origin of the numeric player link.

For each side the effective states are:

    unresolved
    local
    catalogue

Existing non-null player links created before the supplied catalogue are local
user-curated links.

A catalogue refresh may clear and recompute catalogue-derived links. It must
not clear or replace local links.

## Materialisation

Loading the supplied catalogue does not create a row in `players` for every
known person.

A catalogue identity is materialised locally when it is needed, principally
when:

- an imported source name resolves to it; or
- the user adds local information to it.

This keeps the local identity list relevant to the user's game collection.

## Search semantics

Searching by any unambiguous supplied preferred name or alias resolves to the
same materialised player identity.

Consequently, a user may search using the spelling they know without needing
to know which spelling Bermuda uses for display or which spelling appears in a
particular SGF source.

Raw unlinked names remain searchable exactly as before.

## User corrections

Local mappings outrank supplied mappings.

The first catalogue implementation should preserve the existing ability to add
local identities and aliases. Later UI work may additionally expose explicit
suppression or replacement of an incorrect supplied mapping.

Such corrections must survive supplied catalogue updates.

## Unrecognised names

After supplied-catalogue reconciliation, the current list of unlinked source
spellings should be understood as unrecognised source names.

They are not a task list that the user is expected to complete. They are names
for which Bermuda currently has no safe identity assertion.

## Catalogue updates

The catalogue has an independent data version.

When a newer supplied catalogue is loaded:

1. replace/synchronise only Bermuda-owned catalogue rows;
2. preserve all local user data;
3. preserve all local player links;
4. clear and recompute only links recorded as catalogue-derived;
5. leave ambiguous or no-longer-recognised names unresolved.

## Initial implementation boundary

The first implementation should prove the architecture with a very small,
explicitly curated fixture catalogue before attempting broad population.

Only after import, reconciliation, search, update and local-override behaviour
are covered by tests should the project ingest a large real catalogue.

The source and licensing of the production catalogue are a separate data
curation decision and must be established before bulk population.
