use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, Paned, Orientation, Label,
          ListBox, ScrolledWindow, Box, TextView, HeaderBar, Button,
          EventControllerKey, CssProvider};
use glib::clone;
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use chrono::{DateTime, Local};

use crate::note::Note;
use crate::utils::{show_error_dialog, show_confirmation_dialog, schedule_auto_save};

// Struct to handle active note state
// #[derive(Clone)] // Remove derive Clone
struct ActiveNote {
    path: PathBuf,
    title: String,
    has_changes: bool,
    auto_save_source_id: Option<glib::SourceId>,
    note: Note,
}

// Manual implementation of Clone for ActiveNote
impl Clone for ActiveNote {
    fn clone(&self) -> Self {
        ActiveNote {
            path: self.path.clone(),
            title: self.title.clone(),
            has_changes: self.has_changes,
            auto_save_source_id: None, // SourceId cannot be cloned, set to None
            note: self.note.clone(),
        }
    }
}


// Auto-save delay in milliseconds
const AUTO_SAVE_DELAY_MS: u32 = 2000; // 2 seconds

/// Build the user interface
pub fn build_ui(app: &Application) {
    // Load CSS for styling
    load_css();
    
    // Create the main application window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("juswriteit")
        .default_width(1000)
        .default_height(700)
        .css_classes(vec!["dark-mode"]) // Start with dark mode by default
        .build();

    // Create a HeaderBar
    let header_bar = HeaderBar::builder()
        .show_title_buttons(true)
        .css_classes(vec!["header-bar"])
        .build();
    
    // Add app title to the headerbar
    let app_title = Label::builder()
        .label("Just Write")
        .css_classes(vec!["app-title"])
        .build();
    header_bar.set_title_widget(Some(&app_title));

    // Create buttons for the header bar with symbolic icons
    let new_note_button = Button::builder()
        .icon_name("document-new-symbolic")
        .tooltip_text("New Note")
        .css_classes(vec!["header-button"])
        .build();

    let rename_note_button = Button::builder()
        .icon_name("document-edit-symbolic")
        .tooltip_text("Rename Note")
        .sensitive(false) // Initially disabled
        .css_classes(vec!["header-button"])
        .build();

    let delete_note_button = Button::builder()
        .icon_name("user-trash-symbolic")
        .tooltip_text("Delete Note")
        .sensitive(false)
        .css_classes(vec!["header-button"])
        .build();
        
    // Add theme toggle button
    let theme_toggle_button = Button::builder()
        .icon_name("weather-clear-night-symbolic")
        .tooltip_text("Toggle Light/Dark Theme")
        .css_classes(vec!["header-button", "theme-toggle"])
        .build();
    
    // Add theme toggle functionality
    let window_for_theme = window.clone();
    theme_toggle_button.connect_clicked(move |button| {
        if window_for_theme.has_css_class("dark-mode") {
            window_for_theme.remove_css_class("dark-mode");
            window_for_theme.add_css_class("light-mode");
            button.set_icon_name("weather-clear-symbolic");
        } else {
            window_for_theme.remove_css_class("light-mode");
            window_for_theme.add_css_class("dark-mode");
            button.set_icon_name("weather-clear-night-symbolic");
        }
    });

    // Add buttons to the HeaderBar
    header_bar.pack_start(&new_note_button);
    header_bar.pack_start(&rename_note_button);
    header_bar.pack_start(&delete_note_button);
    header_bar.pack_end(&theme_toggle_button);

    // Set the HeaderBar as the window's titlebar
    window.set_titlebar(Some(&header_bar));

    // Create a Paned widget (Horizontal orientation for left/right split)
    let paned = Paned::builder()
        .orientation(Orientation::Horizontal)
        .wide_handle(false) // Slimmer handle
        .css_classes(vec!["main-pane"])
        .build();

    // Create a ListBox for the notes list with improved styling
    let list_box = ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(vec!["notes-list"])
        .vexpand(true)
        .build();

    // Add a "Notes" header to the sidebar
    let sidebar_header = Label::builder()
        .label("NOTES")
        .xalign(0.0)
        .css_classes(vec!["sidebar-header"])
        .build();

    // Create a ScrolledWindow to contain the ListBox with scrolling
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .css_classes(vec!["sidebar-scroll"])
        .build();
    
    scrolled_window.set_child(Some(&list_box));

    // Create a Box to hold the sidebar components
    let left_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["sidebar"])
        .width_request(250) // Fixed sidebar width
        .build();
    
    // Add the sidebar header and scrolled window to the left pane
    left_pane.append(&sidebar_header);
    left_pane.append(&scrolled_window);

    // Improve the editor (TextView) for a more minimalist look
    let text_view = TextView::builder()
        .wrap_mode(gtk::WrapMode::Word)
        .monospace(false)
        .css_classes(vec!["editor"])
        .hexpand(true)
        .vexpand(true)
        .build();
    
    // Configure buffer with some initial settings
    let buffer = text_view.buffer();
    buffer.set_text("");
    
    // Create a ScrolledWindow to contain the TextView with scrolling
    let editor_scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .hexpand(true)
        .vexpand(true)
        .css_classes(vec!["editor-scroll"])
        .build();
    
    editor_scrolled_window.set_child(Some(&text_view));

    // Create a status label for the bottom of the window
    let status_label = Label::builder()
        .label("Ready")
        .xalign(0.0)
        .hexpand(true)
        .css_classes(vec!["status-text"])
        .build();
    
    // Create a word count label
    let word_count_label = Label::builder()
        .label("0 words")
        .xalign(1.0)
        .css_classes(vec!["word-count"])
        .build();
    
    // Create a Box for the status bar
    let status_box = Box::builder()
        .orientation(Orientation::Horizontal)
        .css_classes(vec!["status-bar"])
        .build();
    
    status_box.append(&status_label);
    status_box.append(&word_count_label);
    
    // Create a Box for the right side (editor and status)
    let right_pane = Box::builder()
        .orientation(Orientation::Vertical)
        .css_classes(vec!["editor-area"])
        .build();
    
    right_pane.append(&editor_scrolled_window);
    right_pane.append(&status_box);

    // Create a shared variable to track the active note
    let active_note: Rc<RefCell<Option<ActiveNote>>> = Rc::new(RefCell::new(None));
    
    // Enable/disable buttons based on selection
    let delete_button_ref = delete_note_button.clone();
    let rename_button_ref = rename_note_button.clone();
    let active_note_for_button = active_note.clone();
    
    // Update UI based on selection state
    let update_ui_for_selection = move || {
        let has_selection = active_note_for_button.borrow().is_some();
        delete_button_ref.set_sensitive(has_selection);
        rename_button_ref.set_sensitive(has_selection);
    };
    
    // Clone for row selection handler
    let text_view_for_loading = text_view.clone();
    let active_note_for_loading = active_note.clone();
    let update_ui_for_selection_on_load = update_ui_for_selection.clone();
    let window_for_loading = window.clone();
    let status_label_for_loading = status_label.clone();

    // Connect row-selected signal to load note content
    list_box.connect_row_selected({
        let active_note_for_loading = active_note.clone();
        let text_view_for_loading = text_view.clone();
        let window_for_loading = window.clone();
        let status_label_for_loading = status_label.clone();
        let update_ui_for_selection_on_load = update_ui_for_selection.clone();
        move |_listbox, row_opt| {
            // Cancel any pending auto-save
            // --- FIX: Only borrow mutably for the minimum time needed ---
            let mut clear_auto_save = false;
            {
                let mut active = active_note_for_loading.borrow_mut();
                if let Some(active) = active.as_mut() {
                    if active.auto_save_source_id.is_some() {
                        clear_auto_save = true;
                    }
                }
            }
            if clear_auto_save {
                // Now borrow mutably again just to clear the source_id
                if let Some(active) = active_note_for_loading.borrow_mut().as_mut() {
                    if let Some(source_id) = active.auto_save_source_id.take() {
                        let _ = source_id.remove();
                    }
                }
            }
            // --- END FIX ---

            if let Some(row) = row_opt {
                // Get the note title from the custom widget inside the row
                let title = row.child()
                    .and_then(|child_box| child_box.downcast::<Box>().ok())
                    .and_then(|hbox| hbox.first_child()) // Get the first child (title label)
                    .and_then(|widget| widget.downcast::<Label>().ok())
                    .map(|label| label.label().to_string());

                if let Some(title) = title {
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
                                note: note.clone(), // Store the loaded note
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
                } else {
                     // Handle case where title couldn't be extracted (e.g., placeholder row)
                    *active_note_for_loading.borrow_mut() = None;
                    text_view_for_loading.buffer().set_text("");
                    window_for_loading.set_title(Some("juswriteit"));
                    status_label_for_loading.set_text("Ready");
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
        }
    });

    // Update the buffer connect_changed handler to update word count and status
    let buffer = text_view.buffer();
    let active_note_for_changes = active_note.clone();
    let text_view_for_changes = text_view.clone();
    let status_label_for_changes = status_label.clone(); // Fix: clone status_label
    let word_count_label_for_changes = word_count_label.clone();

    buffer.connect_changed(move |_| {
        let mut active_note_guard = active_note_for_changes.borrow_mut();

        if let Some(active) = active_note_guard.as_mut() {
            // Mark as having unsaved changes
            active.has_changes = true;
            
            // Update the content in the active note object immediately
            let content = text_view_for_changes.buffer().text(
                &text_view_for_changes.buffer().start_iter(),
                &text_view_for_changes.buffer().end_iter(),
                false
            ).to_string();
            active.note.content = content.clone(); // Update content in the stored note

            // Update status label to show unsaved state
            let word_count = count_words(&content);
            status_label_for_changes.set_text("Editing..."); // Show editing status
            word_count_label_for_changes.set_text(&format!("{} words", word_count));
            
            // Cancel existing auto-save timer if there is one
            if let Some(source_id) = active.auto_save_source_id.take() {
                let _ = source_id.remove();
            }

            // Schedule a new auto-save
            let mut note_to_save = active.note.clone();
            // Use glib::clone! to capture variables correctly for the inner closure
            let active_note_ref = active_note_for_changes.clone();
            let status_label_ref = status_label_for_changes.clone(); // Use the correct reference

            active.auto_save_source_id = Some(schedule_auto_save(AUTO_SAVE_DELAY_MS, clone!(@strong active_note_ref, @strong status_label_ref, @strong word_count_label_for_changes => move || {
                // The content is already updated in note_to_save
                match note_to_save.save() {
                    Ok(_) => {
                        // Update status label
                        let word_count = count_words(&note_to_save.content);
                        status_label_ref.set_text("Auto-saved"); // Show auto-save status
                        word_count_label_for_changes.set_text(&format!("{} words", word_count));

                        // Mark as not having unsaved changes and clear timer ID
                        if let Some(active_inner) = active_note_ref.borrow_mut().as_mut() {
                            active_inner.has_changes = false;
                            active_inner.auto_save_source_id = None;
                            active_inner.note.modified_time = note_to_save.modified_time;
                        }

                        // Schedule status update back to normal after delay
                        let status_label_clone = status_label_ref.clone();
                        glib::timeout_add_seconds_local(3, move || {
                            status_label_clone.set_text("Ready");
                            glib::ControlFlow::Break
                        });
                    },
                    Err(e) => {
                        eprintln!("Auto-save error: {}", e);
                        status_label_ref.set_text("Auto-save failed");
                    }
                }
            })));
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

                // Get current content and update the active note object
                let buffer = text_view_for_save.buffer();
                let content = buffer.text(
                    &buffer.start_iter(),
                    &buffer.end_iter(),
                    false
                ).to_string();
                active.note.content = content; // Update content in the stored note

                // Save the note
                match active.note.save() { // Save the note stored in active state
                    Ok(_) => {
                        // Mark as not having unsaved changes
                        active.has_changes = false;
                        
                        // Update status label
                        let word_count = count_words(&active.note.content);
                        status_label_for_save.set_text(&format!("{} words (saved)", word_count));
                        
                        // Schedule status update back to normal after delay
                        let status_label_clone = status_label_for_save.clone();
                        let content_clone = active.note.content.clone();
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
                    note: note.clone(), // Store the new note
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

    // Connect the "Rename" button
    let list_box_for_rename = list_box.clone();
    let active_note_for_rename = active_note.clone();
    let window_for_rename = window.clone();

    // Use clone! for the outer closure
    rename_note_button.connect_clicked(clone!(@strong window_for_rename, @strong active_note_for_rename, @strong list_box_for_rename => move |_| {
        // Borrow immutably first to get the title
        let current_title = active_note_for_rename.borrow().as_ref().map(|a| a.title.clone());

        // Only proceed if there is an active note and we got a title
        if let Some(current_title) = current_title {
            // Show rename dialog, passing owned String
            // Use clone! again for the inner closure (on_confirm)
            // No mutable borrow is held here anymore
            show_rename_dialog(
                &window_for_rename,
                current_title, // Pass owned String
                clone!(@strong active_note_for_rename, @strong window_for_rename, @strong list_box_for_rename => move |new_title| {
                    // This closure is called when the user confirms the rename
                    let rename_result = { // Create a scope for the mutable borrow
                        let mut active_guard = active_note_for_rename.borrow_mut();
                        if let Some(active_inner) = active_guard.as_mut() {
                            match active_inner.note.rename(&new_title) {
                                Ok(_) => {
                                    // Update active note title within the borrow
                                    active_inner.title = new_title.clone();
                                    // Update window title
                                    window_for_rename.set_title(Some(&format!("{} - juswriteit", new_title)));
                                    Ok(()) // Indicate success
                                },
                                Err(e) => {
                                    eprintln!("Error renaming note: {}", e);
                                    Err(e) // Propagate error
                                }
                            }
                        } else {
                            Err("No active note found during rename confirmation.".to_string())
                        }
                        // active_guard is dropped here, releasing the mutable borrow
                    };

                    // Handle result outside the borrow scope
                    match rename_result {
                        Ok(_) => {
                            // Refresh list and reselect *after* borrow is released
                            refresh_note_list(&list_box_for_rename);
                            select_note_by_title(&list_box_for_rename, &new_title);
                        },
                        Err(e) => {
                            // Pass cloned window to error dialog
                            show_error_dialog(&window_for_rename, "Rename Error", &e);
                        }
                    }
                })
            );
        }
    }));

    // Connect the "Delete" button
    let list_box_for_delete = list_box.clone();
    let active_note_for_delete = active_note.clone();
    let window_for_delete = window.clone();
    let text_view_for_delete = text_view.clone();
    let status_label_for_delete = status_label.clone();
    
    delete_note_button.connect_clicked(move |_| {
        let active_note_guard = active_note_for_delete.borrow();
        
        if let Some(active) = active_note_guard.as_ref() {
            let note_to_delete = active.note.clone(); // Clone the note to delete
            let title_clone = active.title.clone();
            
            // Confirmation dialog
            show_confirmation_dialog(
                &window_for_delete,
                "Confirm Deletion",
                &format!("Delete note \"{}\"?", title_clone),
                "This action cannot be undone.",
                clone!(@strong active_note_for_delete, @strong list_box_for_delete,
                      @strong text_view_for_delete, @strong window_for_delete,
                      @strong status_label_for_delete, @strong note_to_delete => move || {

                    match note_to_delete.delete() { // Delete the cloned note
                        Ok(_) => {
                            // Clear the active note if it's the one we just deleted
                            let mut active_guard = active_note_for_delete.borrow_mut();
                            if active_guard.as_ref().map_or(false, |a| a.path == note_to_delete.path) {
                                *active_guard = None;
                                // Clear the editor
                                text_view_for_delete.buffer().set_text("");
                                // Reset window title
                                window_for_delete.set_title(Some("juswriteit"));
                                // Reset status
                                status_label_for_delete.set_text("Ready");
                            }
                            drop(active_guard); // Release borrow before refreshing list

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

    // Set the initial position of the divider (adjusted for improved layout)
    paned.set_position(260);

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

/// Refresh the note list with improved styling
fn refresh_note_list(list_box: &ListBox) {
    // Remove all existing rows
    while let Some(row) = list_box.row_at_index(0) {
        list_box.remove(&row);
    }

    // Get all notes (already sorted by Note::get_all)
    match Note::get_all() {
        Ok(notes) => {
            let mut found_notes = false;

            // Add each note to the list
            for note in notes {
                found_notes = true;

                // Create labels for title, date, and preview
                let title_label = Label::builder()
                    .label(&note.title)
                    .xalign(0.0) // Left-align text
                    .css_classes(vec!["note-title"]) // Add CSS class
                    .halign(gtk::Align::Start)
                    .build();

                // Format date as "Mon DD"
                let date_str = note.modified_time
                    .map(|st| {
                        let dt: DateTime<Local> = st.into();
                        dt.format("%b %d").to_string() // e.g., Apr 21
                    })
                    .unwrap_or_else(|| "-".to_string());

                let date_label = Label::builder()
                    .label(&date_str)
                    .xalign(0.0) // Left-align text
                    .css_classes(vec!["note-date", "dim-label"]) // Add CSS classes
                    .halign(gtk::Align::Start)
                    .build();

                // Create shorter preview text
                let preview_text = if note.content.is_empty() {
                    "Empty".to_string()
                } else {
                    note.content
                        .split_whitespace()
                        .take(5) // Take fewer words for cleaner look
                        .collect::<Vec<&str>>()
                        .join(" ") + "..."
                };

                let preview_label = Label::builder()
                    .label(&preview_text)
                    .xalign(0.0) // Left-align text
                    .css_classes(vec!["note-preview", "dim-label"]) // Add CSS classes
                    .halign(gtk::Align::Start)
                    .build();

                // Create a Vertical Box to hold the labels with improved spacing
                let row_box = Box::builder()
                    .orientation(Orientation::Vertical) 
                    .spacing(2) 
                    .margin_start(12) 
                    .margin_end(12)
                    .margin_top(8) 
                    .margin_bottom(8)
                    .css_classes(vec!["note-row-box"])
                    .build();

                row_box.append(&title_label);
                row_box.append(&date_label);
                row_box.append(&preview_label);

                // Create a ListBoxRow and add the Box to it
                let row = gtk::ListBoxRow::builder()
                    .css_classes(vec!["note-row"])
                    .build();
                    
                row.set_child(Some(&row_box));

                // Add the row to the ListBox
                list_box.append(&row);
            }

            // If no notes were found, show a placeholder message
            if !found_notes {
                let label = Label::builder()
                    .label("No notes yet. Create one!")
                    .xalign(0.0)
                    .margin_start(12)
                    .margin_end(12)
                    .margin_top(12)
                    .margin_bottom(12)
                    .css_classes(vec!["no-notes-label"])
                    .build();
                
                let row = gtk::ListBoxRow::builder()
                    .css_classes(vec!["empty-note-row"])
                    .build();
                    
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
                .margin_start(12)
                .margin_end(12)
                .margin_top(12)
                .margin_bottom(12)
                .css_classes(vec!["error-label"])
                .build();
            
            let row = gtk::ListBoxRow::builder()
                .css_classes(vec!["error-row"])
                .build();
                
            row.set_child(Some(&label));
            list_box.append(&row);
        }
    }
}

/// Find and select a note by its title
fn select_note_by_title(list_box: &ListBox, title_to_find: &str) {
    let mut row_index = 0;

    while let Some(row) = list_box.row_at_index(row_index) {
        // Find the title label within the row's Box (now vertical)
        let title_label_opt = row.child()
            .and_then(|child_box| child_box.downcast::<Box>().ok())
            .and_then(|vbox| vbox.first_child()) // Get the first child (title label)
            .and_then(|widget| widget.downcast::<Label>().ok());

        if let Some(title_label) = title_label_opt {
            if title_label.label() == title_to_find {
                list_box.select_row(Some(&row));
                row.grab_focus();
                return;
            }
        }

        row_index += 1;
    }
}

/// Load CSS styling for the application
fn load_css() {
    // Define possible CSS file locations
    let css_paths = [
        "/usr/share/juswriteit/style.css",
        "src/style.css",
        "style.css",
    ];
    
    // Find the first available CSS file
    if let Some(css_file) = css_paths.iter().find(|&path| std::path::Path::new(path).exists()) {
        match std::fs::read_to_string(css_file) {
            Ok(css_data) => {
                // Create CSS provider and load the CSS data
                let css_provider = CssProvider::new();
                css_provider.load_from_data(&css_data);
                
                // Apply to the default display using the new API
                if let Some(display) = gtk::gdk::Display::default() {
                    gtk::style_context_add_provider_for_display(
                        &display,
                        &css_provider,
                        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                    println!("Loaded CSS from {}", css_file);
                } else {
                    eprintln!("Warning: Could not get default display. CSS styling not applied.");
                }
            },
            Err(e) => {
                eprintln!("Warning: Failed to read CSS file {}: {}", css_file, e);
            }
        }
    } else {
        eprintln!("Warning: No CSS file found. Using default styling.");
    }
}

/// Shows a dialog to rename a note (modern GTK4, no deprecated APIs)
fn show_rename_dialog<F>(parent: &ApplicationWindow, current_title: String, on_confirm: F)
where
    F: Fn(String) + 'static,
{
    use gtk::{Orientation, Box as GtkBox, Entry, Button, Label, Align};

    // Create a new transient window as a modal dialog
    let dialog = ApplicationWindow::builder()
        .transient_for(parent)
        .modal(true)
        .title("Rename Note")
        .default_width(320)
        .default_height(120)
        .css_classes(vec!["rename-dialog"])  // Add CSS class for styling
        .build();

    // Main vertical box
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(16);
    vbox.set_margin_bottom(16);
    vbox.set_margin_start(16);
    vbox.set_margin_end(16);

    let label = Label::new(Some("Enter new name:"));
    label.set_halign(Align::Start);
    vbox.append(&label);

    let entry = Entry::builder()
        .text(&current_title)
        .hexpand(true)
        .build();
    vbox.append(&entry);

    // Button row
    let button_box = GtkBox::new(Orientation::Horizontal, 8);
    button_box.set_halign(Align::End);

    let cancel_button = Button::with_label("Cancel");
    let rename_button = Button::with_label("Rename");
    button_box.append(&cancel_button);
    button_box.append(&rename_button);

    vbox.append(&button_box);
    dialog.set_child(Some(&vbox));

    // Focus entry and select text when shown
    let entry_clone = entry.clone();
    dialog.connect_map(move |_| {
        entry_clone.grab_focus();
        entry_clone.select_region(0, -1);
    });

    // Cancel closes the dialog
    let dialog_clone = dialog.clone();
    cancel_button.connect_clicked(move |_| {
        dialog_clone.close();
    });

    // Confirm logic
    let dialog_clone = dialog.clone();
    let parent_clone = parent.clone();
    let current_title_clone = current_title.clone();
    let entry_for_rename = entry.clone();
    rename_button.connect_clicked(move |_| {
        let new_title = entry_for_rename.text().to_string();
        if !new_title.trim().is_empty() && new_title != current_title_clone {
            dialog_clone.close();
            on_confirm(new_title);
        } else if new_title.trim().is_empty() {
            show_error_dialog(&parent_clone, "Rename Error", "New title cannot be empty.");
        } else {
            // User entered the same title - just close the dialog without an error
            dialog_clone.close();
        }
    });

    // Allow pressing Enter to confirm
    let rename_button_clone = rename_button.clone();
    let entry_for_activate = entry.clone();
    entry_for_activate.connect_activate(move |_| {
        rename_button_clone.emit_clicked();
    });

    dialog.present();
}
