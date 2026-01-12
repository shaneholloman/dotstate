//! Form field and form handling utilities.
//!
//! This module provides abstractions for handling form fields and forms
//! in the TUI, reducing code duplication across screens that need text input.

use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// A single form field with text input support.
#[derive(Debug, Clone)]
pub struct FormField {
    /// The current value of the field.
    pub value: String,
    /// The cursor position within the value.
    pub cursor: usize,
    /// Label displayed above the field.
    pub label: String,
    /// Optional placeholder text shown when value is empty.
    pub placeholder: Option<String>,
    /// Whether this field currently has focus.
    pub is_focused: bool,
    /// Whether this field is disabled (read-only).
    pub is_disabled: bool,
    /// Optional validation function. Returns None if valid, Some(error) if invalid.
    validator: Option<fn(&str) -> Option<String>>,
    /// Cached validation error message.
    validation_error: Option<String>,
}

impl Default for FormField {
    fn default() -> Self {
        Self::new("")
    }
}

impl FormField {
    /// Create a new form field with the given label.
    pub fn new(label: &str) -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            label: label.to_string(),
            placeholder: None,
            is_focused: false,
            is_disabled: false,
            validator: None,
            validation_error: None,
        }
    }

    /// Create a new form field with a placeholder.
    pub fn with_placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    /// Set a validation function for this field.
    pub fn with_validator(mut self, validator: fn(&str) -> Option<String>) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Set the initial value.
    pub fn with_value(mut self, value: &str) -> Self {
        self.value = value.to_string();
        self.cursor = value.chars().count();
        self
    }

    /// Clear the field value.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
        self.validation_error = None;
    }

    /// Set the field value.
    pub fn set_value(&mut self, value: &str) {
        self.value = value.to_string();
        self.cursor = value.chars().count();
        self.validate();
    }

    /// Get the current value trimmed.
    pub fn value_trimmed(&self) -> &str {
        self.value.trim()
    }

    /// Check if the field is empty (ignoring whitespace).
    pub fn is_empty(&self) -> bool {
        self.value.trim().is_empty()
    }

    /// Validate the field and return whether it's valid.
    pub fn validate(&mut self) -> bool {
        if let Some(validator) = self.validator {
            self.validation_error = validator(&self.value);
            self.validation_error.is_none()
        } else {
            self.validation_error = None;
            true
        }
    }

    /// Get the validation error message if any.
    pub fn validation_error(&self) -> Option<&str> {
        self.validation_error.as_deref()
    }

    /// Handle a key press event.
    ///
    /// Returns true if the event was handled.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if self.is_disabled {
            return false;
        }

        match key {
            KeyCode::Char(c) => {
                self.insert_char(c);
                true
            }
            KeyCode::Backspace => {
                self.handle_backspace();
                true
            }
            KeyCode::Delete => {
                self.handle_delete();
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.value.chars().count() {
                    self.cursor += 1;
                }
                true
            }
            KeyCode::Home => {
                self.cursor = 0;
                true
            }
            KeyCode::End => {
                self.cursor = self.value.chars().count();
                true
            }
            _ => false,
        }
    }

    /// Insert a character at the cursor position.
    fn insert_char(&mut self, c: char) {
        let byte_pos = self
            .value
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.value.len());
        self.value.insert(byte_pos, c);
        self.cursor += 1;
    }

    /// Handle backspace key.
    fn handle_backspace(&mut self) {
        if self.cursor > 0 {
            let byte_pos = self
                .value
                .char_indices()
                .nth(self.cursor - 1)
                .map(|(i, _)| i)
                .unwrap_or(0);
            let next_byte_pos = self
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            self.value.replace_range(byte_pos..next_byte_pos, "");
            self.cursor -= 1;
        }
    }

    /// Handle delete key.
    fn handle_delete(&mut self) {
        let char_count = self.value.chars().count();
        if self.cursor < char_count {
            let byte_pos = self
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            let next_byte_pos = self
                .value
                .char_indices()
                .nth(self.cursor + 1)
                .map(|(i, _)| i)
                .unwrap_or(self.value.len());
            self.value.replace_range(byte_pos..next_byte_pos, "");
        }
    }

    /// Render the field.
    pub fn render(&self, frame: &mut Frame, area: Rect, show_label: bool) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.is_focused {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            });

        let block = if show_label && !self.label.is_empty() {
            block.title(format!(" {} ", self.label))
        } else {
            block
        };

        // Determine display text
        let display_text = if self.value.is_empty() {
            if let Some(ref placeholder) = self.placeholder {
                placeholder.clone()
            } else {
                String::new()
            }
        } else {
            self.value.clone()
        };

        let style = if self.is_disabled {
            Style::default().fg(Color::DarkGray)
        } else if self.value.is_empty() && self.placeholder.is_some() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let paragraph = Paragraph::new(display_text).style(style).block(block);

        frame.render_widget(paragraph, area);

        // Render cursor if focused
        if self.is_focused && !self.is_disabled {
            let cursor_x = area.x + 1 + self.cursor as u16;
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width - 1 {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

/// A form containing multiple fields.
#[derive(Debug)]
pub struct Form {
    /// The fields in this form.
    pub fields: Vec<FormField>,
    /// Index of the currently focused field.
    pub focused_index: usize,
}

impl Default for Form {
    fn default() -> Self {
        Self::new()
    }
}

impl Form {
    /// Create a new empty form.
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            focused_index: 0,
        }
    }

    /// Add a field to the form.
    pub fn add_field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    /// Get a reference to a field by index.
    pub fn field(&self, index: usize) -> Option<&FormField> {
        self.fields.get(index)
    }

    /// Get a mutable reference to a field by index.
    pub fn field_mut(&mut self, index: usize) -> Option<&mut FormField> {
        self.fields.get_mut(index)
    }

    /// Get the currently focused field.
    pub fn focused_field(&self) -> Option<&FormField> {
        self.fields.get(self.focused_index)
    }

    /// Get a mutable reference to the currently focused field.
    pub fn focused_field_mut(&mut self) -> Option<&mut FormField> {
        self.fields.get_mut(self.focused_index)
    }

    /// Move focus to the next field.
    pub fn next_field(&mut self) {
        if !self.fields.is_empty() {
            // Unfocus current field
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = false;
            }
            // Move to next (wrapping)
            self.focused_index = (self.focused_index + 1) % self.fields.len();
            // Focus new field
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = true;
            }
        }
    }

    /// Move focus to the previous field.
    pub fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            // Unfocus current field
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = false;
            }
            // Move to previous (wrapping)
            self.focused_index = if self.focused_index == 0 {
                self.fields.len() - 1
            } else {
                self.focused_index - 1
            };
            // Focus new field
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = true;
            }
        }
    }

    /// Focus a specific field by index.
    pub fn focus(&mut self, index: usize) {
        if index < self.fields.len() {
            // Unfocus current field
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = false;
            }
            // Focus new field
            self.focused_index = index;
            if let Some(field) = self.fields.get_mut(self.focused_index) {
                field.is_focused = true;
            }
        }
    }

    /// Handle a key press event on the focused field.
    ///
    /// Returns true if the event was handled.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Tab => {
                self.next_field();
                true
            }
            KeyCode::BackTab => {
                self.prev_field();
                true
            }
            _ => {
                if let Some(field) = self.focused_field_mut() {
                    field.handle_key(key)
                } else {
                    false
                }
            }
        }
    }

    /// Validate all fields.
    ///
    /// Returns a vector of (field_index, error_message) for invalid fields.
    pub fn validate(&mut self) -> Vec<(usize, String)> {
        let mut errors = Vec::new();
        for (i, field) in self.fields.iter_mut().enumerate() {
            if !field.validate() {
                if let Some(error) = field.validation_error() {
                    errors.push((i, error.to_string()));
                }
            }
        }
        errors
    }

    /// Check if all fields are valid.
    pub fn is_valid(&mut self) -> bool {
        self.validate().is_empty()
    }

    /// Clear all fields.
    pub fn clear(&mut self) {
        for field in &mut self.fields {
            field.clear();
        }
        self.focused_index = 0;
        if let Some(field) = self.fields.first_mut() {
            field.is_focused = true;
        }
    }

    /// Initialize focus on the first field.
    pub fn init_focus(&mut self) {
        // Unfocus all
        for field in &mut self.fields {
            field.is_focused = false;
        }
        // Focus first
        self.focused_index = 0;
        if let Some(field) = self.fields.first_mut() {
            field.is_focused = true;
        }
    }
}

