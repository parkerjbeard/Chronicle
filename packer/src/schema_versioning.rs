use serde::{Deserialize, Serialize};
use arrow::datatypes::{DataType, Field, Schema};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use anyhow::{Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl SchemaVersion {
    pub const V1_0_0: Self = Self { major: 1, minor: 0, patch: 0 };
    pub const V1_1_0: Self = Self { major: 1, minor: 1, patch: 0 };
    pub const V2_0_0: Self = Self { major: 2, minor: 0, patch: 0 };
    pub const CURRENT: Self = Self::V1_0_0;
    
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
    
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        // Same major version, this minor >= other minor
        self.major == other.major && self.minor >= other.minor
    }
    
    pub fn needs_migration(&self, target: &Self) -> bool {
        self != target
    }
}

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for SchemaVersion {
    type Err = anyhow::Error;
    
    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(anyhow!("Invalid version format: {}", s));
        }
        
        Ok(Self {
            major: parts[0].parse()?,
            minor: parts[1].parse()?,
            patch: parts[2].parse()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub version: SchemaVersion,
    pub created_at: DateTime<Utc>,
    pub description: String,
    pub backward_compatible: bool,
    pub migration_notes: Vec<String>,
}

impl SchemaMetadata {
    pub fn new(version: SchemaVersion, description: String) -> Self {
        Self {
            version,
            created_at: Utc::now(),
            description,
            backward_compatible: true,
            migration_notes: Vec::new(),
        }
    }
    
    pub fn with_breaking_changes(mut self, notes: Vec<String>) -> Self {
        self.backward_compatible = false;
        self.migration_notes = notes;
        self
    }
}

pub trait SchemaMigration: Send + Sync {
    fn from_version(&self) -> &SchemaVersion;
    fn to_version(&self) -> &SchemaVersion;
    fn migrate_schema(&self, schema: &Schema) -> Result<Schema>;
    fn migrate_data(&self, data: &mut HashMap<String, Vec<u8>>) -> Result<()>;
    fn description(&self) -> &str;
}

#[derive(Debug)]
pub struct SchemaRegistry {
    schemas: HashMap<SchemaVersion, (Schema, SchemaMetadata)>,
    migrations: Vec<Box<dyn SchemaMigration>>,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            schemas: HashMap::new(),
            migrations: Vec::new(),
        };
        
        // Register built-in schemas
        registry.register_builtin_schemas();
        registry.register_builtin_migrations();
        
        registry
    }
    
    fn register_builtin_schemas(&mut self) {
        // Schema v1.0.0 - Original
        let v1_schema = Schema::new(vec![
            Field::new("timestamp_ns", DataType::Int64, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("app_bundle_id", DataType::Utf8, true),
            Field::new("window_title", DataType::Utf8, true),
            Field::new("key_sequence", DataType::Utf8, true),
            Field::new("frame_path", DataType::Utf8, true),
        ]);
        
        let v1_metadata = SchemaMetadata::new(
            SchemaVersion::V1_0_0,
            "Original Chronicle schema".to_string(),
        );
        
        self.schemas.insert(SchemaVersion::V1_0_0, (v1_schema, v1_metadata));
        
        // Schema v1.1.0 - Added metadata field
        let v1_1_schema = Schema::new(vec![
            Field::new("timestamp_ns", DataType::Int64, false),
            Field::new("event_type", DataType::Utf8, false),
            Field::new("app_bundle_id", DataType::Utf8, true),
            Field::new("window_title", DataType::Utf8, true),
            Field::new("key_sequence", DataType::Utf8, true),
            Field::new("frame_path", DataType::Utf8, true),
            Field::new("metadata", DataType::Binary, true), // New field
        ]);
        
        let v1_1_metadata = SchemaMetadata::new(
            SchemaVersion::V1_1_0,
            "Added flexible metadata field".to_string(),
        );
        
        self.schemas.insert(SchemaVersion::V1_1_0, (v1_1_schema, v1_1_metadata));
    }
    
    fn register_builtin_migrations(&mut self) {
        // Migration from v1.0.0 to v1.1.0
        self.migrations.push(Box::new(MigrationV1_0_ToV1_1::new()));
    }
    
    pub fn register_schema(&mut self, version: SchemaVersion, schema: Schema, metadata: SchemaMetadata) {
        self.schemas.insert(version, (schema, metadata));
    }
    
    pub fn register_migration(&mut self, migration: Box<dyn SchemaMigration>) {
        self.migrations.push(migration);
    }
    
    pub fn get_schema(&self, version: &SchemaVersion) -> Result<&Schema> {
        self.schemas.get(version)
            .map(|(schema, _)| schema)
            .ok_or_else(|| anyhow!("Schema version {} not found", version))
    }
    
    pub fn get_metadata(&self, version: &SchemaVersion) -> Result<&SchemaMetadata> {
        self.schemas.get(version)
            .map(|(_, metadata)| metadata)
            .ok_or_else(|| anyhow!("Schema version {} not found", version))
    }
    
    pub fn get_current_schema(&self) -> &Schema {
        &self.schemas[&SchemaVersion::CURRENT].0
    }
    
    pub fn migrate_to_current(&self, from_version: &SchemaVersion, data: &mut HashMap<String, Vec<u8>>) -> Result<SchemaVersion> {
        if from_version == &SchemaVersion::CURRENT {
            return Ok(SchemaVersion::CURRENT);
        }
        
        let migration_path = self.find_migration_path(from_version, &SchemaVersion::CURRENT)?;
        
        for migration in migration_path {
            migration.migrate_data(data)?;
        }
        
        Ok(SchemaVersion::CURRENT)
    }
    
    fn find_migration_path(&self, from: &SchemaVersion, to: &SchemaVersion) -> Result<Vec<&dyn SchemaMigration>> {
        // Simple direct migration for now - can be enhanced for complex paths
        for migration in &self.migrations {
            if migration.from_version() == from && migration.to_version() == to {
                return Ok(vec![migration.as_ref()]);
            }
        }
        
        Err(anyhow!("No migration path found from {} to {}", from, to))
    }
    
    pub fn validate_schema_compatibility(&self, version: &SchemaVersion) -> Result<()> {
        if !version.is_compatible_with(&SchemaVersion::CURRENT) {
            return Err(anyhow!("Schema version {} is not compatible with current version {}", 
                              version, SchemaVersion::CURRENT));
        }
        Ok(())
    }
    
    pub fn list_available_versions(&self) -> Vec<&SchemaVersion> {
        self.schemas.keys().collect()
    }
}

