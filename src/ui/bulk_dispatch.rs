//! Bulk dispatch modal and broadcast list management.

use std::collections::HashSet;
use std::path::PathBuf;

use egui::{Align, Layout, RichText, ScrollArea, Stroke, vec2};
use jiff::civil::Date;

use crate::app::App;
use crate::model::{Action, BroadcastList, BulkDispatchContent, BulkDispatchInitial, ChatId};
use crate::theme::{self, Icon, Palette};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BulkTab {
    #[default]
    Numbers,
    BroadcastLists,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BulkContentType {
    #[default]
    Text,
    Audio,
    Media,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum BulkDispatchMode {
    #[default]
    Immediate,
    Scheduled,
}

#[derive(Clone, Debug)]
pub struct BulkDispatchState {
    pub tab: BulkTab,
    pub phone_text: String,
    pub content_type: BulkContentType,
    pub message_text: String,

    pub audio_path: Option<PathBuf>,
    pub audio_name: Option<String>,
    pub audio_duration: Option<f32>,
    pub audio_samples: Option<Vec<f32>>,

    pub media_paths: Vec<PathBuf>,
    pub media_names: Vec<String>,
    pub media_caption: String,

    pub mode: BulkDispatchMode,
    pub interval_seconds: u32,
    pub schedule_date: Date,
    pub schedule_hour: u8,
    pub schedule_minute: u8,

    pub selected_list_id: Option<String>,
    pub is_sending_to_list: bool,
    pub is_creating_list: bool,
    pub list_name_input: String,
    pub adding_chats: bool,
    pub chat_search: String,
    pub selected_for_list: HashSet<ChatId>,

    pub initialized: bool,
}

impl Default for BulkDispatchState {
    fn default() -> Self {
        let (date, h, min) = crate::util::local_datetime(crate::util::now() + 15 * 60)
            .unwrap_or_else(|| (crate::util::today(), 12, 0));
        Self {
            tab: BulkTab::Numbers,
            phone_text: String::new(),
            content_type: BulkContentType::Text,
            message_text: String::new(),
            audio_path: None,
            audio_name: None,
            audio_duration: None,
            audio_samples: None,
            media_paths: Vec::new(),
            media_names: Vec::new(),
            media_caption: String::new(),
            mode: BulkDispatchMode::Immediate,
            interval_seconds: 15,
            schedule_date: date,
            schedule_hour: h,
            schedule_minute: min,
            selected_list_id: None,
            is_sending_to_list: false,
            is_creating_list: false,
            list_name_input: String::new(),
            adding_chats: false,
            chat_search: String::new(),
            selected_for_list: HashSet::new(),
            initialized: false,
        }
    }
}

pub fn parse_phone_number(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let has_plus = raw.starts_with('+');
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    if digits.starts_with("55") && (digits.len() == 12 || digits.len() == 13) {
        return Some(format!("{digits}@s.whatsapp.net"));
    }
    if (digits.len() == 10 || digits.len() == 11) && !has_plus {
        return Some(format!("55{digits}@s.whatsapp.net"));
    }
    if has_plus && !digits.starts_with("55") && (10..=15).contains(&digits.len()) {
        return Some(format!("{digits}@s.whatsapp.net"));
    }
    if (10..=15).contains(&digits.len()) {
        if digits.len() == 10 || digits.len() == 11 {
            return Some(format!("55{digits}@s.whatsapp.net"));
        }
        return Some(format!("{digits}@s.whatsapp.net"));
    }
    None
}

pub fn parse_phone_numbers(text: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    for line in text.lines() {
        for part in line.split([',', ';']) {
            if let Some(jid) = parse_phone_number(part) {
                if seen.insert(jid.clone()) {
                    results.push(jid);
                }
            }
        }
    }
    results
}

pub fn format_duration_estimate(count: usize, interval_seconds: u32) -> String {
    if count <= 1 {
        return "envio imediato".to_owned();
    }
    let total_secs = (count - 1) as u64 * interval_seconds as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins == 0 {
        format!("~{secs}s")
    } else if secs == 0 {
        format!("~{mins} min")
    } else {
        format!("~{mins}m {secs}s")
    }
}

pub fn bulk_dispatch_dialog(
    app: &mut App,
    ui: &mut egui::Ui,
    initial: Option<&BulkDispatchInitial>,
) {
    let palette = app.palette;

    if !app.bulk_state.initialized {
        let (date, h, min) = crate::util::local_datetime(crate::util::now() + 15 * 60)
            .unwrap_or_else(|| (crate::util::today(), 12, 0));
        app.bulk_state.schedule_date = date;
        app.bulk_state.schedule_hour = h;
        app.bulk_state.schedule_minute = min;
        if app.bulk_state.interval_seconds == 0 {
            app.bulk_state.interval_seconds = 15;
        }

        match initial {
            Some(BulkDispatchInitial::Numbers) | None => {
                app.bulk_state.tab = BulkTab::Numbers;
            }
            Some(BulkDispatchInitial::BroadcastList(id)) => {
                app.bulk_state.tab = BulkTab::BroadcastLists;
                app.bulk_state.selected_list_id = Some(id.clone());
                app.bulk_state.is_sending_to_list = true;
            }
            Some(BulkDispatchInitial::Chats(chats)) => {
                app.bulk_state.tab = BulkTab::BroadcastLists;
                app.bulk_state.selected_for_list = chats.iter().cloned().collect();
                app.bulk_state.is_creating_list = true;
            }
        }
        app.bulk_state.initialized = true;
    }

    ui.horizontal(|ui| {
        let title_text = match app.bulk_state.tab {
            BulkTab::Numbers => "Disparo em Massa",
            BulkTab::BroadcastLists => {
                if app.bulk_state.is_sending_to_list {
                    "Disparo para Lista"
                } else {
                    "Listas de Transmissao"
                }
            }
        };

        theme::text(ui, title_text, theme::bold(18.0), palette.text);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(ui, Icon::X, 16.0, palette.secondary, palette.text, "Fechar")
                .clicked()
            {
                app.bulk_state.initialized = false;
                app.actions.push(Action::CloseDialog);
            }
        });
    });

    ui.add_space(8.0);

    if !app.bulk_state.is_sending_to_list {
        ui.horizontal(|ui| {
            let num_active = app.bulk_state.tab == BulkTab::Numbers;
            let list_active = app.bulk_state.tab == BulkTab::BroadcastLists;

            let num_bg = if num_active {
                palette.surface_active
            } else {
                palette.surface
            };
            let num_color = if num_active {
                palette.text
            } else {
                palette.secondary
            };

            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Colar Numeros")
                            .font(theme::medium(13.0))
                            .color(num_color),
                    )
                    .fill(num_bg)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(6.0),
                )
                .clicked()
            {
                app.bulk_state.tab = BulkTab::Numbers;
            }

            let list_count = app.broadcast_lists.len();
            let list_label = if list_count > 0 {
                format!("Listas de Transmissao ({list_count})")
            } else {
                "Listas de Transmissao".to_owned()
            };

            let list_bg = if list_active {
                palette.surface_active
            } else {
                palette.surface
            };
            let list_color = if list_active {
                palette.text
            } else {
                palette.secondary
            };

            if ui
                .add(
                    egui::Button::new(
                        RichText::new(list_label)
                            .font(theme::medium(13.0))
                            .color(list_color),
                    )
                    .fill(list_bg)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .corner_radius(6.0),
                )
                .clicked()
            {
                app.bulk_state.tab = BulkTab::BroadcastLists;
            }
        });

        ui.add_space(10.0);
    }

    match app.bulk_state.tab {
        BulkTab::Numbers => {
            numbers_view(app, ui, &palette);
        }
        BulkTab::BroadcastLists => {
            broadcast_lists_view(app, ui, &palette);
        }
    }
}

