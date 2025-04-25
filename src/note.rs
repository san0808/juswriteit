use std::fs::{self, File}; // Removed Metadata
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
}
