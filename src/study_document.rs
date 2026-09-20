use crate::Collection;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const STUDY_DOCUMENT_VERSION: u32 = 1;
pub const STUDY_METADATA_PROPERTY: &str = "XBERMUDA";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StudyOrigin {
    ProjectGame { project_path: String, game_id: i64 },
    ExternalSgf { path: String },
    Detached { description: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StudyAnnotationKind {
    Cross,
    Triangle,
    Circle,
    Square,
    Letter,
    MoveNumber { move_number: u32 },
    Label { text: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyAnnotation {
    pub node_id: Option<usize>,
    pub move_number: usize,
    pub point: u16,
    pub kind: StudyAnnotationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StudyDocumentMetadata {
    pub version: u32,
    pub origin: StudyOrigin,
    pub annotations: Vec<StudyAnnotation>,
}

#[derive(Debug, Error)]
pub enum StudyDocumentError {
    #[error("Study document contains no SGF root node")]
    MissingRootNode,

    #[error("invalid Bermuda Study metadata: {0}")]
    InvalidMetadata(#[from] serde_json::Error),

    #[error("unsupported Bermuda Study document version {0}")]
    UnsupportedVersion(u32),
}

impl StudyDocumentMetadata {
    #[must_use]
    pub fn new(origin: StudyOrigin) -> Self {
        Self {
            version: STUDY_DOCUMENT_VERSION,
            origin,
            annotations: Vec::new(),
        }
    }

    pub fn from_collection(collection: &Collection) -> Result<Option<Self>, StudyDocumentError> {
        let Some(root) = collection
            .trees
            .first()
            .and_then(|tree| tree.sequence.first())
        else {
            return Err(StudyDocumentError::MissingRootNode);
        };

        let Some(value) = root.first(STUDY_METADATA_PROPERTY) else {
            return Ok(None);
        };

        let metadata: Self = serde_json::from_str(value)?;

        if metadata.version != STUDY_DOCUMENT_VERSION {
            return Err(StudyDocumentError::UnsupportedVersion(metadata.version));
        }

        Ok(Some(metadata))
    }

    pub fn apply_to_collection(
        &self,
        collection: &mut Collection,
    ) -> Result<(), StudyDocumentError> {
        let Some(root) = collection
            .trees
            .first_mut()
            .and_then(|tree| tree.sequence.first_mut())
        else {
            return Err(StudyDocumentError::MissingRootNode);
        };

        let value = serde_json::to_string(self)?;

        root.properties
            .insert(STUDY_METADATA_PROPERTY.to_owned(), vec![value]);

        Ok(())
    }

    pub fn set_annotation(
        &mut self,
        node_id: Option<usize>,
        move_number: usize,
        point: u16,
        kind: StudyAnnotationKind,
    ) {
        let existing = self.annotations.iter().position(|annotation| {
            annotation.node_id == node_id
                && annotation.move_number == move_number
                && annotation.point == point
        });

        if let Some(index) = existing {
            if self.annotations[index].kind == kind {
                self.annotations.remove(index);
                return;
            }

            self.annotations.remove(index);
        }

        self.annotations.push(StudyAnnotation {
            node_id,
            move_number,
            point,
            kind,
        });
    }

    pub fn annotations_at(
        &self,
        node_id: Option<usize>,
        move_number: usize,
    ) -> impl Iterator<Item = &StudyAnnotation> {
        self.annotations.iter().filter(move |annotation| {
            annotation.node_id == node_id && annotation.move_number == move_number
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_collection, write_collection_sgf};

    #[test]
    fn metadata_round_trip_is_self_contained_in_sgf() {
        let mut collection = parse_collection(b"(;FF[4]GM[1]SZ[19];B[pd](;W[dd])(;W[pp]))")
            .expect("parse source SGF");

        let mut metadata = StudyDocumentMetadata::new(StudyOrigin::ExternalSgf {
            path: "/tmp/source.sgf".to_owned(),
        });

        metadata.set_annotation(Some(2), 2, 3 * 19 + 3, StudyAnnotationKind::Letter);
        metadata.set_annotation(
            Some(2),
            2,
            15 * 19 + 15,
            StudyAnnotationKind::Label {
                text: "idea".to_owned(),
            },
        );

        metadata
            .apply_to_collection(&mut collection)
            .expect("apply Study metadata");

        let written = write_collection_sgf(&collection);
        let reparsed = parse_collection(written.as_bytes()).expect("reparse Study SGF");
        let restored = StudyDocumentMetadata::from_collection(&reparsed)
            .expect("read Study metadata")
            .expect("metadata should be present");

        assert_eq!(restored, metadata);
    }

    #[test]
    fn setting_same_annotation_toggles_it_off() {
        let mut metadata = StudyDocumentMetadata::new(StudyOrigin::Detached {
            description: "test".to_owned(),
        });

        metadata.set_annotation(None, 7, 42, StudyAnnotationKind::Triangle);
        assert_eq!(metadata.annotations.len(), 1);

        metadata.set_annotation(None, 7, 42, StudyAnnotationKind::Triangle);
        assert!(metadata.annotations.is_empty());
    }

    #[test]
    fn replacing_annotation_keeps_only_one_manual_mark_per_point() {
        let mut metadata = StudyDocumentMetadata::new(StudyOrigin::Detached {
            description: "test".to_owned(),
        });

        metadata.set_annotation(None, 4, 20, StudyAnnotationKind::Circle);
        metadata.set_annotation(None, 4, 20, StudyAnnotationKind::Letter);

        assert_eq!(metadata.annotations.len(), 1);
        assert_eq!(metadata.annotations[0].kind, StudyAnnotationKind::Letter);
    }
}
