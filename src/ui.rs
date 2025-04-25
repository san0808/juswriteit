use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label,
          ListBox, ScrolledWindow, Box, TextView, HeaderBar, Button,
          EventControllerKey}; // Removed ResponseType
use glib::clone;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;

use crate::note::Note;
use crate::utils::{show_error_dialog, show_confirmation_dialog, schedule_auto_save};

// Struct to handle active note state
struct ActiveNote {
    path: PathBuf,
    title: String,
    // Flag to track if there are unsaved changes
    has_changes: bool,
    // ID of the scheduled auto-save operation (if any)
    auto_save_source_id: Option<glib::SourceId>,
}

// Auto-save delay in milliseconds
const AUTO_SAVE_DELAY_MS: u32 = 2000; // 2 seconds

/// Build the user interface
pub fn build_ui(app: &Application) {
    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("juswriteit")
        .default_width(800)
        .default_height(600)
        .build();

    // Create a HeaderBar
    let header_bar = HeaderBar::builder()
        .show_title_buttons(true)
        .title_widget(&Label::new(Some("juswriteit")))
        .build();
    
    // Create buttons for the header bar
    let new_note_button = Button::builder()
        .label("New Note")
        .tooltip_text("Create a new note")
        .build();
    
    let delete_note_button = Button::builder()
        .label("Delete")
        .tooltip_text("Delete current note")
        .sensitive(false) // Initially disabled until a note is selected
        .build();
    
    // Add buttons to the HeaderBar
    header_bar.pack_start(&new_note_button);
    header_bar.pack_end(&delete_note_button);
    
    // Set the HeaderBar as the window's titlebar
    window.set_titlebar(Some(&header_bar));

    // Create a Paned widget (Horizontal orientation for left/right split)
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .wide_handle(true)
        .build();

    // Create a ListBox for the notes list
    let list_box = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .build();

    // Create a ScrolledWindow to contain the ListBox with scrolling
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(200)
        .build();
    
    scrolled_window.set_child(Some(&list_box));

    // Create a Box to hold the ScrolledWindow
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    left_pane.append(&scrolled_window);

    // Create the editor (TextView) for the right pane
    let text_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word)
        .monospace(false)
        .margin_start(12)
        .margin_end(12)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    
    text_view.set_editable(true);
    
    // Create a ScrolledWindow to contain the TextView with scrolling
    let editor_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .vexpand(true)
        .build();
    
    editor_scrolled_window.set_child(Some(&text_view));

    // Create a status label for the bottom of the window
    let status_label = Label::builder()
        .label("Ready")
        .xalign(0.0)
        .margin_start(10)
        .margin_end(10)
        .margin_top(5)
        .margin_bottom(5)
        .build();
    
    // Create a Box for the right side (editor and status)
    let right_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .build();
    
    right_pane.append(&editor_scrolled_window);
    right_pane.append(&status_label);

    // Create a shared variable to track the active note
    let active_note: Rc<RefCell<Option<ActiveNote>>> = Rc::new(RefCell::new(None));
    
    // Enable/disable delete button based on selection
    let delete_button_ref = delete_note_button.clone();
    let active_note_for_button = active_note.clone();
    
    // Update UI based on selection state
    let update_ui_for_selection = move || {
        let has_selection = active_note_for_button.borrow().is_some();
        delete_button_ref.set_sensitive(has_selection);
    };
    
    // Clone for row selection handler
    let text_view_for_loading = text_view.clone();
    let active_note_for_loading = active_note.clone();
    let update_ui_for_selection_on_load = update_ui_for_selection.clone();
    let window_for_loading = window.clone();
    let status_label_for_loading = status_label.clone();

    // Connect row-selected signal to load note content
    list_box.connect_row_selected(move |_, row| {
        // Cancel any pending auto-save
        if let Some(active) = active_note_for_loading.borrow_mut().as_mut() {
            if let Some(source_id) = active.auto_save_source_id.take() {
                // Ignore the result of remove()
                let _ = source_id.remove();
            }
        }

        if let Some(row) = row {
            // Get the row index
            let index = row.index();
            
            // Skip if it's a placeholder or error row 
            if index < 0 {
                *active_note_for_loading.borrow_mut() = None;
                update_ui_for_selection_on_load();
                return;
            }
            
            // Get the note title from the row's child (Label)
            if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
                let title = label.label();
                
                // Build the full path to the note file
                let notes_dir = crate::utils::get_notes_dir();
                let file_path = notes_dir.join(format!("{}.md", title));
                
                // Load the note content
                match Note::load(&file_path) {
                    Ok(note) => {
                        // Update the TextView
                        text_view_for_loading.buffer().set_text(&note.content);
                        
                        // Update the active note
                        *active_note_for_loading.borrow_mut() = Some(ActiveNote {
                            path: file_path,
                            title: title.to_string(),
                            has_changes: false,
                            auto_save_source_id: None,
                        });
                        
                        // Update window title
                        window_for_loading.set_title(Some(&format!("{} - juswriteit", title)));
                        
                        // Update status label
                        let word_count = count_words(&note.content);
                        status_label_for_loading.set_text(&format!("{} words", word_count));
                    },
                    Err(e) => {
                        eprintln!("Error loading note content: {}", e);
                        text_view_for_loading.buffer().set_text("");
                        window_for_loading.set_title(Some("juswriteit"));
                        status_label_for_loading.set_text("Error loading note");
                        *active_note_for_loading.borrow_mut() = None;
                    }
                }
            }
        } else {
            // No row selected, clear the TextView
            text_view_for_loading.buffer().set_text("");
            
            // Clear the active note
            *active_note_for_loading.borrow_mut() = None;
            
            // Reset window title and status
            window_for_loading.set_title(Some("juswriteit"));
            status_label_for_loading.set_text("Ready");
        }
        
        // Update UI state based on selection
        update_ui_for_selection_on_load();
    });

    // Connect to the "changed" signal of the text buffer for auto-save
    let buffer = text_view.buffer();
    let active_note_for_changes = active_note.clone();
    let text_view_for_changes = text_view.clone();
    let window_for_changes = window.clone();
    let status_label_for_changes = status_label.clone();

    // Fix the unused variable warning by adding underscore prefix
    buffer.connect_changed(move |_| {
        let mut active_note_guard = active_note_for_changes.borrow_mut();
        
        if let Some(active) = active_note_guard.as_mut() {
            // Mark as having unsaved changes
            active.has_changes = true;
            
            // Update status label to show unsaved state
            let content = text_view_for_changes.buffer().text(
                &text_view_for_changes.buffer().start_iter(),
                &text_view_for_changes.buffer().end_iter(),
                false
            ).to_string();
            
            let word_count = count_words(&content);
            status_label_for_changes.set_text(&format!("{} words (unsaved)", word_count));
            
            // Cancel existing auto-save timer if there is one
            if let Some(source_id) = active.auto_save_source_id.take() {
                // Ignore the result of remove()
                let _ = source_id.remove();
            }
            
            // Schedule a new auto-save
            let note_path = active.path.clone();
            // Rename to add underscore prefix to unused variable
            let _window_ref = window_for_changes.clone();
            let text_view_ref = text_view_for_changes.clone();
            let active_note_ref = active_note_for_changes.clone();
            let status_label_ref = status_label_for_changes.clone();
            
            active.auto_save_source_id = Some(schedule_auto_save(AUTO_SAVE_DELAY_MS, move || {
                // Get current content
                let buffer = text_view_ref.buffer();
                let content = buffer.text(
                    &buffer.start_iter(),
                    &buffer.end_iter(),
                    false
                ).to_string();
                
                // Create a note object and save it
                let note = Note {
                    path: note_path.clone(),
                    title: note_path.file_stem().unwrap_or_default()
                              .to_string_lossy().to_string(),
                    content,
                };
                
                match note.save() {
                    Ok(_) => {
                        // Update status label
                        let word_count = count_words(&note.content);
                        status_label_ref.set_text(&format!("{} words (auto-saved)", word_count));
                        
                        // Mark as not having unsaved changes
                        if let Some(active) = active_note_ref.borrow_mut().as_mut() {
                            active.has_changes = false;
                            // Set the ID to None *within the timer callback*
                            active.auto_save_source_id = None;
                        }
                        
                        // Schedule status update back to normal after delay
                        let status_label_clone = status_label_ref.clone();
                        let content_clone = note.content.clone();
                        glib::timeout_add_seconds_local(3, move || {
                            let word_count = count_words(&content_clone);
                            status_label_clone.set_text(&format!("{} words", word_count));
                            glib::ControlFlow::Break
                        });
                    },
                    Err(e) => {
                        eprintln!("Auto-save error: {}", e);
                        status_label_ref.set_text("Auto-save failed");
                        // Could show a more subtle error indication here
                    }
                }
            }));
        }
    });

    // Add keyboard shortcut handling (Ctrl+S for manual save)
    let key_controller = EventControllerKey::new();
    
    // Clone what we need for the closure
    let text_view_for_save = text_view.clone();
    let active_note_for_save = active_note.clone();
    let window_for_save = window.clone();
    let status_label_for_save = status_label.clone();
    
    key_controller.connect_key_pressed(clone!(@strong window_for_save, @strong text_view_for_save, @strong active_note_for_save, @strong status_label_for_save => move |_, key, _keycode, state| {
        // Check for Ctrl+S
        if key == gtk::gdk::Key::s && state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
            let mut active_note_guard = active_note_for_save.borrow_mut();

            if let Some(active) = active_note_guard.as_mut() {
                // Cancel any pending auto-save
                if let Some(source_id) = active.auto_save_source_id.take() {
                    // Ignore the result of remove()
                    let _ = source_id.remove();
                }

                // Get current content
                let buffer = text_view_for_save.buffer();
                let content = buffer.text(
                    &buffer.start_iter(),
                    &buffer.end_iter(),
                    false
                ).to_string();
                
                // Create a note object and save it
                let note = Note {
                    path: active.path.clone(),
                    title: active.title.clone(),
                    content,
                };
                
                match note.save() {
                    Ok(_) => {
                        // Mark as not having unsaved changes
                        active.has_changes = false;
                        
                        // Update status label
                        let word_count = count_words(&note.content);
                        status_label_for_save.set_text(&format!("{} words (saved)", word_count));
                        
                        // Schedule status update back to normal after delay
                        let status_label_clone = status_label_for_save.clone();
                        let content_clone = note.content.clone();
                        glib::timeout_add_seconds_local(3, move || {
                            let word_count = count_words(&content_clone);
                            status_label_clone.set_text(&format!("{} words", word_count));
                            glib::ControlFlow::Break
                        });
                    },
                    Err(e) => {
                        eprintln!("Error saving note: {}", e);
                        show_error_dialog(&window_for_save, "Save Error", &format!("Failed to save note: {}", e));
                    }
                }
            }
            
            // Return true to indicate we handled the key press
            return glib::Propagation::Stop;
        }
        
        // Let other handlers process the key press
        glib::Propagation::Proceed
    }));
    
    // Add the controller to the window
    window.add_controller(key_controller);
    
    // Connect the "New Note" button
    let list_box_for_new = list_box.clone();
    let active_note_for_new = active_note.clone();
    let text_view_for_new = text_view.clone();
    let window_for_new = window.clone();
    let status_label_for_new = status_label.clone();
    
    new_note_button.connect_clicked(move |_| {
        // Generate a unique title for the new note
        let title = Note::generate_unique_title();
        
        // Create a new note
        match Note::new(&title) {
            Ok(note) => {
                // Clear the editor
                text_view_for_new.buffer().set_text("");
                
                // Update the active note
                *active_note_for_new.borrow_mut() = Some(ActiveNote {
                    path: note.path.clone(),
                    title: note.title.clone(),
                    has_changes: false,
                    auto_save_source_id: None,
                });
                
                // Update window title
                window_for_new.set_title(Some(&format!("{} - juswriteit", title)));
                
                // Update status
                status_label_for_new.set_text("0 words");
                
                // Refresh the list box
                refresh_note_list(&list_box_for_new);
                
                // Find and select the newly created note
                select_note_by_title(&list_box_for_new, &title);
            },
            Err(e) => {
                eprintln!("Error creating new note: {}", e);
                show_error_dialog(&window_for_new, "Create Error", &format!("Failed to create new note: {}", e));
            }
        }
    });
    
    // Connect the "Delete" button
    let list_box_for_delete = list_box.clone();
    let active_note_for_delete = active_note.clone();
    let window_for_delete = window.clone();
    let text_view_for_delete = text_view.clone();
    let status_label_for_delete = status_label.clone();
    
    delete_note_button.connect_clicked(move |_| {
        let active_note_guard = active_note_for_delete.borrow();
        
        if let Some(active) = active_note_guard.as_ref() {
            let path_clone = active.path.clone();
            let title_clone = active.title.clone();
            
            // Confirmation dialog
            show_confirmation_dialog(
                &window_for_delete,
                "Confirm Deletion",
                &format!("Delete note \"{}\"?", title_clone),
                "This action cannot be undone.",
                clone!(@strong active_note_for_delete, @strong list_box_for_delete, 
                      @strong text_view_for_delete, @strong window_for_delete, 
                      @strong status_label_for_delete, @strong path_clone => move || {
                    
                    // Create a Note object for the delete operation
                    let note = Note {
                        path: path_clone.clone(),
                        title: path_clone.file_stem().unwrap_or_default()
                                  .to_string_lossy().to_string(),
                        content: String::new() // Not needed for deletion
                    };
                    
                    match note.delete() {
                        Ok(_) => {
                            // Clear the active note
                            *active_note_for_delete.borrow_mut() = None;
                            
                            // Clear the editor
                            text_view_for_delete.buffer().set_text("");
                            
                            // Reset window title
                            window_for_delete.set_title(Some("juswriteit"));
                            
                            // Reset status
                            status_label_for_delete.set_text("Ready");
                            
                            // Refresh the list box
                            refresh_note_list(&list_box_for_delete);
                        },
                        Err(e) => {
                            eprintln!("Error deleting note: {}", e);
                            show_error_dialog(&window_for_delete, "Delete Error", 
                                             &format!("Failed to delete note: {}", e));
                        }
                    }
                })
            );
        }
    });

    // Add the panes to the Paned widget
    paned.set_start_child(Some(&left_pane));
    paned.set_end_child(Some(&right_pane));

    // Set the initial position of the divider
    paned.set_position(250);

    // Set the Paned widget as the child of the window
    window.set_child(Some(&paned));

    // Populate the notes list
    refresh_note_list(&list_box);

    // Present the window to the user
    window.present();
}

/// Count words in text
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Refresh the note list
fn refresh_note_list(list_box: &ListBox) {
    // Remove all existing rows
    while let Some(row) = list_box.row_at_index(0) {
        list_box.remove(&row);
    }
    
    // Get all notes
    match Note::get_all() {
        Ok(notes) => {
            let mut found_notes = false;
            
            // Add each note to the list
            for note in notes {
                found_notes = true;
                
                // Create a label with the note title
                let label = Label::builder()
                    .label(&note.title)
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
            eprintln!("Error reading notes: {}", e);
            
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

/// Find and select a note by its title
fn select_note_by_title(list_box: &ListBox, title: &str) {
    let mut row_index = 0;
    
    while let Some(row) = list_box.row_at_index(row_index) {
        if let Some(label) = row.child().and_then(|w| w.downcast::<Label>().ok()) {
            if label.label() == title {
                list_box.select_row(Some(&row));
                row.grab_focus();
                return;
            }
        }
        
        row_index += 1;
    }
}