/// Common validators for form fields.
pub mod validators {
    /// Validate that a field is not empty.
    pub fn required(value: &str) -> Option<String> {
        if value.trim().is_empty() {
            Some("This field is required".to_string())
        } else {
            None
        }
    }

    /// Validate that a field has at least 3 characters.
    pub fn min_length_3(value: &str) -> Option<String> {
        if value.len() < 3 {
            Some("Must be at least 3 characters".to_string())
        } else {
            None
        }
    }

    /// Validate that a field contains only alphanumeric characters and underscores.
    pub fn alphanumeric_underscore(value: &str) -> Option<String> {
        if value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            None
        } else {
            Some("Only letters, numbers, underscores, and hyphens allowed".to_string())
        }
    }

    /// Validate that a field looks like a valid path.
    pub fn valid_path(value: &str) -> Option<String> {
        if value.trim().is_empty() {
            return Some("Path cannot be empty".to_string());
        }
        // Basic validation - just check it doesn't contain null bytes
        if value.contains('\0') {
            return Some("Path contains invalid characters".to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_insert_char() {
        let mut field = FormField::new("Test");
        field.handle_key(KeyCode::Char('h'));
        field.handle_key(KeyCode::Char('i'));
        assert_eq!(field.value, "hi");
        assert_eq!(field.cursor, 2);
    }

    #[test]
    fn test_field_backspace() {
        let mut field = FormField::new("Test").with_value("hello");
        field.handle_key(KeyCode::Backspace);
        assert_eq!(field.value, "hell");
        assert_eq!(field.cursor, 4);
    }

    #[test]
    fn test_field_cursor_movement() {
        let mut field = FormField::new("Test").with_value("hello");
        field.handle_key(KeyCode::Home);
        assert_eq!(field.cursor, 0);
        field.handle_key(KeyCode::End);
        assert_eq!(field.cursor, 5);
        field.handle_key(KeyCode::Left);
        assert_eq!(field.cursor, 4);
        field.handle_key(KeyCode::Right);
        assert_eq!(field.cursor, 5);
    }

    #[test]
    fn test_form_navigation() {
        let mut form = Form::new()
            .add_field(FormField::new("Field 1"))
            .add_field(FormField::new("Field 2"));
        form.init_focus();

        assert_eq!(form.focused_index, 0);
        assert!(form.fields[0].is_focused);

        form.next_field();
        assert_eq!(form.focused_index, 1);
        assert!(!form.fields[0].is_focused);
        assert!(form.fields[1].is_focused);

        form.next_field();
        assert_eq!(form.focused_index, 0); // Wrap around
    }

    #[test]
    fn test_validator_required() {
        assert!(validators::required("").is_some());
        assert!(validators::required("   ").is_some());
        assert!(validators::required("value").is_none());
    }
}
