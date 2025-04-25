use std::path::PathBuf;
use gtk::{glib, prelude::*};
use gtk::{ApplicationWindow, Dialog, ResponseType, Label, Box, Orientation};

/// Get the path to the notes directory
pub fn get_notes_dir() -> PathBuf {
    let user_data_dir = glib::user_data_dir();
    user_data_dir.join("juswriteit/notes")
}

/// Show an error dialog
pub fn show_error_dialog(parent: &ApplicationWindow, title: &str, message: &str) {
    let dialog = Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .build();
    
    let content_area = dialog.content_area();
    let message_label = Label::builder()
        .label(message)
        .xalign(0.0)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();
    
    content_area.append(&message_label);
    
    dialog.add_button("OK", ResponseType::Ok);
    dialog.set_default_response(ResponseType::Ok);
    
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    
    dialog.present();
}

/// Schedule an auto-save operation with a delay
pub fn schedule_auto_save<F: Fn() + 'static>(delay_ms: u32, callback: F) -> glib::SourceId {
    glib::timeout_add_local(
        std::time::Duration::from_millis(delay_ms.into()),
        move || {
            callback();
            // Don't repeat
            glib::ControlFlow::Break
        }
    )
}

/// Create a confirmation dialog
pub fn show_confirmation_dialog<F: Fn() + 'static + Clone>(
    parent: &ApplicationWindow,
    title: &str,
    message: &str,
    details: &str,
    confirm_action: F) {
    
    let dialog = Dialog::builder()
        .title(title)
        .transient_for(parent)
        .modal(true)
        .build();
    
    // Add message
    let content_area = dialog.content_area();
    let message_box = Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(10)
        .margin_start(20)
        .margin_end(20)
        .margin_top(20)
        .margin_bottom(20)
        .build();
    
    let title_label = Label::builder()
        .label(message)
        .xalign(0.0)
        .build();
    title_label.add_css_class("title-3");
    
    let detail_label = Label::builder()
        .label(details)
        .xalign(0.0)
        .build();
    
    message_box.append(&title_label);
    message_box.append(&detail_label);
    content_area.append(&message_box);
    
    // Add Cancel button
    dialog.add_button("Cancel", ResponseType::Cancel);
    
    // Add confirm button (destructive)
    let confirm_button = dialog.add_button("Confirm", ResponseType::Accept);
    confirm_button.add_css_class("destructive-action");
    
    dialog.set_default_response(ResponseType::Cancel);
    
    // Use the Clone trait properly with the confirm_action
    let action_clone = confirm_action.clone();
    
    // Handle response
    dialog.connect_response(move |dialog, response| {
        dialog.close();
        
        if response == ResponseType::Accept {
            action_clone();
        }
    });
    
    dialog.present();
}
