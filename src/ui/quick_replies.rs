//! WhatsApp Business quick-reply manager.

use egui::{Align, Layout, TextEdit};

use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon, Palette};

pub fn manager(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        theme::text(ui, "Quick replies", theme::bold(18.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(ui, Icon::X, 17.0, palette.secondary, palette.text, "Close")
                .clicked()
            {
                app.actions.push(Action::CloseDialog);
            }
        });
    });
    theme::text(
        ui,
        "Synced with WhatsApp Business. Type / in a chat to insert one.",
        theme::regular(12.5),
        palette.secondary,
    );
    ui.separator();

    ui.label("Shortcut");
    ui.add(TextEdit::singleline(&mut app.quick_reply_shortcut).hint_text("/thanks"));
    ui.label("Message");
    ui.add(
        TextEdit::multiline(&mut app.quick_reply_message)
            .desired_rows(3)
            .hint_text("Thank you for contacting us."),
    );
    ui.label("Keywords (comma separated)");
    ui.add(TextEdit::singleline(&mut app.quick_reply_keywords).hint_text("thanks, customer"));
    let editing = app.quick_reply_editing.clone();
    let ready = !app
        .quick_reply_shortcut
        .trim()
        .trim_start_matches('/')
        .is_empty()
        && !app.quick_reply_message.trim().is_empty();
    if ui
        .add_enabled(
            ready,
            egui::Button::new(if editing.is_some() { "Save" } else { "Add" }),
        )
        .clicked()
    {
        app.actions.push(Action::SaveQuickReply {
            id: editing,
            shortcut: app.quick_reply_shortcut.clone(),
            message: app.quick_reply_message.clone(),
            keywords: app
                .quick_reply_keywords
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
        });
    }

    ui.separator();
    if app.quick_replies.is_empty() {
        theme::text(
            ui,
            "No quick replies yet.",
            theme::regular(13.0),
            palette.secondary,
        );
        return;
    }
    let replies = app.quick_replies.clone();
    egui::ScrollArea::vertical()
        .max_height(260.0)
        .show(ui, |ui| {
            for reply in replies {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        theme::text(
                            ui,
                            format!("/{}", reply.shortcut),
                            theme::semibold(13.5),
                            palette.text,
                        );
                        theme::text(ui, &reply.message, theme::regular(12.5), palette.secondary);
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::icon_button(
                            ui,
                            Icon::Trash,
                            15.0,
                            palette.secondary,
                            palette.danger,
                            "Delete",
                        )
                        .clicked()
                        {
                            app.actions.push(Action::DeleteQuickReply(reply.id.clone()));
                        }
                        if theme::icon_button(
                            ui,
                            Icon::Pencil,
                            15.0,
                            palette.secondary,
                            palette.text,
                            "Edit",
                        )
                        .clicked()
                        {
                            app.quick_reply_editing = Some(reply.id.clone());
                            app.quick_reply_shortcut = reply.shortcut.clone();
                            app.quick_reply_message = reply.message.clone();
                            app.quick_reply_keywords = reply.keywords.join(", ");
                        }
                    });
                });
                ui.separator();
            }
        });
}
