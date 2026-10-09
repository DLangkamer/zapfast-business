//! Account-scoped CRM task and follow-up inbox.

use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke};

use crate::app::App;
use crate::model::{Action, CrmFollowup};
use crate::theme::{self, Icon};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let now = crate::util::now();
    let mut pending: Vec<_> = app
        .crm_followups
        .iter()
        .filter(|item| !item.done)
        .cloned()
        .collect();
    pending.sort_by_key(|item| item.remind_at);
    let completed = app.crm_followups.iter().filter(|item| item.done).count();
    let overdue = pending.iter().filter(|item| item.remind_at <= now).count();

    ui.horizontal(|ui| {
        theme::icon(ui, Icon::Bell, 20.0, palette.accent);
        theme::text(ui, "Central de tarefas", theme::bold(18.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(ui, Icon::X, 16.0, palette.dim, palette.text, "Fechar").clicked()
            {
                app.actions.push(Action::CloseDialog);
            }
        });
    });
    theme::text(
        ui,
        "Follow-ups desta conta do WhatsApp. Abra a conversa, conclua ou adie sem perder o contexto.",
        theme::regular(12.0),
        palette.dim,
    );
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        summary(ui, &palette, "Atrasados", overdue, overdue > 0);
        summary(ui, &palette, "Pendentes", pending.len(), false);
        summary(ui, &palette, "Concluídos", completed, false);
    });
    ui.separator();

    if pending.is_empty() {
        ui.add_space(24.0);
        ui.vertical_centered(|ui| {
            theme::icon(ui, Icon::CircleCheck, 30.0, palette.accent);
            theme::text(
                ui,
                "Nenhum follow-up pendente",
                theme::semibold(14.0),
                palette.text,
            );
            theme::text(
                ui,
                "Crie um lembrete na lateral CRM de uma conversa.",
                theme::regular(12.0),
                palette.dim,
            );
        });
        return;
    }

    egui::ScrollArea::vertical()
        .max_height(520.0)
        .show(ui, |ui| {
            for followup in pending {
                row(app, ui, &followup, now);
                ui.add_space(6.0);
            }
        });
}

fn summary(
    ui: &mut egui::Ui,
    palette: &crate::theme::Palette,
    title: &str,
    value: usize,
    alert: bool,
) {
    Frame::new()
        .fill(if alert {
            palette.danger.gamma_multiply(0.12)
        } else {
            palette.surface
        })
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            theme::text(
                ui,
                &format!("{title}: {value}"),
                theme::semibold(12.0),
                if alert { palette.danger } else { palette.text },
            );
        });
}

fn row(app: &mut App, ui: &mut egui::Ui, followup: &CrmFollowup, now: i64) {
    let palette = app.palette;
    let contact = app
        .chat(&followup.chat_id)
        .map(|chat| app.chat_title(chat))
        .unwrap_or_else(|| {
            followup
                .chat_id
                .trim_end_matches("@s.whatsapp.net")
                .to_owned()
        });
    let overdue = followup.remind_at <= now;
    let when = crate::util::local_datetime(followup.remind_at)
        .map(|(date, hour, minute)| {
            format!(
                "{:02}/{:02}/{} às {:02}:{:02}",
                date.day(),
                date.month(),
                date.year(),
                hour,
                minute
            )
        })
        .unwrap_or_else(|| "Data indisponível".to_owned());

    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(
            1.0,
            if overdue {
                palette.danger
            } else {
                palette.surface_hover
            },
        ))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::text(ui, &followup.title, theme::semibold(13.0), palette.text);
                    theme::text(ui, &contact, theme::regular(12.0), palette.secondary);
                    theme::text(
                        ui,
                        &when,
                        theme::regular(11.0),
                        if overdue { palette.danger } else { palette.dim },
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Concluir").clicked() {
                        app.actions
                            .push(Action::CompleteCrmFollowup(followup.id.clone()));
                    }
                    if ui.small_button("+1 dia").clicked() {
                        app.actions.push(Action::SnoozeCrmFollowup {
                            id: followup.id.clone(),
                            until: now + 86_400,
                        });
                    }
                    if ui.small_button("Abrir conversa").clicked() {
                        app.actions.push(Action::CloseDialog);
                        app.actions.push(Action::OpenChat(followup.chat_id.clone()));
                    }
                });
            });
        });
}