fn numbers_view(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let parsed_recipients = parse_phone_numbers(&app.bulk_state.phone_text);
    let recipient_count = parsed_recipients.len();

    ui.label(
        RichText::new("Numeros de telefone (um por linha)")
            .font(theme::semibold(13.0))
            .color(palette.text),
    );

    ui.add_space(2.0);

    let edit = egui::TextEdit::multiline(&mut app.bulk_state.phone_text)
        .hint_text("(33) 99946-4500\n(33) 99995-8058\n(33) 99158-3706")
        .desired_rows(4)
        .desired_width(ui.available_width())
        .font(theme::regular(13.5));

    ui.add(edit);

    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("DDI 55 adicionado automaticamente")
                .font(theme::regular(11.5))
                .color(palette.secondary),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if recipient_count == 1 {
                "1 numero valido".to_owned()
            } else {
                format!("{recipient_count} numeros validos")
            };
            ui.label(RichText::new(label).font(theme::semibold(11.5)).color(
                if recipient_count > 0 {
                    palette.accent
                } else {
                    palette.secondary
                },
            ));
        });
    });

    ui.add_space(10.0);

    content_type_selector(app, ui, palette);

    ui.add_space(8.0);

    content_input_section(app, ui, palette);

    ui.add_space(10.0);

    timing_mode_section(app, ui, palette);

    ui.add_space(10.0);

    interval_section(app, ui, palette, recipient_count);

    ui.add_space(14.0);

    ui.horizontal(|ui| {
        if theme::pill_button(ui, palette, "Cancelar", false).clicked() {
            app.bulk_state.initialized = false;
            app.actions.push(Action::CloseDialog);
        }

        let has_content = match app.bulk_state.content_type {
            BulkContentType::Text => !app.bulk_state.message_text.trim().is_empty(),
            BulkContentType::Audio => app.bulk_state.audio_samples.is_some(),
            BulkContentType::Media => !app.bulk_state.media_paths.is_empty(),
        };

        let can_start = recipient_count > 0 && has_content;
        let button_label = format!("Iniciar Disparo ({recipient_count})");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let start_btn = if can_start {
                theme::pill_button(ui, palette, &button_label, true)
            } else {
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new(&button_label).color(palette.secondary)),
                )
            };

            if start_btn.clicked() {
                execute_dispatch(app, parsed_recipients);
            }
        });
    });
}

