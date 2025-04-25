use gtk::prelude::*; // Import common GTK traits
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label, 
          ListBox, ScrolledWindow, Box, TextView}; // Added TextView
use std::fs::{self, File}; // For directory creation, listing, and file operations
use std::io::Read; // For reading file content
use std::path::PathBuf; // For path manipulation

// Application ID (used by the system to identify the app)
// Follows reverse domain name notation
const APP_ID: &str = "com.example.juswriteit"; // Replace example.com with your domain or username

// Helper function to get the path to the notes directory
fn get_notes_dir() -> PathBuf {
    // glib::user_data_dir() returns a PathBuf directly
    let user_data_dir = glib::user_data_dir();
    
    // Join the path
    user_data_dir.join("juswriteit/notes")
}

// Function to ensure the notes directory exists
fn ensure_notes_dir_exists() -> Result<PathBuf, String> {
    let notes_dir = get_notes_dir();
    if !notes_dir.exists() {
        println!("Notes directory not found, creating at: {:?}", notes_dir);
        fs::create_dir_all(&notes_dir)
            .map_err(|e| format!("Failed to create notes directory {:?}: {}", notes_dir, e))?;
    }
    Ok(notes_dir)
}

fn main() -> glib::ExitCode {
    // Ensure the notes directory exists before starting the app
    if let Err(err) = ensure_notes_dir_exists() {
        eprintln!("Error initializing notes directory: {}", err);
        // Optionally show a graphical error dialog here later
        return glib::ExitCode::FAILURE; // Indicate failure
    }


    // Register resources - Placeholder for future icons, CSS, etc.
    // gio::resources_register_include!("compiled.gresource")
    //     .expect("Failed to register resources.");

    // Create a new GTK application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to the "activate" signal to build the UI when the app starts
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

// Function to build the user interface
fn build_ui(app: &Application) {
    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("juswriteit") // Initial window title
        .default_width(800)  // Sensible default size
        .default_height(600)
        // .icon_name("org.gtk.TextEditor") // Example: Use a standard icon for now
        .build();

    // Create a Paned widget (Horizontal orientation for left/right split)
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .wide_handle(true) // Makes the separator easier to grab
        .build();

    // Create a ListBox for the notes list
    let list_box = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single) // Allow only one selection
        .build();

    // Create a ScrolledWindow to contain the ListBox with scrolling
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling
        .min_content_width(200) // Minimum width
        .child(&list_box) // Add ListBox to ScrolledWindow
        .build();

    // Create a Box to hold the ScrolledWindow (for potential header/footer later)
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    left_pane.append(&scrolled_window);

    // Create the editor (TextView) for the right pane
    let text_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word) // Enable word wrapping
        .monospace(false) // Use proportional font (not monospace) for better readability
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    
    // Make the TextView editable
    text_view.set_editable(true);
    
    // Create a ScrolledWindow to contain the TextView with scrolling
    let editor_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never) // Disable horizontal scrolling (due to word wrap)
        .child(&text_view)
        .hexpand(true) // Allow to expand horizontally
        .vexpand(true) // Allow to expand vertically
        .build();

    // Add the panes to the Paned widget
    paned.set_start_child(Some(&left_pane));
    paned.set_end_child(Some(&editor_scrolled_window));

    // Set the initial position of the divider (e.g., 250 pixels from the left)
    paned.set_position(250);

    // Set the Paned widget as the child of the window
    window.set_child(Some(&paned));

    // Connect the row-selected signal to load note content
    list_box.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            // Get the row index
            let index = row.index();
            
            // Skip if it's a placeholder or error row 
            // (this assumes placeholder or error rows would be the only ones when no valid notes exist)
            if index < 0 {
                return;
            }
            
            // Get the note title from the row's child (Label)
            if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
                let title = label.label();
                
                // Build the full path to the note file
                let notes_dir = get_notes_dir();
                let file_path = notes_dir.join(format!("{}.md", title));
                
                // Load the content into the TextView
                if let Err(e) = load_note_content(&text_view, &file_path) {
                    eprintln!("Error loading note content: {}", e);
                    // Future: Show an error dialog or message
                }
            }
        } else {
            // No row selected, clear the TextView
            let buffer = text_view.buffer();
            buffer.set_text("");
        }
    });

    // Populate the notes list
    populate_notes_list(&list_box);

    // Present the window to the user
    window.present();
}

// Function to load note content from a file into a TextView
fn load_note_content(text_view: &TextView, file_path: &PathBuf) -> Result<(), String> {
    // Get the TextView's buffer (this directly returns a TextBuffer, not an Option)
    let buffer = text_view.buffer();
    
    // Read file content
    let mut file = File::open(file_path)
        .map_err(|e| format!("Failed to open file {}: {}", file_path.display(), e))?;
    
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| format!("Failed to read file {}: {}", file_path.display(), e))?;
    
    // Set content to buffer
    buffer.set_text(&content);
    
    Ok(())
}

// Function to populate the ListBox with notes
fn populate_notes_list(list_box: &ListBox) {
    // Get the notes directory
    let notes_dir = get_notes_dir();
    
    // Read the directory entries
    match fs::read_dir(&notes_dir) {
        Ok(entries) => {
            let mut found_notes = false;
            
            // Process each entry
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    
                    // Check if it's a .md file
                    if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                        found_notes = true;
                        
                        // Get the filename without the .md extension as the note title
                        if let Some(filename) = path.file_stem() {
                            if let Some(title) = filename.to_str() {
                                // Create a label with the note title
                                let label = Label::builder()
                                    .label(title)
                                    .xalign(0.0) // Left-align text
                                    .margin_start(5)
                                    .margin_end(5)
                                    .margin_top(5)
                                    .margin_bottom(5)
                                    .build();
                                
                                // Create a ListBoxRow and add the label to it
                                let row = gtk::ListBoxRow::new();
                                row.set_child(Some(&label));
                                
                                // Add the row to the ListBox
                                list_box.append(&row);
                            }
                        }
                    }
                }
            }
            
            // If no notes were found, show a placeholder message
            if !found_notes {
                let label = Label::builder()
                    .label("No notes yet. Create one!")
                    .xalign(0.0)
                    .margin_start(5)
                    .margin_end(5)
                    .margin_top(5)
                    .margin_bottom(5)
                    .build();
                
                let row = gtk::ListBoxRow::new();
                row.set_child(Some(&label));
                row.set_sensitive(false); // Make it non-selectable
                list_box.append(&row);
            }
        },
        Err(e) => {
            eprintln!("Error reading notes directory: {}", e);
            // Add an error message to the list
            let label = Label::builder()
                .label("Error loading notes")
                .xalign(0.0)
                .margin_start(5)
                .margin_end(5)
                .margin_top(5)
                .margin_bottom(5)
                .build();
            
            let row = gtk::ListBoxRow::new();
            row.set_child(Some(&label));
            list_box.append(&row);
        }
    }
}
