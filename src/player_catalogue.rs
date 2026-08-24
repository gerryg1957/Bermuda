use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

const SUPPLIED_CATALOGUE_JSON: &str = include_str!("../data/player_catalogue.json");

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlayerCatalogue {
    pub version: u64,
    pub players: Vec<CataloguePlayer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CataloguePlayer {
    pub key: String,
    pub preferred_name: String,

    #[serde(default)]
    pub aliases: Vec<CatalogueAlias>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CatalogueAlias {
    pub name: String,

    #[serde(default)]
    pub notes: Option<String>,
}

impl PlayerCatalogue {
    pub fn supplied() -> Result<Self> {
        Self::from_json(SUPPLIED_CATALOGUE_JSON)
            .context("parsing Bermuda supplied player catalogue")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let catalogue: Self =
            serde_json::from_str(json).context("parsing player catalogue JSON")?;

        catalogue.validate()?;

        Ok(catalogue)
    }

    fn validate(&self) -> Result<()> {
        if self.version == 0 {
            bail!("player catalogue version must be greater than zero");
        }

        let mut player_keys = HashSet::new();

        for player in &self.players {
            if player.key.trim().is_empty() {
                bail!("player catalogue key must not be empty");
            }

            if player.preferred_name.trim().is_empty() {
                bail!(
                    "player catalogue preferred name must not be empty for key {:?}",
                    player.key
                );
            }

            if !player_keys.insert(player.key.as_str()) {
                bail!("duplicate player catalogue key {:?}", player.key);
            }

            let mut alias_names = HashSet::new();

            for alias in &player.aliases {
                if alias.name.trim().is_empty() {
                    bail!(
                        "player catalogue alias must not be empty for key {:?}",
                        player.key
                    );
                }

                if !alias_names.insert(alias.name.as_str()) {
                    bail!(
                        "duplicate player catalogue alias {:?} for key {:?}",
                        alias.name,
                        player.key
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supplied_catalogue_parses() -> Result<()> {
        let catalogue = PlayerCatalogue::supplied()?;

        assert_eq!(catalogue.version, 1);
        assert!(catalogue.players.is_empty());

        Ok(())
    }

    #[test]
    fn parses_player_aliases_and_optional_notes() -> Result<()> {
        let catalogue = PlayerCatalogue::from_json(
            r#"
            {
              "version": 7,
              "players": [
                {
                  "key": "test:player-a",
                  "preferred_name": "Player A",
                  "aliases": [
                    {
                      "name": "Alias A1"
                    },
                    {
                      "name": "Alias A2",
                      "notes": "test provenance note"
                    }
                  ]
                },
                {
                  "key": "test:player-b",
                  "preferred_name": "Player B"
                }
              ]
            }
            "#,
        )?;

        assert_eq!(catalogue.version, 7);
        assert_eq!(catalogue.players.len(), 2);

        let player_a = &catalogue.players[0];
        assert_eq!(player_a.key, "test:player-a");
        assert_eq!(player_a.preferred_name, "Player A");
        assert_eq!(player_a.aliases.len(), 2);

        assert_eq!(player_a.aliases[0].name, "Alias A1");
        assert_eq!(player_a.aliases[0].notes, None);

        assert_eq!(player_a.aliases[1].name, "Alias A2");
        assert_eq!(
            player_a.aliases[1].notes.as_deref(),
            Some("test provenance note")
        );

        let player_b = &catalogue.players[1];
        assert_eq!(player_b.key, "test:player-b");
        assert_eq!(player_b.preferred_name, "Player B");
        assert!(player_b.aliases.is_empty());

        Ok(())
    }

    #[test]
    fn rejects_semantically_invalid_catalogues() {
        let cases = [
            (
                r#"{"version":0,"players":[]}"#,
                "version must be greater than zero",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"","preferred_name":"Player A"}
                    ]
                }"#,
                "key must not be empty",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"test:a","preferred_name":""}
                    ]
                }"#,
                "preferred name must not be empty",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {"key":"test:a","preferred_name":"Player A"},
                        {"key":"test:a","preferred_name":"Player B"}
                    ]
                }"#,
                "duplicate player catalogue key",
            ),
            (
                r#"{
                    "version":1,
                    "players":[
                        {
                            "key":"test:a",
                            "preferred_name":"Player A",
                            "aliases":[
                                {"name":"Alias A"},
                                {"name":"Alias A"}
                            ]
                        }
                    ]
                }"#,
                "duplicate player catalogue alias",
            ),
        ];

        for (json, expected_message) in cases {
            let error = PlayerCatalogue::from_json(json).expect_err("invalid catalogue must fail");

            assert!(
                error.to_string().contains(expected_message),
                "expected {expected_message:?}, got {error:?}"
            );
        }
    }

    #[test]
    fn rejects_malformed_catalogue() {
        let error = PlayerCatalogue::from_json(
            r#"
            {
              "version": 1,
              "players": [
                {
                  "preferred_name": "Missing key"
                }
              ]
            }
            "#,
        )
        .expect_err("catalogue without player key must fail");

        assert!(error.to_string().contains("parsing player catalogue JSON"));
    }
}
