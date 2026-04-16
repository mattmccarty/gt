//! Interactive prompts for gitid
//!
//! This module provides interactive prompts using dialoguer.

use crate::error::{Error, Result};

/// Prompts the user for confirmation
#[cfg(feature = "interactive")]
pub fn confirm(prompt: &str, default: bool) -> Result<bool> {
    dialoguer::Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn confirm(_prompt: &str, default: bool) -> Result<bool> {
    Ok(default)
}

/// Prompts the user for text input
#[cfg(feature = "interactive")]
pub fn input(prompt: &str) -> Result<String> {
    dialoguer::Input::new()
        .with_prompt(prompt)
        .interact_text()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn input(_prompt: &str) -> Result<String> {
    Err(Error::InputRequired {
        field: "input".to_string(),
    })
}

/// Prompts the user for text input with a default value
#[cfg(feature = "interactive")]
pub fn input_with_default(prompt: &str, default: &str) -> Result<String> {
    dialoguer::Input::new()
        .with_prompt(prompt)
        .default(default.to_string())
        .interact_text()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn input_with_default(_prompt: &str, default: &str) -> Result<String> {
    Ok(default.to_string())
}

/// Prompts the user to select from a list
#[cfg(feature = "interactive")]
pub fn select<T: ToString>(prompt: &str, items: &[T]) -> Result<usize> {
    dialoguer::Select::new()
        .with_prompt(prompt)
        .items(items)
        .interact()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn select<T: ToString>(_prompt: &str, _items: &[T]) -> Result<usize> {
    Ok(0)
}

/// Prompts the user for multiple selections
#[cfg(feature = "interactive")]
pub fn multi_select<T: ToString>(prompt: &str, items: &[T]) -> Result<Vec<usize>> {
    dialoguer::MultiSelect::new()
        .with_prompt(prompt)
        .items(items)
        .interact()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn multi_select<T: ToString>(_prompt: &str, items: &[T]) -> Result<Vec<usize>> {
    Ok((0..items.len()).collect())
}

/// Prompts for a password/passphrase (hidden input)
#[cfg(feature = "interactive")]
pub fn password(prompt: &str) -> Result<String> {
    dialoguer::Password::new()
        .with_prompt(prompt)
        .interact()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn password(_prompt: &str) -> Result<String> {
    Err(Error::InputRequired {
        field: "password".to_string(),
    })
}

/// Prompts for a password with confirmation
#[cfg(feature = "interactive")]
pub fn password_with_confirm(prompt: &str) -> Result<String> {
    dialoguer::Password::new()
        .with_prompt(prompt)
        .with_confirmation("Confirm", "Passwords don't match")
        .interact()
        .map_err(|_| Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn password_with_confirm(_prompt: &str) -> Result<String> {
    Err(Error::InputRequired {
        field: "password".to_string(),
    })
}

/// Editor prompt for multi-line input
#[cfg(feature = "interactive")]
pub fn editor(initial_content: &str) -> Result<String> {
    dialoguer::Editor::new()
        .edit(initial_content)
        .map_err(|_| Error::Cancelled)?
        .ok_or(Error::Cancelled)
}

#[cfg(not(feature = "interactive"))]
pub fn editor(initial_content: &str) -> Result<String> {
    Ok(initial_content.to_string())
}