fn content_type_selector(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let width = ui.available_width();
    let col_w = (width - 12.0) / 3.0;

    ui.horizontal(|ui| {
        let types = [
            (BulkContentType::Text, "Texto"),
            (BulkContentType::Audio, "Audio"),
            (BulkContentType::Media, "Foto/Video"),
        ];

        for (ct, name) in types {
            let active = app.bulk_state.content_type == ct;
            let bg = if active {
                palette.surface_active
            } else {
                palette.surface
            };
            let color = if active {
                palette.text
            } else {
                palette.secondary
            };

            if ui
                .add_sized(
                    vec2(col_w, 32.0),
                    egui::Button::new(RichText::new(name).font(theme::semibold(13.0)).color(color))
                        .fill(bg)
                        .stroke(Stroke::new(
                            1.0,
                            if active {
                                palette.accent
                            } else {
                                palette.outline
                            },
                        ))
                        .corner_radius(6.0),
                )
                .clicked()
            {
                app.bulk_state.content_type = ct;
            }
        }
    });
}

fn content_input_section(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    match app.bulk_state.content_type {
        BulkContentType::Text => {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Variaveis dinamicas:")
                        .font(theme::regular(11.5))
                        .color(palette.secondary),
                );
                let vars = [
                    ("{{primeiro_nome}}", "Primeiro nome"),
                    ("{{saudacao}}", "Saudacao"),
                    ("{{nome}}", "Nome completo"),
                    ("{{data}}", "Data"),
                    ("{{hora}}", "Hora"),
                ];
                for (tag, tip) in vars {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new(tag)
                                    .font(theme::mono(11.0))
                                    .color(palette.accent),
                            )
                            .fill(palette.surface_active)
                            .stroke(Stroke::new(1.0, palette.outline))
                            .corner_radius(4.0),
                        )
                        .on_hover_text(format!("Personalizar com {tip} de cada destinatario"))
                        .clicked()
                    {
                        if !app.bulk_state.message_text.is_empty()
                            && !app.bulk_state.message_text.ends_with(' ')
                        {
                            app.bulk_state.message_text.push(' ');
                        }
                        app.bulk_state.message_text.push_str(tag);
                    }
                }
            });
            let edit = egui::TextEdit::multiline(&mut app.bulk_state.message_text)
                .hint_text("Digite a mensagem a ser enviada (suporta {{primeiro_nome}}, {{saudacao}}...)")
                .desired_rows(3)
                .desired_width(ui.available_width())
                .font(theme::regular(13.5));
            ui.add(edit);
        }
        BulkContentType::Audio => {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                let audio_name = app.bulk_state.audio_name.clone();
                let audio_duration = app.bulk_state.audio_duration;

                if let Some(name) = audio_name {
                    ui.horizontal(|ui| {
                        let duration_text = if let Some(secs) = audio_duration {
                            let mins = (secs / 60.0) as u32;
                            let s = (secs % 60.0) as u32;
                            format!("{mins:02}:{s:02}")
                        } else {
                            "Audio pronto".to_owned()
                        };
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&name)
                                    .font(theme::semibold(13.0))
                                    .color(palette.text),
                            );
                            ui.label(
                                RichText::new(format!("{duration_text} - Audio de voz (PTT)"))
                                    .font(theme::regular(11.5))
                                    .color(palette.secondary),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::soft_button(ui, palette, Some(Icon::X), "Remover", false)
                                .clicked()
                            {
                                app.bulk_state.audio_path = None;
                                app.bulk_state.audio_name = None;
                                app.bulk_state.audio_duration = None;
                                app.bulk_state.audio_samples = None;
                            }
                            let is_playing = app.player.status("preview_bulk_audio").state
                                == crate::audio::State::Playing;
                            if theme::soft_button(
                                ui,
                                palette,
                                Some(if is_playing { Icon::Pause } else { Icon::Play }),
                                if is_playing { "Pausar" } else { "Ouvir áudio" },
                                false,
                            )
                            .clicked()
                            {
                                if let Some(samples) = &app.bulk_state.audio_samples {
                                    app.actions.push(Action::PlayVoiceSamples {
                                        id: "preview_bulk_audio".to_owned(),
                                        samples: samples.clone(),
                                    });
                                } else if let Some(path) = &app.bulk_state.audio_path {
                                    app.actions.push(Action::PlayVoice {
                                        message: "preview_bulk_audio".to_owned(),
                                        path: path.clone(),
                                    });
                                }
                            }
                        });
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        if theme::soft_button(
                            ui,
                            palette,
                            Some(Icon::Mic),
                            "Selecionar arquivo de audio (MP3, OGG, WAV, M4A)...",
                            false,
                        )
                        .clicked()
                        {
                            pick_and_decode_audio(app);
                        }
                        ui.label(
                            RichText::new(
                                "O audio sera enviado como mensagem de voz autentica no WhatsApp",
                            )
                            .font(theme::regular(11.5))
                            .color(palette.secondary),
                        );
                        ui.add_space(4.0);
                    });
                }
            });
        }
        BulkContentType::Media => {
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                if !app.bulk_state.media_paths.is_empty() {
                    ui.horizontal(|ui| {
                        let count = app.bulk_state.media_paths.len();
                        let label = if count == 1 {
                            app.bulk_state
                                .media_names
                                .first()
                                .cloned()
                                .unwrap_or_else(|| "1 arquivo".to_owned())
                        } else {
                            format!("{count} arquivos selecionados")
                        };
                        ui.label(
                            RichText::new(label)
                                .font(theme::semibold(13.0))
                                .color(palette.text),
                        );
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::soft_button(ui, palette, Some(Icon::X), "Remover", false)
                                .clicked()
                            {
                                app.bulk_state.media_paths.clear();
                                app.bulk_state.media_names.clear();
                            }
                        });
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        if theme::soft_button(
                            ui,
                            palette,
                            Some(Icon::Image),
                            "Selecionar fotos ou videos...",
                            false,
                        )
                        .clicked()
                        {
                            pick_media_files(app);
                        }
                        ui.add_space(4.0);
                    });
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Variaveis na legenda:")
                            .font(theme::regular(11.5))
                            .color(palette.secondary),
                    );
                    let vars = [
                        ("{{primeiro_nome}}", "Primeiro nome"),
                        ("{{saudacao}}", "Saudacao"),
                        ("{{nome}}", "Nome completo"),
                    ];
                    for (tag, tip) in vars {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(tag)
                                        .font(theme::mono(11.0))
                                        .color(palette.accent),
                                )
                                .fill(palette.surface_active)
                                .stroke(Stroke::new(1.0, palette.outline))
                                .corner_radius(4.0),
                            )
                            .on_hover_text(format!("Personalizar legenda com {tip}"))
                            .clicked()
                        {
                            if !app.bulk_state.media_caption.is_empty()
                                && !app.bulk_state.media_caption.ends_with(' ')
                            {
                                app.bulk_state.media_caption.push(' ');
                            }
                            app.bulk_state.media_caption.push_str(tag);
                        }
                    }
                });
                let caption_edit = egui::TextEdit::singleline(&mut app.bulk_state.media_caption)
                    .hint_text("Legenda da midia (opcional, suporta {{primeiro_nome}})...")
                    .desired_width(ui.available_width());
                ui.add(caption_edit);
            });
        }
    }
}

