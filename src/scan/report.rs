//! Scan report generation
//!
//! This module generates human-readable and structured scan reports.

use crate::scan::detector::DetectedIdentity;
use crate::scan::git_scanner::GitScanResult;
use crate::scan::ssh_scanner::SshScanResult;

/// Complete scan report
#[derive(Debug, Default)]
pub struct ScanReport {
    /// SSH scan results
    pub ssh: SshScanResult,
    /// Git scan results
    pub git: GitScanResult,
    /// Detected identities
    pub identities: Vec<DetectedIdentity>,
    /// Recommendations
    pub recommendations: Vec<Recommendation>,
}

/// A recommendation from the scan
#[derive(Debug)]
pub struct Recommendation {
    /// Recommendation type
    pub kind: RecommendationType,
    /// Description
    pub description: String,
    /// Suggested command
    pub command: Option<String>,
}

/// Type of recommendation
#[derive(Debug, Clone, Copy)]
pub enum RecommendationType {
    /// Import existing configuration
    Import,
    /// Fix configuration issue
    Fix,
    /// Security improvement
    Security,
    /// Performance improvement
    Performance,
    /// General suggestion
    Suggestion,
}

impl ScanReport {
    /// Generate recommendations based on scan results
    pub fn generate_recommendations(&mut self) {
        // Recommend import if identities found
        if !self.identities.is_empty() {
            self.recommendations.push(Recommendation {
                kind: RecommendationType::Import,
                description: format!(
                    "Found {} existing identities that can be imported",
                    self.identities.len()
                ),
                command: Some("gitid init --import".to_string()),
            });
        }

        // Check for orphaned SSH keys
        for key in &self.ssh.keys {
            if key.is_gitid && !key.in_config {
                self.recommendations.push(Recommendation {
                    kind: RecommendationType::Fix,
                    description: format!(
                        "SSH key {} is not referenced in SSH config",
                        key.path
                    ),
                    command: None,
                });
            }
        }

        // Check for missing include files
        for cond in &self.git.conditionals {
            if !cond.file_exists {
                self.recommendations.push(Recommendation {
                    kind: RecommendationType::Fix,
                    description: format!(
                        "Conditional include file {} does not exist",
                        cond.path
                    ),
                    command: None,
                });
            }
        }

        // Security recommendations
        // Note: Permission checks would be added here
    }

    /// Get summary statistics
    #[must_use]
    pub fn summary(&self) -> ScanSummary {
        ScanSummary {
            ssh_hosts: self.ssh.hosts.len(),
            ssh_keys: self.ssh.keys.len(),
            gitid_hosts: self.ssh.hosts.iter().filter(|h| h.is_gitid).count(),
            gitid_keys: self.ssh.keys.iter().filter(|k| k.is_gitid).count(),
            conditionals: self.git.conditionals.len(),
            url_rewrites: self.git.url_rewrites.len(),
            detected_identities: self.identities.len(),
            recommendations: self.recommendations.len(),
        }
    }
}

/// Summary statistics from a scan
#[derive(Debug)]
pub struct ScanSummary {
    /// Total SSH host entries
    pub ssh_hosts: usize,
    /// Total SSH keys
    pub ssh_keys: usize,
    /// gitid-style SSH hosts
    pub gitid_hosts: usize,
    /// gitid-style SSH keys
    pub gitid_keys: usize,
    /// Git conditional includes
    pub conditionals: usize,
    /// Git URL rewrites
    pub url_rewrites: usize,
    /// Detected identities
    pub detected_identities: usize,
    /// Recommendations
    pub recommendations: usize,
}

impl std::fmt::Display for ScanSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Scan Summary")?;
        writeln!(f, "============")?;
        writeln!(f, "SSH hosts: {} ({} gitid)", self.ssh_hosts, self.gitid_hosts)?;
        writeln!(f, "SSH keys: {} ({} gitid)", self.ssh_keys, self.gitid_keys)?;
        writeln!(f, "Conditional includes: {}", self.conditionals)?;
        writeln!(f, "URL rewrites: {}", self.url_rewrites)?;
        writeln!(f, "Detected identities: {}", self.detected_identities)?;
        writeln!(f, "Recommendations: {}", self.recommendations)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_report() {
        let report = ScanReport::default();
        let summary = report.summary();

        assert_eq!(summary.ssh_hosts, 0);
        assert_eq!(summary.detected_identities, 0);
    }
}
