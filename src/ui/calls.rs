//! Calls history and Calls tab view.

use egui::{Align, Frame, Layout, Margin, Rect, ScrollArea, Sense, Vec2, vec2};

use crate::app::App;
use crate::model::{Action, CallFilter, CallStatus, Content, Page};
use crate::theme::{self, Icon, Palette};
use crate::ui::widgets;

struct CallItem {
    chat_id: String,
    title: String,
    from_me: bool,
    timestamp: i64,
    status: CallStatus,
    video: bool,
    duration: u64,
}

/// The central pane when on Calls page and no chat is selected.
pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    ui.vertical_centered(|ui| {
        ui.add_space(ui.available_height() / 3.0);
        theme::icon(ui, Icon::Phone, 56.0, palette.dim);
        ui.add_space(16.0);
        theme::text(
            ui,
            "Faça ligações de voz e vídeo",
            theme::bold(20.0),
            palette.text,
        );
        ui.add_space(8.0);
        theme::text(
            ui,
            "Conecte-se com seus clientes e contatos diretamente pelo ZapFast.",
            theme::regular(14.0),
            palette.dim,
        );
        ui.add_space(16.0);
        let label = "Nova chamada";
        let response = theme::circle_button(
            ui,
            Icon::PhonePlus,
            72.0,
            palette.accent,
            palette.accent_hover,
            palette.on_accent,
            label,
        );
        if response.clicked() {
            app.actions.push(Action::ShowDialog(crate::model::Dialog::NewChat));
        }
        ui.add_space(10.0);
        theme::text(ui, label, theme::semibold(15.0), palette.text);
    });
}

/// The left-hand calls list (history and filter chips).
pub fn list(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let width = app.settings.sidebar_width;

    egui::Panel::left("calls-panel")
        .resizable(false)
        .exact_size(width)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::ZERO))
        .show(ui, |ui| {
            Frame::new()
                .inner_margin(Margin::symmetric(14, 8))
                .show(ui, |ui| {
                    // Header row
                    ui.horizontal(|ui| {
                        theme::text(ui, "Ligações", theme::bold(20.0), palette.text);
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::icon_button(
                                ui,
                                Icon::PhonePlus,
                                18.0,
                                palette.secondary,
                                palette.text,
                                "Nova chamada",
                            )
                            .clicked()
                            {
                                app.actions.push(Action::ShowDialog(crate::model::Dialog::NewChat));
                            }
                        });
                    });

                    ui.add_space(8.0);

                    // Filter chips: Todas / Perdidas
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(6.0, 4.0);
                        for filter in CallFilter::EVERY {
                            let selected = app.call_filter == filter;
                            let label = filter.label(app.locale);
                            let chip = widgets::filter_chip(ui, &palette, label.as_ref(), 0, selected);
                            if chip.clicked() {
                                app.actions.push(Action::SetCallFilter(filter));
                            }
                        }
                    });
                });

            ui.separator();

            // Collect call items without holding borrow on app
            let mut calls: Vec<CallItem> = Vec::new();
            for chat in &app.chats {
                if let Some(convo) = app.conversations.get(&chat.id) {
                    for msg in &convo.messages {
                        if let Content::Call {
                            video,
                            status,
                            duration,
                            ..
                        } = msg.content
                        {
                            if app.call_filter == CallFilter::Missed && !status.is_bad() {
                                continue;
                            }
                            calls.push(CallItem {
                                chat_id: chat.id.clone(),
                                title: app.chat_title(chat),
                                from_me: msg.from_me,
                                timestamp: msg.timestamp,
                                status,
                                video,
                                duration,
                            });
                        }
                    }
                }
            }
            calls.sort_by_key(|c| -c.timestamp);

            if calls.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    theme::icon(ui, Icon::Phone, 40.0, palette.dim);
                    ui.add_space(12.0);
                    theme::text(
                        ui,
                        if app.call_filter == CallFilter::Missed {
                            "Nenhuma chamada perdida"
                        } else {
                            "Nenhuma chamada recente"
                        },
                        theme::bold(15.0),
                        palette.text,
                    );
                    ui.add_space(6.0);
                    theme::text(
                        ui,
                        "As ligações de voz e vídeo feitas ou recebidas no WhatsApp aparecerão aqui.",
                        theme::regular(12.0),
                        palette.dim,
                    );
                });
            } else {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for item in calls {
                            render_call_row(app, ui, &palette, item);
                        }
                    });
            }
        });
}

fn render_call_row(
    app: &mut App,
    ui: &mut egui::Ui,
    palette: &Palette,
    item: CallItem,
) {
    let is_missed = item.status.is_bad();
    let dir_icon = if is_missed {
        Icon::PhoneMissed
    } else if item.from_me {
        Icon::PhoneOutgoing
    } else {
        Icon::PhoneIncoming
    };
    let dir_color = if is_missed {
        palette.danger
    } else {
        palette.accent
    };

    let date_str = crate::util::local_datetime(item.timestamp)
        .map(|(d, h, m)| format!("{:02}/{:02} {:02}:{:02}", d.day(), d.month(), h, m))
        .unwrap_or_else(|| "-".to_string());

    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), 60.0),
        Sense::click(),
    );

    if response.hovered() {
        widgets::highlight_card(ui, palette, rect, palette.surface_hover);
    }

    if response.clicked() {
        app.actions.push(Action::Open(Page::Chats));
        app.actions.push(Action::OpenChat(item.chat_id.clone()));
    }

    if ui.is_rect_visible(rect) {
        let avatar_rect = Rect::from_min_size(rect.left_top() + vec2(10.0, 10.0), Vec2::splat(40.0));
        let pic = app.avatar(&item.chat_id);
        widgets::paint_avatar(ui, palette, avatar_rect, &item.title, &item.chat_id, pic.as_deref());

        let text_left = avatar_rect.right() + 10.0;
        let right_margin = 12.0;
        let name_rect = Rect::from_min_size(
            rect.left_top() + vec2(text_left - rect.left(), 12.0),
            vec2(rect.width() - (text_left - rect.left()) - right_margin - 30.0, 18.0),
        );
        let name_galley = ui.painter().layout_no_wrap(
            item.title.clone(),
            theme::medium(14.0),
            if is_missed { palette.danger } else { palette.text },
        );
        ui.painter().galley(name_rect.min, name_galley, palette.text);

        let sub_rect = Rect::from_min_size(
            rect.left_top() + vec2(text_left - rect.left(), 33.0),
            vec2(rect.width() - (text_left - rect.left()) - right_margin - 30.0, 16.0),
        );
        theme::paint_icon(
            ui,
            dir_icon,
            Rect::from_min_size(sub_rect.min, Vec2::splat(14.0)),
            13.0,
            dir_color,
        );
        let sub_text = if item.duration > 0 {
            format!("{} • {}s", date_str, item.duration)
        } else {
            date_str
        };
        let sub_galley = ui.painter().layout_no_wrap(
            sub_text,
            theme::regular(12.0),
            palette.dim,
        );
        ui.painter().galley(sub_rect.min + vec2(18.0, 0.0), sub_galley, palette.dim);

        let right_icon_rect = Rect::from_min_size(
            rect.right_top() + vec2(-right_margin - 18.0, 21.0),
            Vec2::splat(18.0),
        );
        theme::paint_icon(
            ui,
            if item.video { Icon::Video } else { Icon::Phone },
            right_icon_rect,
            16.0,
            palette.secondary,
        );
    }
}
