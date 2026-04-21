//! Active-identity state file
//!
//! A user-scoped file recording the identity selected by `gt config id use`
//! when the command runs outside any git repository. Passthroughs that run
//! before a repo exists (notably `gt clone`) consult this file to honor the
//! user's declared intent.
//!
//! Inside an existing repo, `gt config id use` writes local git config and
//! does not touch this file. The active-identity file is specifically the
//! fallback for the pre-repo case.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::util::config_dir;

/// Name of the file under `config_dir()` that stores the active identity.
const FILE_NAME: &str = "active-id.toml";

/// The active identity as recorded on disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ActiveIdentity {
    /// Identity name (matches a key under `[identities]` in the main config).
    pub identity: String,
}

impl ActiveIdentity {
    /// Path to the state file: `<config_dir()>/active-id.toml`.
    pub fn path() -> Result<PathBuf> {
        Ok(config_dir()?.join(FILE_NAME))
    }

    /// Load the active identity from disk, if set.
    pub fn load() -> Result<Option<Self>> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(&path)?;
        let active: ActiveIdentity = toml::from_str(&contents)?;
        Ok(Some(active))
    }

    /// Write the active identity to disk, creating the parent directory if needed.
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Remove the state file if present. Returns `true` if a file was removed.
    pub fn clear() -> Result<bool> {
        let path = Self::path()?;
        if path.exists() {
            fs::remove_file(&path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_toml() {
        let original = ActiveIdentity {
            identity: "work".to_string(),
        };
        let serialized = toml::to_string_pretty(&original).unwrap();
        let parsed: ActiveIdentity = toml::from_str(&serialized).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn rejects_missing_identity_field() {
        let bad = "";
        let result: std::result::Result<ActiveIdentity, _> = toml::from_str(bad);
        assert!(result.is_err());
    }

    #[test]
    fn accepts_extra_fields_gracefully() {
        // Forward compatibility: if a future gt adds fields, an older gt should
        // still parse the file rather than fail hard.
        let input = "identity = \"work\"\nunknown = \"value\"\n";
        let parsed: ActiveIdentity = toml::from_str(input).unwrap();
        assert_eq!(parsed.identity, "work");
    }
}
