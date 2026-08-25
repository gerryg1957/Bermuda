use std::{path::Path, pin::Pin};

use bermuda::{
    player_directory::{PlayerDirectory, PlayerKnownNameKind},
    project_manager::ProjectManager,
};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use serde_json::json;

#[cxx_qt::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");

        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, players_json)]
        #[qproperty(QString, unresolved_json)]
        #[qproperty(QString, aliases_json)]
        #[qproperty(QString, known_names_json)]
        #[qproperty(QString, error_message)]
        #[qproperty(QString, status_message)]
        #[qproperty(i64, selected_player_id)]
        type PlayerIdentityModel = super::PlayerIdentityModelRust;

        #[qinvokable]
        #[cxx_name = "loadProject"]
        fn load_project(self: Pin<&mut PlayerIdentityModel>, project_path: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "loadAliases"]
        fn load_aliases(self: Pin<&mut PlayerIdentityModel>, player_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "createPlayer"]
        fn create_player(self: Pin<&mut PlayerIdentityModel>, preferred_name: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "createPlayerAndAssign"]
        fn create_player_and_assign(
            self: Pin<&mut PlayerIdentityModel>,
            preferred_name: &QString,
            source_id: i64,
            source_name: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "renamePlayer"]
        fn rename_player(
            self: Pin<&mut PlayerIdentityModel>,
            player_id: i64,
            preferred_name: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "assignSourceName"]
        fn assign_source_name(
            self: Pin<&mut PlayerIdentityModel>,
            player_id: i64,
            source_id: i64,
            source_name: &QString,
        ) -> bool;

        #[qinvokable]
        #[cxx_name = "removeAlias"]
        fn remove_alias(self: Pin<&mut PlayerIdentityModel>, alias_id: i64) -> bool;

        #[qinvokable]
        #[cxx_name = "deletePlayer"]
        fn delete_player(self: Pin<&mut PlayerIdentityModel>, player_id: i64) -> bool;
    }
}

pub struct PlayerIdentityModelRust {
    pub(crate) players_json: QString,
    pub(crate) unresolved_json: QString,
    pub(crate) aliases_json: QString,
    pub(crate) known_names_json: QString,
    pub(crate) error_message: QString,
    pub(crate) status_message: QString,
    pub(crate) selected_player_id: i64,

    project_path: String,
}

impl Default for PlayerIdentityModelRust {
    fn default() -> Self {
        Self {
            players_json: QString::from("[]"),
            unresolved_json: QString::from("[]"),
            aliases_json: QString::from("[]"),
            known_names_json: QString::from("[]"),
            error_message: QString::default(),
            status_message: QString::default(),
            selected_player_id: -1,
            project_path: String::new(),
        }
    }
}

struct IdentitySnapshot {
    players_json: QString,
    unresolved_json: QString,
    aliases_json: QString,
    known_names_json: QString,
}

impl ffi::PlayerIdentityModel {
    fn load_project(mut self: Pin<&mut Self>, project_path: &QString) -> bool {
        let project_path = project_path.to_string();

        if project_path.trim().is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        {
            let mut rust = self.as_mut().rust_mut();
            rust.project_path = project_path.clone();
        }

        refresh_model(self, &project_path, None, None)
    }

    fn load_aliases(mut self: Pin<&mut Self>, player_id: i64) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let (aliases_json, known_names_json) =
            match player_name_json_for_player(&project_path, player_id) {
                Ok(values) => values,
                Err(error) => return set_failure(self, error),
            };

        self.as_mut().set_aliases_json(aliases_json);
        self.as_mut().set_known_names_json(known_names_json);
        self.as_mut().set_selected_player_id(player_id);
        self.as_mut().set_error_message(QString::default());

        true
    }

    fn create_player(self: Pin<&mut Self>, preferred_name: &QString) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let preferred_name = preferred_name.to_string();

        let player = match ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| directory.create_player(&preferred_name))
        {
            Ok(player) => player,
            Err(error) => return set_failure(self, error.to_string()),
        };

        refresh_model(
            self,
            &project_path,
            Some(player.id),
            Some(format!("Created player {}", player.preferred_name)),
        )
    }

    fn create_player_and_assign(
        self: Pin<&mut Self>,
        preferred_name: &QString,
        source_id: i64,
        source_name: &QString,
    ) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let preferred_name = preferred_name.to_string();
        let source_name = source_name.to_string();

        let (player, result) = match ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| {
                directory.create_player_and_assign_source_name(
                    &preferred_name,
                    source_id,
                    &source_name,
                )
            }) {
            Ok(value) => value,
            Err(error) => return set_failure(self, error.to_string()),
        };

        refresh_model(
            self,
            &project_path,
            Some(player.id),
            Some(format!(
                "Created {} and assigned {} occurrence(s) of {:?} from {} {}",
                player.preferred_name,
                result.linked_count(),
                result.name,
                result.source_name,
                result.source_version
            )),
        )
    }

    fn rename_player(self: Pin<&mut Self>, player_id: i64, preferred_name: &QString) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let preferred_name = preferred_name.to_string();

        let result = ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| directory.rename_player(player_id, &preferred_name));

        if let Err(error) = result {
            return set_failure(self, error.to_string());
        }

        refresh_model(
            self,
            &project_path,
            Some(player_id),
            Some(format!("Renamed player to {}", preferred_name.trim())),
        )
    }

    fn assign_source_name(
        self: Pin<&mut Self>,
        player_id: i64,
        source_id: i64,
        source_name: &QString,
    ) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let source_name = source_name.to_string();

        let result = match ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| {
                directory.assign_source_name_to_player(player_id, source_id, &source_name)
            }) {
            Ok(result) => result,
            Err(error) => return set_failure(self, error.to_string()),
        };

        refresh_model(
            self,
            &project_path,
            Some(player_id),
            Some(format!(
                "Assigned {} occurrence(s) of {:?} from {} {}",
                result.linked_count(),
                result.name,
                result.source_name,
                result.source_version
            )),
        )
    }

    fn remove_alias(self: Pin<&mut Self>, alias_id: i64) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();
        let player_id = self.as_ref().rust().selected_player_id;

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        if player_id < 0 {
            return set_failure(self, "no player identity is currently selected".to_owned());
        }

        let result = ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| directory.remove_alias(alias_id));

        if let Err(error) = result {
            return set_failure(self, error.to_string());
        }

        refresh_model(
            self,
            &project_path,
            Some(player_id),
            Some("Unlinked alias; matching source names are unresolved again".to_owned()),
        )
    }

    fn delete_player(self: Pin<&mut Self>, player_id: i64) -> bool {
        let project_path = self.as_ref().rust().project_path.clone();

        if project_path.is_empty() {
            return set_failure(self, "no Bermuda database is currently open".to_owned());
        }

        let preferred_name = match ProjectManager::new()
            .open(Path::new(&project_path))
            .and_then(|project| project.player_directory())
            .and_then(|directory| directory.delete_player(player_id))
        {
            Ok(preferred_name) => preferred_name,
            Err(error) => return set_failure(self, error.to_string()),
        };

        refresh_model(
            self,
            &project_path,
            None,
            Some(format!(
                "Removed identity {preferred_name}; source names and games were preserved"
            )),
        )
    }
}

