//! WhatsApp Business quick-reply manager with audio PTT support.

use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke, TextEdit};

use crate::app::App;
use crate::model::Action;
use crate::theme::{self, Icon, Palette};

pub fn manager(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        theme::text(ui, "Respostas rapidas & Audios PTT", theme::bold(18.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(ui, Icon::X, 17.0, palette.secondary, palette.text, "Fechar")
                .clicked()
            {
                app.quick_reply_editing = None;
                app.quick_reply_voice = None;
                app.quick_reply_voice_name = None;
                app.quick_reply_voice_duration = None;
                app.actions.push(Action::CloseDialog);
            }
        });
    });
    theme::text(
        ui,
        "Sincronizado com WhatsApp Business. Digite / na conversa para usar ou enviar audio gravado na hora.",
        theme::regular(12.5),
        palette.secondary,
    );
    ui.separator();

    ui.horizontal(|ui| {
        ui.label(theme::rich_text("Atalho (sem a barra)", theme::medium(13.0), palette.text));
        if app.quick_reply_editing.is_some() {
            theme::text(ui, "(Editando resposta)", theme::semibold(12.0), palette.primary);
        }
    });
    ui.add(TextEdit::singleline(&mut app.quick_reply_shortcut).hint_text("ex: apresentacao ou preco"));

    ui.label(theme::rich_text("Mensagem de texto (opcional se houver audio)", theme::medium(13.0), palette.text));
    ui.add(
        TextEdit::multiline(&mut app.quick_reply_message)
            .desired_rows(2)
            .hint_text("Ex: Ola! Segue abaixo a nossa apresentacao detalhada."),
    );

    ui.label(theme::rich_text("Palavras-chave (separadas por virgula)", theme::medium(13.0), palette.text));
    ui.add(TextEdit::singleline(&mut app.quick_reply_keywords).hint_text("ex: plano, proposta, suporte"));

    ui.add_space(4.0);

    // Audio attachment section
    ui.label(theme::rich_text("Audio de voz (Enviado como se tivesse gravado na hora)", theme::bold(13.0), palette.text));
    if let Some(voice_name) = &app.quick_reply_voice_name {
        Frame::new()
            .fill(palette.surface_active)
            .stroke(Stroke::new(1.0, palette.primary))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::text(ui, "🎙", theme::bold(15.0), palette.primary);
                    ui.vertical(|ui| {
                        theme::text(ui, voice_name, theme::semibold(13.0), palette.text);
                        theme::text(
                            ui,
                            "Chega ao cliente como nota de voz oficial (PTT) com waveform",
                            theme::regular(11.5),
                            palette.secondary,
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::icon_button(
                            ui,
                            Icon::Trash,
                            14.0,
                            palette.secondary,
                            palette.danger,
                            "Remover audio",
                        )
                        .clicked()
                        {
                            app.quick_reply_voice = None;
                            app.quick_reply_voice_name = None;
                            app.quick_reply_voice_duration = None;
                        }
                    });
                });
            });
    } else {
        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("🎙 Anexar arquivo de audio (.mp3, .ogg, .wav, .m4a)")
                            .font(theme::medium(12.5))
                            .color(palette.primary),
                    )
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(6.0),
                )
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Selecionar audio para resposta rapida")
                    .add_filter("Audio", &["mp3", "ogg", "opus", "wav", "m4a", "aac"])
                    .pick_file()
                {
                    match crate::audio::decode_file(&path) {
                        Ok(samples) => {
                            let duration = samples.len() as f32 / crate::voice::RATE as f32;
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("audio")
                                .to_string();
                            let mins = (duration / 60.0) as u32;
                            let secs = (duration % 60.0) as u32;
                            app.quick_reply_voice_name =
                                Some(format!("{name} ({mins}:{secs:02})"));
                            app.quick_reply_voice_duration = Some(duration);
                            app.quick_reply_voice = Some(samples);
                        }
                        Err(err) => {
                            app.toast_error(format!("Erro ao decodificar audio: {err}"));
                        }
                    }
                }
            }
        });
    }

    ui.add_space(4.0);

    let editing = app.quick_reply_editing.clone();
    let ready = !app
        .quick_reply_shortcut
        .trim()
        .trim_start_matches('/')
        .is_empty()
        && (!app.quick_reply_message.trim().is_empty() || app.quick_reply_voice.is_some());

    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                ready,
                egui::Button::new(if editing.is_some() {
                    "Salvar alteracoes"
                } else {
                    "Adicionar resposta rapida"
                })
                .fill(if ready { palette.primary } else { palette.surface })
                .corner_radius(6.0),
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
                voice: app.quick_reply_voice.clone(),
            });
        }

        if app.quick_reply_editing.is_some() {
            if ui
                .add(
                    egui::Button::new("Cancelar edicao")
                        .fill(palette.surface)
                        .stroke(Stroke::new(1.0, palette.outline))
                        .corner_radius(6.0),
                )
                .clicked()
            {
                app.quick_reply_editing = None;
                app.quick_reply_shortcut.clear();
                app.quick_reply_message.clear();
                app.quick_reply_keywords.clear();
                app.quick_reply_voice = None;
                app.quick_reply_voice_name = None;
                app.quick_reply_voice_duration = None;
            }
        }
    });

    ui.separator();
    if app.quick_replies.is_empty() {
        theme::text(
            ui,
            "Nenhuma resposta rapida cadastrada ainda.",
            theme::regular(13.0),
            palette.secondary,
        );
        return;
    }

    let replies = app.quick_replies.clone();
    let active_chat = app.active_chat();

    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            for reply in replies {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            theme::text(
                                ui,
                                format!("/{}", reply.shortcut),
                                theme::bold(14.0),
                                palette.text,
                            );
                            if let Some(voice) = &reply.voice {
                                let dur = voice.len() as f32 / crate::voice::RATE as f32;
                                let mins = (dur / 60.0) as u32;
                                let secs = (dur % 60.0) as u32;
                                theme::text(
                                    ui,
                                    format!("🎙 Audio PTT ({mins}:{secs:02})"),
                                    theme::semibold(12.0),
                                    palette.primary,
                                );
                            }
                        });
                        if !reply.message.is_empty() {
                            theme::text(ui, &reply.message, theme::regular(12.5), palette.secondary);
                        }
                        if !reply.keywords.is_empty() {
                            theme::text(
                                ui,
                                format!("Tags: {}", reply.keywords.join(", ")),
                                theme::regular(11.0),
                                palette.secondary,
                            );
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::icon_button(
                            ui,
                            Icon::Trash,
                            15.0,
                            palette.secondary,
                            palette.danger,
                            "Excluir",
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
                            "Editar",
                        )
                        .clicked()
                        {
                            app.quick_reply_editing = Some(reply.id.clone());
                            app.quick_reply_shortcut = reply.shortcut.clone();
                            app.quick_reply_message = reply.message.clone();
                            app.quick_reply_keywords = reply.keywords.join(", ");
                            app.quick_reply_voice = reply.voice.clone();
                            app.quick_reply_voice_name = reply.voice.as_ref().map(|v| {
                                let dur = v.len() as f32 / crate::voice::RATE as f32;
                                let mins = (dur / 60.0) as u32;
                                let secs = (dur % 60.0) as u32;
                                format!("Audio gravado ({mins}:{secs:02})")
                            });
                            app.quick_reply_voice_duration = reply
                                .voice
                                .as_ref()
                                .map(|v| v.len() as f32 / crate::voice::RATE as f32);
                        }
                        if reply.voice.is_some() && active_chat.is_some() {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Enviar agora")
                                            .font(theme::medium(12.0))
                                            .color(palette.primary),
                                    )
                                    .fill(palette.surface_active)
                                    .stroke(Stroke::new(1.0, palette.primary))
                                    .corner_radius(4.0),
                                )
                                .clicked()
                            {
                                app.actions.push(Action::ApplyQuickReply(reply.clone()));
                                app.actions.push(Action::CloseDialog);
                            }
                        }
                    });
                });
                ui.separator();
            }
        });
}
