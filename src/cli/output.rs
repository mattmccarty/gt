//! Output formatting for gitid
//!
//! This module handles formatting command output for different formats:
//! - Terminal (human-readable with colors)
//! - JSON (machine-readable)
//! - CSV (tabular)

use std::collections::HashMap;

use serde::Serialize;

use crate::cli::args::OutputFormat;
use crate::cmd::Context;
use crate::error::{Error, Result};

/// Output from a command execution
#[derive(Debug, Serialize)]
pub struct Output {
    /// Whether the operation succeeded
    pub success: bool,

    /// Main message
    pub message: String,

    /// Additional details (key-value pairs)
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub details: HashMap<String, String>,

    /// Tabular data (for list commands)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table: Vec<HashMap<String, String>>,

    /// Warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Is this a dry-run result?
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub dry_run: bool,
}

impl Output {
    /// Create a success output with a message
    #[must_use]
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            details: HashMap::new(),
            table: Vec::new(),
            warnings: Vec::new(),
            dry_run: false,
        }
    }

    /// Create an error output
    #[must_use]
    pub fn error(err: &Error) -> Self {
        Self {
            success: false,
            message: err.to_string(),
            details: HashMap::new(),
            table: Vec::new(),
            warnings: Vec::new(),
            dry_run: false,
        }
    }

    /// Create a dry-run output
    #[must_use]
    pub fn dry_run(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            details: HashMap::new(),
            table: Vec::new(),
            warnings: Vec::new(),
            dry_run: true,
        }
    }

    /// Add a detail
    #[must_use]
    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }

    /// Add a warning
    #[must_use]
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add a table row
    #[must_use]
    pub fn with_row(mut self, row: HashMap<String, String>) -> Self {
        self.table.push(row);
        self
    }

    /// Add multiple table rows
    #[must_use]
    pub fn with_rows(mut self, rows: Vec<HashMap<String, String>>) -> Self {
        self.table.extend(rows);
        self
    }

    /// Print the output according to context settings
    pub fn print(&self, ctx: &Context) -> Result<()> {
        match ctx.output_format {
            OutputFormat::Terminal => self.print_terminal(ctx),
            OutputFormat::Json => self.print_json(),
            OutputFormat::Csv => self.print_csv(),
        }
    }

    fn print_terminal(&self, ctx: &Context) -> Result<()> {
        use std::io::Write;

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();

        // Don't print if quiet mode (except errors)
        if ctx.quiet && self.success {
            return Ok(());
        }

        // Dry-run prefix
        if self.dry_run {
            writeln!(handle, "[DRY RUN] {}", self.message)?;
        } else if self.success {
            writeln!(handle, "{}", self.message)?;
        } else {
            writeln!(handle, "Error: {}", self.message)?;
        }

        // Details
        for (key, value) in &self.details {
            writeln!(handle, "  {}: {}", key, value)?;
        }

        // Table (simple format)
        if !self.table.is_empty() {
            writeln!(handle)?;
            self.print_table(&mut handle)?;
        }

        // Warnings
        for warning in &self.warnings {
            writeln!(handle, "Warning: {}", warning)?;
        }

        Ok(())
    }

    fn print_table(&self, handle: &mut impl std::io::Write) -> Result<()> {
        if self.table.is_empty() {
            return Ok(());
        }

        // Get all column names from all rows
        let mut columns: Vec<String> = self
            .table
            .iter()
            .flat_map(|row| row.keys().cloned())
            .collect();
        columns.sort();
        columns.dedup();

        // Calculate column widths
        let widths: HashMap<&str, usize> = columns
            .iter()
            .map(|col| {
                let max_width = self
                    .table
                    .iter()
                    .map(|row| row.get(col).map_or(0, String::len))
                    .max()
                    .unwrap_or(0)
                    .max(col.len());
                (col.as_str(), max_width)
            })
            .collect();

        // Print header
        for (i, col) in columns.iter().enumerate() {
            let width = widths[col.as_str()];
            if i > 0 {
                write!(handle, "  ")?;
            }
            write!(handle, "{:width$}", col.to_uppercase(), width = width)?;
        }
        writeln!(handle)?;

        // Print rows
        for row in &self.table {
            for (i, col) in columns.iter().enumerate() {
                let width = widths[col.as_str()];
                let value = row.get(col).map_or("", String::as_str);
                if i > 0 {
                    write!(handle, "  ")?;
                }
                write!(handle, "{:width$}", value, width = width)?;
            }
            writeln!(handle)?;
        }

        Ok(())
    }

    fn print_json(&self) -> Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        println!("{}", json);
        Ok(())
    }

    fn print_csv(&self) -> Result<()> {
        if self.table.is_empty() {
            // For non-tabular output, just print message
            println!("success,message");
            println!("{},{}", self.success, self.message);
            return Ok(());
        }

        // Get all column names
        let mut columns: Vec<String> = self
            .table
            .iter()
            .flat_map(|row| row.keys().cloned())
            .collect();
        columns.sort();
        columns.dedup();

        // Print header
        println!("{}", columns.join(","));

        // Print rows
        for row in &self.table {
            let values: Vec<&str> = columns
                .iter()
                .map(|col| row.get(col).map_or("", String::as_str))
                .collect();
            println!("{}", values.join(","));
        }

        Ok(())
    }
}

/// Builder for creating table output
pub struct TableBuilder {
    columns: Vec<String>,
    rows: Vec<HashMap<String, String>>,
}

impl TableBuilder {
    /// Create a new table builder with columns
    #[must_use]
    pub fn new(columns: Vec<impl Into<String>>) -> Self {
        Self {
            columns: columns.into_iter().map(Into::into).collect(),
            rows: Vec::new(),
        }
    }

    /// Add a row with values in column order
    #[must_use]
    pub fn row(mut self, values: Vec<impl Into<String>>) -> Self {
        let row: HashMap<String, String> = self
            .columns
            .iter()
            .zip(values)
            .map(|(k, v)| (k.clone(), v.into()))
            .collect();
        self.rows.push(row);
        self
    }

    /// Build the output
    #[must_use]
    pub fn build(self, message: impl Into<String>) -> Output {
        Output::success(message).with_rows(self.rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_success() {
        let output = Output::success("Operation completed");
        assert!(output.success);
        assert_eq!(output.message, "Operation completed");
    }

    #[test]
    fn test_output_with_details() {
        let output = Output::success("Done")
            .with_detail("identity", "work")
            .with_detail("strategy", "ssh-alias");

        assert_eq!(output.details.len(), 2);
        assert_eq!(output.details.get("identity"), Some(&"work".to_string()));
    }

    #[test]
    fn test_table_builder() {
        let output = TableBuilder::new(vec!["name", "email", "provider"])
            .row(vec!["work", "work@co.com", "github"])
            .row(vec!["personal", "me@email.com", "github"])
            .build("Found 2 identities");

        assert_eq!(output.table.len(), 2);
        assert_eq!(output.table[0].get("name"), Some(&"work".to_string()));
    }
}
