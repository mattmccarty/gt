//! Demonstration of the pagination utilities
//!
//! This example shows how to use the reusable pagination functions
//! from gt::util for displaying long content with user control.
//!
//! Run with: cargo run --example pagination_demo

use gt::util::{paginate_output, execute_git_command_paginated};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example 1: Basic pagination with custom content ===\n");

    // Create some sample content
    let lines: Vec<String> = (1..=100)
        .map(|i| format!("This is line number {}", i))
        .collect();

    let header = vec![
        "Custom Header".to_string(),
        "=============".to_string(),
        "This content will be paginated, showing 20 lines at a time.".to_string(),
        String::new(),
    ];

    // Paginate the content (user presses Enter to see more)
    paginate_output(lines.into_iter(), 20, Some(header))?;

    println!("\n=== Example 2: Paginated git command output ===\n");

    // Example of using pagination with git log
    let git_header = vec![
        "Custom Git Log Header".to_string(),
        "=====================".to_string(),
        "Showing git log with pagination".to_string(),
        String::new(),
    ];

    // This would show git log output with pagination
    // execute_git_command_paginated("log", &["--oneline".to_string()], 20, Some(git_header))?;

    println!("\nPagination utilities are reusable for:");
    println!("  - Help text (like git commit --help)");
    println!("  - Log output (like git log)");
    println!("  - Search results");
    println!("  - Any long text output that needs user control");

    Ok(())
}