fn set_failure(mut model: Pin<&mut ffi::PlayerIdentityModel>, error: String) -> bool {
    model.as_mut().set_error_message(QString::from(error));
    model.as_mut().set_status_message(QString::default());

    false
}

fn refresh_model(
    mut model: Pin<&mut ffi::PlayerIdentityModel>,
    project_path: &str,
    selected_player_id: Option<i64>,
    status: Option<String>,
) -> bool {
    let snapshot = match load_snapshot(project_path, selected_player_id) {
        Ok(snapshot) => snapshot,
        Err(error) => return set_failure(model, error),
    };

    model.as_mut().set_players_json(snapshot.players_json);
    model.as_mut().set_unresolved_json(snapshot.unresolved_json);
    model.as_mut().set_aliases_json(snapshot.aliases_json);
    model
        .as_mut()
        .set_known_names_json(snapshot.known_names_json);
    model
        .as_mut()
        .set_selected_player_id(selected_player_id.unwrap_or(-1));
    model.as_mut().set_error_message(QString::default());
    model
        .as_mut()
        .set_status_message(QString::from(status.unwrap_or_default()));

    true
}

fn load_snapshot(
    project_path: &str,
    selected_player_id: Option<i64>,
) -> Result<IdentitySnapshot, String> {
    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    let directory = project
        .player_directory()
        .map_err(|error| error.to_string())?;

    let players = directory
        .list_players()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|player| {
            json!({
                "id": player.id,
                "preferredName": player.preferred_name,
            })
        })
        .collect::<Vec<_>>();

    let unresolved = directory
        .unresolved_names()
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|name| {
            json!({
                "sourceId": name.source_id,
                "sourceName": name.source_name,
                "sourceVersion": name.source_version,
                "name": name.name,
                "occurrenceCount": name.occurrence_count,
            })
        })
        .collect::<Vec<_>>();

    let (aliases, known_names) = match selected_player_id {
        Some(player_id) => player_name_values(&directory, player_id)?,
        None => (Vec::new(), Vec::new()),
    };

    Ok(IdentitySnapshot {
        players_json: serialise_json(&players)?,
        unresolved_json: serialise_json(&unresolved)?,
        aliases_json: serialise_json(&aliases)?,
        known_names_json: serialise_json(&known_names)?,
    })
}

fn player_name_values(
    directory: &PlayerDirectory,
    player_id: i64,
) -> Result<(Vec<serde_json::Value>, Vec<serde_json::Value>), String> {
    let aliases = directory
        .aliases_for_player(player_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|alias| {
            json!({
                "id": alias.id,
                "name": alias.name,
                "sourceId": alias.source_id,
                "sourceName": alias.source_name,
                "sourceVersion": alias.source_version,
                "notes": alias.notes,
            })
        })
        .collect::<Vec<_>>();

    let known_names = directory
        .known_names_for_player(player_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .map(|known_name| {
            let kind = match known_name.kind {
                PlayerKnownNameKind::Preferred => "preferred",
                PlayerKnownNameKind::Supplied => "supplied",
                PlayerKnownNameKind::Local => "local",
            };

            json!({
                "name": known_name.name,
                "kind": kind,
                "localAliasId": known_name.local_alias_id,
                "sourceId": known_name.source_id,
                "sourceName": known_name.source_name,
                "sourceVersion": known_name.source_version,
                "notes": known_name.notes,
            })
        })
        .collect::<Vec<_>>();

    Ok((aliases, known_names))
}

fn player_name_json_for_player(
    project_path: &str,
    player_id: i64,
) -> Result<(QString, QString), String> {
    let project = ProjectManager::new()
        .open(Path::new(project_path))
        .map_err(|error| error.to_string())?;

    let directory = project
        .player_directory()
        .map_err(|error| error.to_string())?;

    let (aliases, known_names) = player_name_values(&directory, player_id)?;

    Ok((serialise_json(&aliases)?, serialise_json(&known_names)?))
}

fn serialise_json(values: &[serde_json::Value]) -> Result<QString, String> {
    serde_json::to_string(values)
        .map(QString::from)
        .map_err(|error| error.to_string())
}