// Example migration implementation
struct MigrationV1_0_ToV1_1 {
    from: SchemaVersion,
    to: SchemaVersion,
}

impl MigrationV1_0_ToV1_1 {
    fn new() -> Self {
        Self {
            from: SchemaVersion::V1_0_0,
            to: SchemaVersion::V1_1_0,
        }
    }
}

impl SchemaMigration for MigrationV1_0_ToV1_1 {
    fn from_version(&self) -> &SchemaVersion {
        &self.from
    }
    
    fn to_version(&self) -> &SchemaVersion {
        &self.to
    }
    
    fn migrate_schema(&self, schema: &Schema) -> Result<Schema> {
        let mut fields = schema.fields().clone();
        
        // Add metadata field if not present
        if !fields.iter().any(|f| f.name() == "metadata") {
            fields.push(Field::new("metadata", DataType::Binary, true));
        }
        
        Ok(Schema::new(fields))
    }
    
    fn migrate_data(&self, data: &mut HashMap<String, Vec<u8>>) -> Result<()> {
        // Add empty metadata field to existing records
        if !data.contains_key("metadata") {
            data.insert("metadata".to_string(), Vec::new());
        }
        Ok(())
    }
    
    fn description(&self) -> &str {
        "Add metadata field for extensible event data"
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionedParquetFile {
    pub file_path: String,
    pub schema_version: SchemaVersion,
    pub created_at: DateTime<Utc>,
    pub record_count: u64,
    pub checksum: String,
}

impl VersionedParquetFile {
    pub fn new(file_path: String, schema_version: SchemaVersion, record_count: u64) -> Self {
        Self {
            file_path,
            schema_version,
            created_at: Utc::now(),
            record_count,
            checksum: String::new(), // Will be calculated
        }
    }
    
    pub fn calculate_checksum(&mut self) -> Result<()> {
        use sha2::{Sha256, Digest};
        
        let data = std::fs::read(&self.file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        self.checksum = format!("{:x}", hasher.finalize());
        Ok(())
    }
    
    pub fn verify_checksum(&self) -> Result<bool> {
        use sha2::{Sha256, Digest};
        
        let data = std::fs::read(&self.file_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let calculated = format!("{:x}", hasher.finalize());
        Ok(calculated == self.checksum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_schema_version_parsing() {
        let version: SchemaVersion = "1.2.3".parse().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        
        assert!("invalid".parse::<SchemaVersion>().is_err());
        assert!("1.2".parse::<SchemaVersion>().is_err());
    }
    
    #[test]
    fn test_schema_compatibility() {
        let v1_0 = SchemaVersion::new(1, 0, 0);
        let v1_1 = SchemaVersion::new(1, 1, 0);
        let v2_0 = SchemaVersion::new(2, 0, 0);
        
        assert!(v1_1.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v1_1));
        assert!(!v2_0.is_compatible_with(&v1_0));
        assert!(!v1_0.is_compatible_with(&v2_0));
    }
    
    #[test]
    fn test_schema_registry() {
        let registry = SchemaRegistry::new();
        
        // Test getting schemas
        let v1_schema = registry.get_schema(&SchemaVersion::V1_0_0).unwrap();
        assert_eq!(v1_schema.fields().len(), 6);
        
        let v1_1_schema = registry.get_schema(&SchemaVersion::V1_1_0).unwrap();
        assert_eq!(v1_1_schema.fields().len(), 7);
        
        // Test current schema
        let current = registry.get_current_schema();
        assert_eq!(current.fields().len(), 6); // V1_0_0 is current
    }
    
    #[test]
    fn test_migration() {
        let registry = SchemaRegistry::new();
        let mut data = HashMap::new();
        
        // Simulate v1.0.0 data
        data.insert("timestamp_ns".to_string(), vec![1, 2, 3, 4]);
        data.insert("event_type".to_string(), vec![5, 6, 7, 8]);
        
        // Migrate to current
        let result_version = registry.migrate_to_current(&SchemaVersion::V1_0_0, &mut data);
        assert!(result_version.is_ok());
        assert_eq!(result_version.unwrap(), SchemaVersion::CURRENT);
    }
    
    #[test]
    fn test_versioned_parquet_file() {
        use tempfile::NamedTempFile;
        
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), b"test data").unwrap();
        
        let mut versioned_file = VersionedParquetFile::new(
            temp_file.path().to_string_lossy().to_string(),
            SchemaVersion::V1_0_0,
            100,
        );
        
        versioned_file.calculate_checksum().unwrap();
        assert!(!versioned_file.checksum.is_empty());
        
        assert!(versioned_file.verify_checksum().unwrap());
    }
}