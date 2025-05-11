use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use chrono::{DateTime, Local};

/// Represents a note in the application
#[derive(Clone)] // Keep Clone derive
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub content: String,
    pub modified_time: Option<SystemTime>, // Added modification time
}

impl Note {
    /// Create a new empty note with the given title
    pub fn new(title: &str) -> Result<Self, String> {
        let notes_dir = crate::utils::get_notes_dir();
        let file_path = notes_dir.join(format!("{}.md", title));

        // Create an empty file
        File::create(&file_path)
            .map_err(|e| format!("Failed to create note file: {}", e))?;

        // Get metadata after creation
        let metadata = fs::metadata(&file_path)
            .map_err(|e| format!("Failed to get metadata for new note: {}", e))?;
        let modified_time = metadata.modified().ok();

        Ok(Note {
            path: file_path,
            title: title.to_string(),
            content: String::new(),
            modified_time, // Store modification time
        })
    }

    /// Load a note from a file
    pub fn load(path: &Path) -> Result<Self, String> {
        // Get the filename without extension as the title
        let title = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("Invalid note filename: {:?}", path))?
            .to_string();

        // Read the file content
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open note file: {}", e))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read note content: {}", e))?;

        // Get metadata
        let metadata = fs::metadata(path)
            .map_err(|e| format!("Failed to get metadata for note: {}", e))?;
        let modified_time = metadata.modified().ok();

        Ok(Note {
            path: path.to_path_buf(),
            title,
            content,
            modified_time, // Store modification time
        })
    }

    /// Save the note content to its file
    pub fn save(&mut self) -> Result<(), String> { // Changed to &mut self
        let mut file = File::create(&self.path)
            .map_err(|e| format!("Failed to create note file: {}", e))?;

        file.write_all(self.content.as_bytes())
            .map_err(|e| format!("Failed to write note content: {}", e))?;

        // Update modification time after saving
        let metadata = fs::metadata(&self.path)
            .map_err(|e| format!("Failed to get metadata after saving: {}", e))?;
        self.modified_time = metadata.modified().ok();

        Ok(())
    }

    /// Delete this note
    pub fn delete(&self) -> Result<(), String> {
        fs::remove_file(&self.path)
            .map_err(|e| format!("Failed to delete note: {}", e))
    }

    /// Rename the note file and update the note's state
    pub fn rename(&mut self, new_title: &str) -> Result<(), String> {
        // Basic validation for the new title
        if new_title.trim().is_empty() {
            return Err("New title cannot be empty.".to_string());
        }
        // Add more validation if needed (e.g., disallowed characters)

        let notes_dir = crate::utils::get_notes_dir();
        let new_path = notes_dir.join(format!("{}.md", new_title));

        // Check if a note with the new title already exists
        if new_path.exists() && new_path != self.path {
            return Err(format!("A note named \"{}\" already exists.", new_title));
        }

        // Attempt to rename the file
        fs::rename(&self.path, &new_path)
            .map_err(|e| format!("Failed to rename note file: {}", e))?;

        // Update the note's state
        self.path = new_path;
        self.title = new_title.to_string();

        // Update modification time (optional, renaming might not update it)
        let metadata = fs::metadata(&self.path)
            .map_err(|e| format!("Failed to get metadata after renaming: {}", e))?;
        self.modified_time = metadata.modified().ok();

        Ok(())
    }

    /// Get all notes in the notes directory
    pub fn get_all() -> Result<Vec<Note>, String> {
        let notes_dir = crate::utils::get_notes_dir();
        let entries = fs::read_dir(&notes_dir)
            .map_err(|e| format!("Failed to read notes directory: {}", e))?;

        let mut notes = Vec::new();

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                // Only process .md files
                if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                    // Load the note which now includes metadata reading
                    match Note::load(&path) {
                        Ok(note) => notes.push(note),
                        Err(e) => eprintln!("Error loading note {:?}: {}", path, e),
                    }
                }
            }
        }

        // Sort notes by modification time, descending (most recent first)
        notes.sort_by(|a, b| {
            b.modified_time.cmp(&a.modified_time) // Reverse order
        });

        Ok(notes)
    }

    /// Generate a new unique note title with the current date
    pub fn generate_unique_title() -> String {
        let now: DateTime<Local> = Local::now();
        let formatted_date = now.format("%Y-%m-%d").to_string();

        let notes_dir = crate::utils::get_notes_dir();
        let mut note_number = 1;
        let mut title = format!("Note {}", formatted_date);

        // Check if file already exists, increment until we find a unique name
        while notes_dir.join(format!("{}.md", title)).exists() {
            note_number += 1;
            title = format!("Note {} ({})", formatted_date, note_number);
        }

        title
    }

    /// Check if a note is empty or nearly empty
    /// Considers notes with just whitespace or very few characters as empty.
    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }
    
    /// Update note title with today's date if it's empty and old
    /// Returns Ok(true) if the title was updated, Ok(false) otherwise.
    pub fn update_title_if_empty_and_old(&mut self) -> Result<bool, String> {
        // Only update empty notes
        if !self.is_empty() {
            return Ok(false);
        }
        
        // Check if the note's title contains a date that isn't today
        if let Some(modified_time) = self.modified_time {
            let modified_dt: DateTime<Local> = modified_time.into();
            let now = Local::now();
            
            // Check if it's from a previous day
            if modified_dt.date_naive() < now.date_naive() {
                // Generate a new title with today's date
                let new_title = Self::generate_unique_title();
                self.rename(&new_title)?; // Use existing rename logic
                return Ok(true); // Title was updated
            }
        }
        
        Ok(false) // Title was not updated
    }

    /// Generate title from the note's content automatically
    pub fn generate_title_from_content(&self) -> Option<String> {
        // Extract the first non-empty line
        let first_line = self.content
            .lines()
            .find(|line| !line.trim().is_empty())?;

        // Limit title length
        let mut title = first_line.trim();
        if title.len() > 50 {
            title = &title[0..47];
            return Some(format!("{}...", title));
        }
        
        // If title is too short, stick with date-based title
        if title.len() < 3 {
            return None;
        }
        
        Some(title.to_string())
    }

    /// Check if this note has a default date-based title
    pub fn has_default_title(&self) -> bool {
        self.title.starts_with("Note 20")
    }
}