fn pick_and_decode_audio(app: &mut App) {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Selecionar audio para disparo")
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
                app.bulk_state.audio_path = Some(path);
                app.bulk_state.audio_name = Some(name);
                app.bulk_state.audio_duration = Some(duration);
                app.bulk_state.audio_samples = Some(samples);
            }
            Err(err) => {
                app.toast_error(format!("Erro ao decodificar audio: {err}"));
            }
        }
    }
}

fn pick_media_files(app: &mut App) {
    if let Some(paths) = rfd::FileDialog::new()
        .set_title("Selecionar fotos ou videos")
        .add_filter(
            "Midia",
            &["png", "jpg", "jpeg", "webp", "mp4", "mov", "avi", "pdf"],
        )
        .pick_files()
    {
        if !paths.is_empty() {
            let names: Vec<String> = paths
                .iter()
                .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(str::to_owned))
                .collect();
            app.bulk_state.media_paths = paths;
            app.bulk_state.media_names = names;
        }
    }
}

fn timing_mode_section(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let width = ui.available_width();
    let col_w = (width - 8.0) / 2.0;

    ui.horizontal(|ui| {
        let now_active = app.bulk_state.mode == BulkDispatchMode::Immediate;
        let sched_active = app.bulk_state.mode == BulkDispatchMode::Scheduled;

        let now_bg = if now_active {
            palette.surface_active
        } else {
            palette.surface
        };
        let now_color = if now_active {
            palette.text
        } else {
            palette.secondary
        };

        if ui
            .add_sized(
                vec2(col_w, 32.0),
                egui::Button::new(
                    RichText::new("Enviar agora")
                        .font(theme::semibold(13.0))
                        .color(now_color),
                )
                .fill(now_bg)
                .stroke(Stroke::new(
                    1.0,
                    if now_active {
                        palette.accent
                    } else {
                        palette.outline
                    },
                ))
                .corner_radius(6.0),
            )
            .clicked()
        {
            app.bulk_state.mode = BulkDispatchMode::Immediate;
        }

        let sched_bg = if sched_active {
            palette.surface_active
        } else {
            palette.surface
        };
        let sched_color = if sched_active {
            palette.text
        } else {
            palette.secondary
        };

        if ui
            .add_sized(
                vec2(col_w, 32.0),
                egui::Button::new(
                    RichText::new("Agendar")
                        .font(theme::semibold(13.0))
                        .color(sched_color),
                )
                .fill(sched_bg)
                .stroke(Stroke::new(
                    1.0,
                    if sched_active {
                        palette.accent
                    } else {
                        palette.outline
                    },
                ))
                .corner_radius(6.0),
            )
            .clicked()
        {
            app.bulk_state.mode = BulkDispatchMode::Scheduled;
        }
    });

    if app.bulk_state.mode == BulkDispatchMode::Scheduled {
        ui.add_space(8.0);
        crate::ui::dialogs::date_time_picker(
            ui,
            app.locale,
            palette,
            "bulk-dispatch-date-time",
            &mut app.bulk_state.schedule_date,
            &mut app.bulk_state.schedule_hour,
            &mut app.bulk_state.schedule_minute,
        );
    }
}

