#!/usr/bin/env python3

"""
Generate Bermuda's runtime player catalogue from:

  data/player_catalogue-sources.json
  data/player_catalogue-curation.json
  an archived u-go.net player database snapshot

The generator never modifies a Bermuda game database.
"""

from __future__ import annotations

import argparse
from collections import defaultdict
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import sys
import tempfile
from typing import Any


SOURCE_KEY = "u-go-player-list"
BERMUDA_KEY_RE = re.compile(r"^bermuda:p[0-9]{6}$")


def fail(message: str) -> None:
    print(f"ERROR: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_json(path: Path, description: str) -> Any:
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"{description} not found: {path}")

    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        fail(f"{description} is not valid JSON: {path}: {exc}")


def load_gzip_json(path: Path) -> Any:
    try:
        with gzip.open(path, "rt", encoding="utf-8") as stream:
            return json.load(stream)
    except FileNotFoundError:
        fail(f"u-go snapshot not found: {path}")
    except (OSError, json.JSONDecodeError) as exc:
        fail(f"cannot read u-go snapshot {path}: {exc}")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()

    try:
        with path.open("rb") as stream:
            for block in iter(
                lambda: stream.read(1024 * 1024),
                b"",
            ):
                digest.update(block)
    except FileNotFoundError:
        fail(f"u-go snapshot not found: {path}")

    return digest.hexdigest()


def find_u_go_source(
    manifest: dict[str, Any],
) -> dict[str, Any]:
    sources = manifest.get("sources")

    if not isinstance(sources, list):
        fail("source manifest 'sources' must be a list")

    matches = [
        source
        for source in sources
        if isinstance(source, dict)
        and source.get("key") == SOURCE_KEY
    ]

    if len(matches) != 1:
        fail(
            f"expected exactly one {SOURCE_KEY!r} source; "
            f"found {len(matches)}"
        )

    return matches[0]


def verify_snapshot(
    snapshot_path: Path,
    source: dict[str, Any],
) -> list[dict[str, Any]]:
    expected = source.get("sha256")

    if (
        not isinstance(expected, str)
        or re.fullmatch(r"[0-9a-f]{64}", expected) is None
    ):
        fail("source manifest contains an invalid SHA-256")

    actual = sha256_file(snapshot_path)

    if actual != expected:
        fail(
            "u-go snapshot SHA-256 mismatch\n"
            f"  expected: {expected}\n"
            f"  actual:   {actual}"
        )

    data = load_gzip_json(snapshot_path)

    if not isinstance(data, list) or not data:
        fail(
            "u-go snapshot must contain a non-empty "
            "top-level player list"
        )

    return data


def index_u_go_players(
    players: list[dict[str, Any]],
) -> dict[int, dict[str, Any]]:
    result: dict[int, dict[str, Any]] = {}

    for player in players:
        if not isinstance(player, dict):
            fail("u-go snapshot contains a malformed player")

        player_id = player.get("id")

        if not isinstance(player_id, int) or player_id <= 0:
            fail(f"u-go player has invalid id {player_id!r}")

        if player_id in result:
            fail(f"duplicate u-go player id {player_id}")

        result[player_id] = player

    return result


def snapshot_names(
    player: dict[str, Any],
) -> list[dict[str, Any]]:
    result = []

    for group in player.get("names", []):
        if not isinstance(group, dict):
            continue

        for entry in group.get("simplenames", []):
            if not isinstance(entry, dict):
                continue

            name = entry.get("name")

            if not isinstance(name, str) or not name:
                continue

            languages = []

            for language in entry.get("languages", []):
                if not isinstance(language, dict):
                    continue

                code = language.get("language")

                if isinstance(code, str) and code:
                    languages.append({
                        "language": code,
                        "preferred": bool(
                            language.get("preferred")
                        ),
                    })

            result.append({
                "name": name,
                "databases": list(
                    entry.get("databases", [])
                ),
                "languages": languages,
                "incorrect": bool(
                    entry.get("incorrect")
                ),
            })

    return result


def validate_curated_names(
    curated: dict[str, Any],
    source_players: list[dict[str, Any]],
) -> None:
    key = curated["key"]
    curated_names = curated.get("names")

    if not isinstance(curated_names, list) or not curated_names:
        fail(f"{key!r} contains no source names")

    available = []

    for source_player in source_players:
        available.extend(snapshot_names(source_player))

    for entry in curated_names:
        if not isinstance(entry, dict):
            fail(f"{key!r} contains a malformed source name")

        name = entry.get("name")

        if not isinstance(name, str) or not name.strip():
            fail(f"{key!r} contains an empty source name")

        expected = {
            "name": name,
            "databases": list(
                entry.get("databases", [])
            ),
            "languages": list(
                entry.get("languages", [])
            ),
            "incorrect": bool(
                entry.get("incorrect")
            ),
        }

        if expected not in available:
            fail(
                f"{key!r} curated name {name!r} "
                "does not match the adopted u-go snapshot"
            )


