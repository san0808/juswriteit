use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Local};

/// Represents a note in the application
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub content: String,
}

impl Note {
    /// Create a new empty note with the given title
    pub fn new(title: &str) -> Result<Self, String> {
        let notes_dir = crate::utils::get_notes_dir();
        let file_path = notes_dir.join(format!("{}.md", title));
        
        // Create an empty file
        File::create(&file_path)
            .map_err(|e| format!("Failed to create note file: {}", e))?;
        
        Ok(Note {
            path: file_path,
            title: title.to_string(),
            content: String::new(),
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
        
        Ok(Note {
            path: path.to_path_buf(),
            title,
            content,
        })
    }
    
    /// Save the note content to its file
    pub fn save(&self) -> Result<(), String> {
        let mut file = File::create(&self.path)
            .map_err(|e| format!("Failed to create note file: {}", e))?;
        
        file.write_all(self.content.as_bytes())
            .map_err(|e| format!("Failed to write note content: {}", e))?;
        
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
                    match Note::load(&path) {
                        Ok(note) => notes.push(note),
                        Err(e) => eprintln!("Error loading note {:?}: {}", path, e),
                    }
                }
            }
        }
        
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