fn interval_section(app: &mut App, ui: &mut egui::Ui, palette: &Palette, recipient_count: usize) {
    let secs = app.bulk_state.interval_seconds;
    let label = if secs < 60 {
        format!("Intervalo entre envios: {secs}s")
    } else {
        let mins = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("Intervalo entre envios: {mins} min")
        } else {
            format!("Intervalo entre envios: {mins}m {s}s")
        }
    };

    ui.horizontal(|ui| {
        ui.label(
            RichText::new(label)
                .font(theme::semibold(13.0))
                .color(palette.text),
        );
    });

    ui.add_space(2.0);

    let slider = egui::Slider::new(&mut app.bulk_state.interval_seconds, 1..=300)
        .show_value(false)
        .trailing_fill(true);
    ui.add(slider);

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("1s")
                .font(theme::regular(11.0))
                .color(palette.dim),
        );
        ui.spacing_mut().item_spacing.x = 4.0;
        let presets = [5, 15, 30, 60, 120];
        for p in presets {
            let label = if p < 60 {
                format!("{p}s")
            } else {
                format!("{}m", p / 60)
            };
            let active = app.bulk_state.interval_seconds == p;
            let bg = if active {
                palette.surface_active
            } else {
                palette.surface
            };
            let color = if active {
                palette.accent
            } else {
                palette.secondary
            };

            if ui
                .add(
                    egui::Button::new(RichText::new(label).font(theme::regular(11.0)).color(color))
                        .fill(bg)
                        .corner_radius(4.0),
                )
                .clicked()
            {
                app.bulk_state.interval_seconds = p;
            }
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new("5 min")
                    .font(theme::regular(11.0))
                    .color(palette.dim),
            );
        });
    });

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        let rec_str = if recipient_count == 1 {
            "1 destinatario".to_owned()
        } else {
            format!("{recipient_count} destinatarios")
        };
        ui.label(
            RichText::new(rec_str)
                .font(theme::semibold(12.5))
                .color(palette.text),
        );

        let est_str = format!(
            "- {}",
            format_duration_estimate(recipient_count, app.bulk_state.interval_seconds)
        );
        ui.label(
            RichText::new(est_str)
                .font(theme::regular(12.5))
                .color(palette.secondary),
        );
    });
}

fn execute_dispatch(app: &mut App, targets: Vec<String>) {
    let send_at = match app.bulk_state.mode {
        BulkDispatchMode::Immediate => None,
        BulkDispatchMode::Scheduled => crate::util::to_unix_seconds(
            app.bulk_state.schedule_date,
            app.bulk_state.schedule_hour,
            app.bulk_state.schedule_minute,
        ),
    };

    let content = match app.bulk_state.content_type {
        BulkContentType::Text => {
            BulkDispatchContent::Text(app.bulk_state.message_text.trim().to_owned())
        }
        BulkContentType::Audio => {
            if let Some(samples) = &app.bulk_state.audio_samples {
                BulkDispatchContent::Voice(samples.clone())
            } else {
                return;
            }
        }
        BulkContentType::Media => {
            if !app.bulk_state.media_paths.is_empty() {
                let caption = (!app.bulk_state.media_caption.trim().is_empty())
                    .then(|| app.bulk_state.media_caption.trim().to_owned());
                BulkDispatchContent::Files {
                    paths: app.bulk_state.media_paths.clone(),
                    caption,
                }
            } else {
                return;
            }
        }
    };

    app.actions.push(Action::ExecuteBulkDispatch {
        targets,
        content,
        interval_seconds: app.bulk_state.interval_seconds,
        send_at,
    });

    app.bulk_state.initialized = false;
    app.actions.push(Action::CloseDialog);
}