def make_runtime_player(
    curated: dict[str, Any],
    by_u_go_id: dict[int, dict[str, Any]],
    snapshot_date: str,
) -> dict[str, Any]:
    key = curated.get("key")

    if (
        not isinstance(key, str)
        or BERMUDA_KEY_RE.fullmatch(key) is None
    ):
        fail(f"invalid Bermuda catalogue key {key!r}")

    if curated.get("review_status") != "approved":
        fail(f"{key!r} is not approved")

    preferred_name = curated.get("preferred_name")

    if (
        not isinstance(preferred_name, str)
        or not preferred_name.strip()
    ):
        fail(f"{key!r} has no preferred display name")

    source_record = curated.get("source_record")

    if not isinstance(source_record, dict):
        fail(f"{key!r} has no source_record")

    if source_record.get("source") != SOURCE_KEY:
        fail(
            f"{key!r} references unexpected source "
            f"{source_record.get('source')!r}"
        )

    if source_record.get("snapshot_date") != snapshot_date:
        fail(
            f"{key!r} snapshot date does not match "
            "the source manifest"
        )

    external_ids = curated.get("external_ids")

    if not isinstance(external_ids, dict):
        fail(f"{key!r} has no external_ids")

    u_go_ids = external_ids.get("u_go")

    if (
        not isinstance(u_go_ids, list)
        or not u_go_ids
        or any(
            not isinstance(player_id, int)
            or player_id <= 0
            for player_id in u_go_ids
        )
    ):
        fail(f"{key!r} has invalid u-go ids")

    if len(u_go_ids) != len(set(u_go_ids)):
        fail(f"{key!r} repeats a u-go id")

    missing = [
        player_id
        for player_id in u_go_ids
        if player_id not in by_u_go_id
    ]

    if missing:
        fail(
            f"{key!r} references u-go ids absent "
            f"from the snapshot: {missing}"
        )

    source_players = [
        by_u_go_id[player_id]
        for player_id in u_go_ids
    ]

    recorded_key_name = source_record.get("key_name")

    if recorded_key_name is not None:
        actual_key_names = {
            player.get("key_name")
            for player in source_players
        }

        if recorded_key_name not in actual_key_names:
            fail(
                f"{key!r} recorded u-go key name "
                f"{recorded_key_name!r} does not match "
                "the adopted snapshot"
            )

    curated_wikidata = external_ids.get("wikidata")

    actual_wikidata = {
        player.get("wikidata")
        for player in source_players
        if player.get("wikidata")
    }

    if curated_wikidata is None:
        if actual_wikidata:
            fail(
                f"{key!r} omits Wikidata id present "
                f"in the snapshot: {sorted(actual_wikidata)}"
            )
    else:
        if (
            not isinstance(curated_wikidata, str)
            or not curated_wikidata.strip()
        ):
            fail(f"{key!r} has malformed Wikidata id")

        if actual_wikidata != {curated_wikidata}:
            fail(
                f"{key!r} Wikidata id "
                f"{curated_wikidata!r} does not match "
                f"snapshot values {sorted(actual_wikidata)!r}"
            )

    validate_curated_names(
        curated,
        source_players,
    )

    aliases = set()

    for entry in curated["names"]:
        if bool(entry.get("incorrect")):
            continue

        name = entry["name"]

        if name != preferred_name:
            aliases.add(name)

    return {
        "key": key,
        "preferred_name": preferred_name,
        "aliases": [
            {"name": name}
            for name in sorted(aliases)
        ],
    }


def validate_runtime_catalogue(
    catalogue: dict[str, Any],
) -> dict[str, list[str]]:
    version = catalogue.get("version")

    if not isinstance(version, int) or version <= 0:
        fail(
            "runtime catalogue version must be "
            "greater than zero"
        )

    players = catalogue.get("players")

    if not isinstance(players, list):
        fail("runtime catalogue players must be a list")

    player_keys = set()
    names_to_keys: dict[str, set[str]] = defaultdict(set)

    for player in players:
        key = player.get("key")
        preferred = player.get("preferred_name")
        aliases = player.get("aliases", [])

        if not isinstance(key, str) or not key.strip():
            fail("runtime player key must not be empty")

        if key in player_keys:
            fail(f"duplicate runtime key {key!r}")

        player_keys.add(key)

        if (
            not isinstance(preferred, str)
            or not preferred.strip()
        ):
            fail(
                f"runtime preferred name must not be "
                f"empty for {key!r}"
            )

        names_to_keys[preferred].add(key)

        if not isinstance(aliases, list):
            fail(f"runtime aliases malformed for {key!r}")

        alias_names = set()

        for alias in aliases:
            if not isinstance(alias, dict):
                fail(f"malformed alias for {key!r}")

            name = alias.get("name")

            if (
                not isinstance(name, str)
                or not name.strip()
            ):
                fail(f"empty alias for {key!r}")

            if name in alias_names:
                fail(
                    f"duplicate alias {name!r} "
                    f"for {key!r}"
                )

            alias_names.add(name)
            names_to_keys[name].add(key)

    # The Rust catalogue deliberately permits the same exact spelling
    # to occur for different identities. Such a spelling resolves as
    # ambiguous rather than being guessed.
    return {
        name: sorted(keys)
        for name, keys in names_to_keys.items()
        if len(keys) > 1
    }


