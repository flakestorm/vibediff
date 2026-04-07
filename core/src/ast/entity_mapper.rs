use std::{ops::RangeInclusive, path::PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::git::diff_parser::Language;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChange {
    pub entity_id: Uuid,
    pub file_path: PathBuf,
    pub language: Language,
    pub entity_type: EntityType,
    pub entity_name: String,
    pub fully_qualified_name: String,
    pub change_kind: ChangeKind,
    pub changed_lines: RangeInclusive<u32>,
    pub side_effects: Vec<SideEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityType {
    Function,
    Method,
    Class,
    Module,
    Interface,
    Type,
    Constant,
    Import,
    Export,
    Decorator,
    Attribute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SideEffect {
    ExternalApiCall,
    GlobalStateRead,
    EnvVarAccess,
}