fn broadcast_lists_view(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    if app.bulk_state.is_sending_to_list {
        if let Some(list_id) = &app.bulk_state.selected_list_id {
            if let Some(list) = app
                .broadcast_lists
                .iter()
                .find(|l| &l.id == list_id)
                .cloned()
            {
                send_to_list_form(app, ui, palette, &list);
                return;
            }
        }
        app.bulk_state.is_sending_to_list = false;
    }

    if let Some(list_id) = app.bulk_state.selected_list_id.clone() {
        if let Some(list) = app
            .broadcast_lists
            .iter()
            .find(|l| l.id == list_id)
            .cloned()
        {
            broadcast_list_detail(app, ui, palette, &list);
            return;
        } else {
            app.bulk_state.selected_list_id = None;
        }
    }

    if app.bulk_state.is_creating_list {
        create_list_form(app, ui, palette);
        return;
    }

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Suas listas de transmissao")
                .font(theme::semibold(14.0))
                .color(palette.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::soft_button(ui, palette, Some(Icon::Plus), "Nova Lista", true).clicked() {
                app.bulk_state.is_creating_list = true;
                app.bulk_state.list_name_input.clear();
                app.bulk_state.selected_for_list.clear();
            }
        });
    });

    ui.add_space(8.0);

    if app.broadcast_lists.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Nenhuma lista de transmissao criada ainda.")
                    .font(theme::regular(13.0))
                    .color(palette.secondary),
            );
            ui.add_space(6.0);
            if theme::pill_button(ui, palette, "+ Criar Primeira Lista", true).clicked() {
                app.bulk_state.is_creating_list = true;
                app.bulk_state.list_name_input.clear();
                app.bulk_state.selected_for_list.clear();
            }
        });
        return;
    }

    ScrollArea::vertical()
        .max_height(320.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let lists = app.broadcast_lists.clone();
            for list in lists {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&list.name)
                                    .font(theme::semibold(14.0))
                                    .color(palette.text),
                            );
                            let count = list.chats.len();
                            let chat_label = if count == 1 {
                                "1 chat".to_owned()
                            } else {
                                format!("{count} chats")
                            };
                            ui.label(
                                RichText::new(chat_label)
                                    .font(theme::regular(12.0))
                                    .color(palette.secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::icon_button(
                                ui,
                                Icon::Trash,
                                16.0,
                                palette.secondary,
                                palette.accent,
                                "Excluir",
                            )
                            .clicked()
                            {
                                app.actions
                                    .push(Action::DeleteBroadcastList(list.id.clone()));
                            }

                            if theme::soft_button(ui, palette, Some(Icon::Pencil), "Editar", false)
                                .clicked()
                            {
                                app.bulk_state.selected_list_id = Some(list.id.clone());
                            }

                            if theme::pill_button(ui, palette, "Enviar", true).clicked() {
                                app.bulk_state.selected_list_id = Some(list.id.clone());
                                app.bulk_state.is_sending_to_list = true;
                            }
                        });
                    });
                });
                ui.add_space(4.0);
            }
        });
}