def write_atomic_json(
    path: Path,
    data: dict[str, Any],
    force: bool,
) -> None:
    if path.exists() and not force:
        fail(
            f"output already exists: {path}; "
            "use --force to replace it"
        )

    path.parent.mkdir(
        parents=True,
        exist_ok=True,
    )

    encoded = json.dumps(
        data,
        ensure_ascii=False,
        indent=2,
    ) + "\n"

    # Verify our own generated JSON before touching the output.
    json.loads(encoded)

    temporary_path = None

    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as stream:
            temporary_path = Path(stream.name)
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())

        os.replace(temporary_path, path)
        temporary_path = None

    finally:
        if temporary_path is not None:
            temporary_path.unlink(missing_ok=True)


def parse_arguments() -> argparse.Namespace:
    repo_root = Path(__file__).resolve().parent.parent

    parser = argparse.ArgumentParser(
        description=(
            "Generate Bermuda's runtime player catalogue"
        )
    )

    parser.add_argument(
        "--manifest",
        type=Path,
        default=(
            repo_root
            / "data/player_catalogue-sources.json"
        ),
    )

    parser.add_argument(
        "--curation",
        type=Path,
        default=(
            repo_root
            / "data/player_catalogue-curation.json"
        ),
    )

    parser.add_argument(
        "--snapshot",
        type=Path,
        required=True,
    )

    parser.add_argument(
        "--output",
        type=Path,
        required=True,
    )

    parser.add_argument(
        "--force",
        action="store_true",
    )

    return parser.parse_args()


def main() -> None:
    args = parse_arguments()

    manifest = load_json(
        args.manifest,
        "player catalogue source manifest",
    )

    curation = load_json(
        args.curation,
        "player catalogue curation",
    )

    if not isinstance(manifest, dict):
        fail("source manifest must be a JSON object")

    if not isinstance(curation, dict):
        fail("curation must be a JSON object")

    manifest_version = manifest.get("version")

    if (
        not isinstance(manifest_version, int)
        or manifest_version <= 0
    ):
        fail("invalid source manifest version")

    if (
        curation.get("source_manifest_version")
        != manifest_version
    ):
        fail(
            "curation source_manifest_version does not "
            "match the source manifest"
        )

    catalogue_version = curation.get("catalogue_version")

    if (
        not isinstance(catalogue_version, int)
        or catalogue_version <= 0
    ):
        fail("invalid catalogue_version in curation")

    source = find_u_go_source(manifest)

    snapshot_date = source.get("snapshot_date")

    if (
        not isinstance(snapshot_date, str)
        or not snapshot_date
    ):
        fail("u-go source has no snapshot_date")

    u_go_players = verify_snapshot(
        args.snapshot,
        source,
    )

    by_u_go_id = index_u_go_players(
        u_go_players
    )

    curated_players = curation.get("players")

    if (
        not isinstance(curated_players, list)
        or not curated_players
    ):
        fail("curation contains no players")

    runtime_players = []
    curated_keys = set()

    for curated in curated_players:
        if not isinstance(curated, dict):
            fail(
                "curation contains a malformed player"
            )

        key = curated.get("key")

        if key in curated_keys:
            fail(
                f"duplicate curated Bermuda key {key!r}"
            )

        curated_keys.add(key)

        runtime_players.append(
            make_runtime_player(
                curated,
                by_u_go_id,
                snapshot_date,
            )
        )

    runtime_players.sort(
        key=lambda player: player["key"]
    )

    catalogue = {
        "version": catalogue_version,
        "players": runtime_players,
    }

    ambiguities = validate_runtime_catalogue(
        catalogue
    )

    write_atomic_json(
        args.output,
        catalogue,
        args.force,
    )

    alias_count = sum(
        len(player["aliases"])
        for player in runtime_players
    )

    print(
        "===== BERMUDA PLAYER CATALOGUE GENERATED ====="
    )
    print(f"source: {source.get('name')}")
    print(f"snapshot date: {snapshot_date}")
    print(
        f"snapshot SHA-256: {source.get('sha256')}"
    )
    print(
        f"catalogue version: {catalogue_version}"
    )
    print(f"players: {len(runtime_players)}")
    print(f"aliases: {alias_count}")
    print(
        "ambiguous exact names:",
        len(ambiguities),
    )
    print(f"output: {args.output}")

    if ambiguities:
        print()
        print(
            "===== REVIEWABLE EXACT-NAME AMBIGUITIES ====="
        )

        for name in sorted(ambiguities):
            print(
                f"{name}: "
                + ", ".join(ambiguities[name])
            )


if __name__ == "__main__":
    main()
