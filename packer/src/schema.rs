// Schema versioning and evolution support
use serde::{Deserialize, Serialize};
use arrow::datatypes::{DataType, Field, Schema};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    pub const CURRENT: Self = Self { major: 1, minor: 0, patch: 0 };
    
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major && self.minor >= other.minor
    }
}

#[derive(Debug, Clone)]
pub struct VersionedSchema {
    pub version: SchemaVersion,
    pub schema: Schema,
    pub migrations: Vec<SchemaMigration>,
}

#[derive(Debug, Clone)]
pub struct SchemaMigration {
    pub from_version: SchemaVersion,
    pub to_version: SchemaVersion,
    pub migration_fn: fn(&mut Schema) -> Result<(), Box<dyn std::error::Error>>,
}

impl VersionedSchema {
    pub fn current() -> Self {
        Self {
            version: SchemaVersion::CURRENT,
            schema: Self::build_current_schema(),
            migrations: vec![
                // Future migrations will be added here
            ],
        }
    }
    
    fn build_current_schema() -> Schema {
        Schema::new(vec![
            Field::new("timestamp_ns", DataType::Int64, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("app_bundle_id", DataType::Utf8, true),
            Field::new("metadata", DataType::Binary, true), // JSON blob for extensibility
            Field::new("schema_version", DataType::Utf8, false),
        ])
    }
    
    pub fn migrate_from(&self, from_version: &SchemaVersion) -> Result<Schema, Box<dyn std::error::Error>> {
        let mut schema = self.schema.clone();
        
        for migration in &self.migrations {
            if migration.from_version == *from_version {
                (migration.migration_fn)(&mut schema)?;
            }
        }
        
        Ok(schema)
    }
}

// Future-proof event format
#[derive(Debug, Serialize, Deserialize)]
pub struct EventMetadata {
    pub version: String,
    pub event_specific: serde_json::Value, // Flexible metadata
    pub extensions: Option<serde_json::Value>, // Future extensions
}