fn broadcast_list_detail(
    app: &mut App,
    ui: &mut egui::Ui,
    palette: &Palette,
    list: &BroadcastList,
) {
    ui.horizontal(|ui| {
        if theme::icon_button(
            ui,
            Icon::ArrowLeft,
            16.0,
            palette.secondary,
            palette.text,
            "Voltar",
        )
        .clicked()
        {
            app.bulk_state.selected_list_id = None;
            app.bulk_state.adding_chats = false;
            return;
        }

        ui.vertical(|ui| {
            ui.label(
                RichText::new(&list.name)
                    .font(theme::bold(15.0))
                    .color(palette.text),
            );
            let count = list.chats.len();
            ui.label(
                RichText::new(format!("{count} chats"))
                    .font(theme::regular(11.5))
                    .color(palette.secondary),
            );
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::icon_button(
                ui,
                Icon::Trash,
                16.0,
                palette.secondary,
                palette.accent,
                "Excluir lista",
            )
            .clicked()
            {
                app.actions
                    .push(Action::DeleteBroadcastList(list.id.clone()));
                app.bulk_state.selected_list_id = None;
            }

            if theme::pill_button(ui, palette, "Enviar", true).clicked() {
                app.bulk_state.is_sending_to_list = true;
            }
        });
    });

    ui.add_space(8.0);

    if app.bulk_state.adding_chats {
        let chat_items: Vec<(ChatId, String)> = app
            .chats
            .iter()
            .map(|c| (c.id.clone(), app.chat_title(c)))
            .collect();
        let needle = crate::util::search_key(app.bulk_state.chat_search.trim());
        let mut list_to_save = None;

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Adicionar conversas")
                        .font(theme::semibold(13.0))
                        .color(palette.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if theme::soft_button(ui, palette, None, "Concluir", true).clicked() {
                        app.bulk_state.adding_chats = false;
                    }
                });
            });

            ui.add_space(4.0);
            let search_edit = egui::TextEdit::singleline(&mut app.bulk_state.chat_search)
                .hint_text("Buscar conversas...")
                .desired_width(ui.available_width());
            ui.add(search_edit);

            ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                for (chat_id, title) in chat_items {
                    if !needle.is_empty() && !crate::util::search_key(&title).contains(&needle) {
                        continue;
                    }
                    let mut in_list = list.chats.contains(&chat_id);
                    if ui.checkbox(&mut in_list, &title).changed() {
                        let mut updated = list.clone();
                        if in_list {
                            if !updated.chats.contains(&chat_id) {
                                updated.chats.push(chat_id.clone());
                            }
                        } else {
                            updated.chats.retain(|c| c != &chat_id);
                        }
                        list_to_save = Some(updated);
                    }
                }
            });
        });

        if let Some(updated) = list_to_save {
            app.actions.push(Action::SaveBroadcastList(updated));
        }
    } else {
        ui.horizontal(|ui| {
            if theme::soft_button(ui, palette, Some(Icon::Plus), "Adicionar conversas", false)
                .clicked()
            {
                app.bulk_state.adding_chats = true;
            }
        });
    }

    ui.add_space(6.0);

    ScrollArea::vertical()
        .max_height(240.0)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for chat_id in &list.chats {
                let chat_title = app
                    .chat(chat_id)
                    .map(|c| app.chat_title(c))
                    .unwrap_or_else(|| chat_id.clone());
                let is_group = chat_id.ends_with("@g.us");

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(&chat_title)
                                    .font(theme::semibold(13.0))
                                    .color(palette.text),
                            );
                            let kind_badge = if is_group { "Grupo" } else { "Contato" };
                            ui.label(
                                RichText::new(kind_badge)
                                    .font(theme::regular(11.0))
                                    .color(palette.secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::icon_button(
                                ui,
                                Icon::X,
                                14.0,
                                palette.secondary,
                                palette.text,
                                "Remover",
                            )
                            .clicked()
                            {
                                let mut updated = list.clone();
                                updated.chats.retain(|c| c != chat_id);
                                app.actions.push(Action::SaveBroadcastList(updated));
                            }
                        });
                    });
                });
                ui.add_space(2.0);
            }
        });
}

fn send_to_list_form(app: &mut App, ui: &mut egui::Ui, palette: &Palette, list: &BroadcastList) {
    let recipient_count = list.chats.len();

    ui.horizontal(|ui| {
        if theme::icon_button(
            ui,
            Icon::ArrowLeft,
            16.0,
            palette.secondary,
            palette.text,
            "Voltar para lista",
        )
        .clicked()
        {
            app.bulk_state.is_sending_to_list = false;
            return;
        }
        ui.label(
            RichText::new(format!("Disparo para lista: {}", list.name))
                .font(theme::semibold(14.0))
                .color(palette.text),
        );
    });

    ui.add_space(8.0);

    timing_mode_section(app, ui, palette);

    ui.add_space(8.0);

    content_type_selector(app, ui, palette);

    ui.add_space(8.0);

    content_input_section(app, ui, palette);

    ui.add_space(8.0);

    interval_section(app, ui, palette, recipient_count);

    ui.add_space(12.0);

    ui.horizontal(|ui| {
        if theme::pill_button(ui, palette, "Cancelar", false).clicked() {
            app.bulk_state.is_sending_to_list = false;
        }

        let has_content = match app.bulk_state.content_type {
            BulkContentType::Text => !app.bulk_state.message_text.trim().is_empty(),
            BulkContentType::Audio => app.bulk_state.audio_samples.is_some(),
            BulkContentType::Media => !app.bulk_state.media_paths.is_empty(),
        };

        let can_start = recipient_count > 0 && has_content;
        let button_label = format!("Enviar ({recipient_count})");

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let start_btn = if can_start {
                theme::pill_button(ui, palette, &button_label, true)
            } else {
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new(&button_label).color(palette.secondary)),
                )
            };

            if start_btn.clicked() {
                execute_dispatch(app, list.chats.clone());
            }
        });
    });
}

fn create_list_form(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        if theme::icon_button(
            ui,
            Icon::ArrowLeft,
            16.0,
            palette.secondary,
            palette.text,
            "Voltar",
        )
        .clicked()
        {
            app.bulk_state.is_creating_list = false;
            return;
        }
        ui.label(
            RichText::new("Criar Nova Lista de Transmissao")
                .font(theme::bold(15.0))
                .color(palette.text),
        );
    });

    ui.add_space(8.0);

    ui.label(
        RichText::new("Nome da lista")
            .font(theme::semibold(13.0))
            .color(palette.text),
    );
    let name_edit = egui::TextEdit::singleline(&mut app.bulk_state.list_name_input)
        .hint_text("Ex: PROJETO MED, Clientes VIP, etc.")
        .desired_width(ui.available_width());
    ui.add(name_edit);

    ui.add_space(8.0);

    ui.label(
        RichText::new("Selecione as conversas")
            .font(theme::semibold(13.0))
            .color(palette.text),
    );
    let search_edit = egui::TextEdit::singleline(&mut app.bulk_state.chat_search)
        .hint_text("Buscar contatos e grupos...")
        .desired_width(ui.available_width());
    ui.add(search_edit);

    let needle = crate::util::search_key(app.bulk_state.chat_search.trim());
    let chat_items: Vec<(ChatId, String)> = app
        .chats
        .iter()
        .map(|c| (c.id.clone(), app.chat_title(c)))
        .collect();

    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for (chat_id, title) in chat_items {
            if !needle.is_empty() && !crate::util::search_key(&title).contains(&needle) {
                continue;
            }
            let mut checked = app.bulk_state.selected_for_list.contains(&chat_id);
            if ui.checkbox(&mut checked, &title).changed() {
                if checked {
                    app.bulk_state.selected_for_list.insert(chat_id);
                } else {
                    app.bulk_state.selected_for_list.remove(&chat_id);
                }
            }
        }
    });

    ui.add_space(8.0);

    let selected_count = app.bulk_state.selected_for_list.len();
    ui.label(
        RichText::new(format!("{selected_count} conversas selecionadas"))
            .font(theme::regular(12.0))
            .color(palette.secondary),
    );

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        if theme::pill_button(ui, palette, "Cancelar", false).clicked() {
            app.bulk_state.is_creating_list = false;
        }

        let can_save = !app.bulk_state.list_name_input.trim().is_empty();
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let save_btn = if can_save {
                theme::pill_button(ui, palette, "Salvar Lista", true)
            } else {
                ui.add_enabled(
                    false,
                    egui::Button::new(RichText::new("Salvar Lista").color(palette.secondary)),
                )
            };

            if save_btn.clicked() {
                let id = format!("list_{}", crate::util::now());
                let list = BroadcastList {
                    id: id.clone(),
                    name: app.bulk_state.list_name_input.trim().to_owned(),
                    chats: app.bulk_state.selected_for_list.iter().cloned().collect(),
                    created_at: crate::util::now(),
                };
                app.actions.push(Action::SaveBroadcastList(list));
                app.bulk_state.is_creating_list = false;
                app.bulk_state.selected_list_id = Some(id);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_brazilian_phone_numbers_with_and_without_ddi() {
        assert_eq!(
            parse_phone_number("(33) 99946-4500"),
            Some("5533999464500@s.whatsapp.net".to_owned())
        );
        assert_eq!(
            parse_phone_number("(33) 99995-8058"),
            Some("5533999958058@s.whatsapp.net".to_owned())
        );
        assert_eq!(
            parse_phone_number("(33) 99158-3706"),
            Some("5533991583706@s.whatsapp.net".to_owned())
        );
        assert_eq!(
            parse_phone_number("31988887777"),
            Some("5531988887777@s.whatsapp.net".to_owned())
        );
        assert_eq!(
            parse_phone_number("+55 33 99946-4500"),
            Some("5533999464500@s.whatsapp.net".to_owned())
        );
        assert_eq!(
            parse_phone_number("5533999464500"),
            Some("5533999464500@s.whatsapp.net".to_owned())
        );
    }

    #[test]
    fn parses_international_numbers() {
        assert_eq!(
            parse_phone_number("+1 202 555 0123"),
            Some("12025550123@s.whatsapp.net".to_owned())
        );
    }

    #[test]
    fn parses_multiline_text_with_deduplication() {
        let input = "
        (33) 99946-4500
        (33) 99995-8058
        (33) 99946-4500
        (33) 99158-3706
        ";
        let parsed = parse_phone_numbers(input);
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0], "5533999464500@s.whatsapp.net");
        assert_eq!(parsed[1], "5533999958058@s.whatsapp.net");
        assert_eq!(parsed[2], "5533991583706@s.whatsapp.net");
    }

    #[test]
    fn formats_duration_estimate() {
        assert_eq!(format_duration_estimate(1, 30), "envio imediato");
        assert_eq!(format_duration_estimate(2, 30), "~30s");
        assert_eq!(format_duration_estimate(43, 15), "~10m 30s");
        assert_eq!(format_duration_estimate(41, 15), "~10 min");
    }
}
