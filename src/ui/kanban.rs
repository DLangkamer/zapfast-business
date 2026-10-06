//! Visual Sales Kanban Board: customizable pipeline columns, deal cards,
//! stage transitions, contact picker, search filter, and JSON backup export/import.

use egui::{vec2, Align, CornerRadius, Frame, Layout, Margin, Stroke, Vec2};

use crate::app::App;
use crate::model::{Action, CrmColumn, CrmDeal, Dialog};
use crate::theme::{self, Icon, Palette};
use super::crm_sidecar::{format_currency, parse_hex_color};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;

    // Allocate comfortable height for the Kanban board modal
    let min_board_height = (ui.ctx().content_rect().height() - 120.0).clamp(460.0, 720.0);
    ui.set_min_height(min_board_height);

    ui.vertical(|ui| {
        // 1. Top Bar: Title, KPIs, Actions (Novo Negócio, Nova Etapa, Métricas, Backup, Fechar)
        render_top_bar(app, ui, &palette);

        ui.add_space(8.0);

        // 2. Search & Filter Bar
        render_search_bar(app, ui, &palette);

        ui.add_space(8.0);

        // 3. Optional Sub-panels: Add Deal or Add Column if active
        render_popovers(app, ui, &palette);

        ui.separator();
        ui.add_space(8.0);

        // 4. Horizontal Board Columns
        render_board(app, ui, &palette);
    });
}

fn render_top_bar(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let total_deals = app.crm_deals.len();
    let total_value: i64 = app.crm_deals.values().map(|d| d.value_cents).sum();

    ui.horizontal(|ui| {
        theme::icon(ui, Icon::ListChecks, 22.0, palette.accent);
        theme::text(ui, "Funil de Vendas Comercial", theme::bold(18.0), palette.text);

        // KPI Badge
        Frame::new()
            .fill(palette.surface_hover)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 4))
            .show(ui, |ui| {
                let kpi_text = format!("{} negócios • {}", total_deals, format_currency(total_value));
                theme::text(ui, &kpi_text, theme::medium(12.0), palette.accent);
            });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Close dialog
            if theme::icon_button(ui, Icon::X, 18.0, palette.dim, palette.text, "Fechar").clicked() {
                app.dialog = None;
            }

            // Metrics button
            if ui.button("📊 Métricas").clicked() {
                app.actions.push(Action::ShowDialog(Dialog::CrmMetrics));
            }

            // Backup Dropdown / Buttons
            egui::ComboBox::from_id_salt("kanban_backup_menu")
                .selected_text("💾 Backup")
                .width(100.0)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "📤 Exportar Backup JSON").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Backup", &["json"])
                            .set_file_name("zapfast-crm-backup.json")
                            .set_title("Salvar backup do CRM")
                            .save_file()
                        {
                            app.actions.push(Action::ExportCrmBackup(path));
                        }
                    }
                    if ui.selectable_label(false, "📥 Importar Backup JSON").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON Backup", &["json"])
                            .set_title("Selecionar arquivo de backup do CRM")
                            .pick_file()
                        {
                            app.actions.push(Action::ImportCrmBackup(path));
                        }
                    }
                });

            // + Nova Etapa button
            let add_stage_id = ui.id().with("kanban_add_stage_open");
            let add_stage_open = ui.ctx().data(|d| d.get_temp::<bool>(add_stage_id)).unwrap_or(false);
            if ui.button(if add_stage_open { "✕ Cancelar Etapa" } else { "+ Nova Etapa" }).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(add_stage_id, !add_stage_open));
            }

            // + Novo Negócio button (Primary Accent)
            let add_deal_id = ui.id().with("kanban_add_deal_open");
            let add_deal_open = ui.ctx().data(|d| d.get_temp::<bool>(add_deal_id)).unwrap_or(false);
            let btn_deal = egui::Button::new(if add_deal_open { "✕ Cancelar" } else { "+ Novo Negócio" })
                .fill(if add_deal_open { palette.surface_hover } else { palette.accent });
            if ui.add(btn_deal).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(add_deal_id, !add_deal_open));
            }
        });
    });
}

fn render_search_bar(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        theme::icon(ui, Icon::Search, 15.0, palette.dim);
        ui.add(
            egui::TextEdit::singleline(&mut app.crm_search)
                .hint_text("Buscar por contato, telefone, nota interna ou etiqueta...")
                .desired_width(ui.available_width() - 40.0),
        );
        if !app.crm_search.is_empty() {
            if theme::icon_button(ui, Icon::X, 12.0, palette.dim, palette.text, "Limpar busca").clicked() {
                app.crm_search.clear();
            }
        }
    });
}

fn render_popovers(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let add_deal_id = ui.id().with("kanban_add_deal_open");
    let add_stage_id = ui.id().with("kanban_add_stage_open");
    let add_deal_open = ui.ctx().data(|d| d.get_temp::<bool>(add_deal_id)).unwrap_or(false);
    let add_stage_open = ui.ctx().data(|d| d.get_temp::<bool>(add_stage_id)).unwrap_or(false);

    // 1. Popover: Adicionar Novo Negócio ao Funil
    if add_deal_open {
        Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.accent))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::User, 16.0, palette.accent);
                        theme::text(ui, "Adicionar Contato / Negócio ao Funil", theme::bold(14.0), palette.text);
                    });

                    ui.add_space(8.0);

                    let filter_contact_id = ui.id().with("add_deal_filter_contact");
                    let mut contact_filter = ui.ctx().data(|d| d.get_temp::<String>(filter_contact_id)).unwrap_or_default();
                    let selected_chat_id = ui.id().with("add_deal_selected_chat");
                    let mut selected_chat = ui.ctx().data(|d| d.get_temp::<String>(selected_chat_id)).unwrap_or_default();

                    let selected_col_id = ui.id().with("add_deal_selected_col");
                    let mut selected_col = ui.ctx().data(|d| d.get_temp::<String>(selected_col_id)).unwrap_or_else(|| {
                        app.crm_columns.first().map(|c| c.id.clone()).unwrap_or_default()
                    });

                    let val_id = ui.id().with("add_deal_value_input");
                    let mut val_str = ui.ctx().data(|d| d.get_temp::<String>(val_id)).unwrap_or_default();

                    let tag_id = ui.id().with("add_deal_tags_input");
                    let mut tag_str = ui.ctx().data(|d| d.get_temp::<String>(tag_id)).unwrap_or_default();

                    ui.horizontal(|ui| {
                        // Contact Picker
                        ui.vertical(|ui| {
                            theme::text(ui, "Selecione o Contato:", theme::semibold(12.0), palette.dim);
                            ui.add(
                                egui::TextEdit::singleline(&mut contact_filter)
                                    .hint_text("Filtrar contatos...")
                                    .desired_width(220.0),
                            );

                            let filter_lower = contact_filter.trim().to_lowercase();
                            let chats: Vec<_> = app.chats.iter()
                                .filter(|c| {
                                    if filter_lower.is_empty() { return true; }
                                    let title = app.chat_title(c).to_lowercase();
                                    title.contains(&filter_lower) || c.id.contains(&filter_lower)
                                })
                                .take(25)
                                .collect();

                            let current_display = if selected_chat.is_empty() {
                                "Nenhum selecionado".to_owned()
                            } else {
                                app.chat(&selected_chat).map(|c| app.chat_title(c)).unwrap_or_else(|| selected_chat.clone())
                            };

                            egui::ComboBox::from_id_salt("picker_chat_combo")
                                .selected_text(&current_display)
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    for chat in chats {
                                        let title = app.chat_title(chat);
                                        if ui.selectable_label(selected_chat == chat.id, &title).clicked() {
                                            selected_chat = chat.id.clone();
                                        }
                                    }
                                });
                        });

                        ui.add_space(16.0);

                        // Stage Selector
                        ui.vertical(|ui| {
                            theme::text(ui, "Etapa Inicial:", theme::semibold(12.0), palette.dim);
                            let col_title = app.crm_columns.iter().find(|c| c.id == selected_col).map(|c| c.title.as_str()).unwrap_or("Selecione");
                            egui::ComboBox::from_id_salt("picker_col_combo")
                                .selected_text(col_title)
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    for col in &app.crm_columns {
                                        if ui.selectable_label(selected_col == col.id, &col.title).clicked() {
                                            selected_col = col.id.clone();
                                        }
                                    }
                                });

                            ui.add_space(4.0);
                            theme::text(ui, "Valor em R$:", theme::semibold(12.0), palette.dim);
                            ui.add(
                                egui::TextEdit::singleline(&mut val_str)
                                    .hint_text("ex: 1500,00")
                                    .desired_width(180.0),
                            );
                        });

                        ui.add_space(16.0);

                        // Tags & Submit
                        ui.vertical(|ui| {
                            theme::text(ui, "Etiquetas (separadas por vírgula):", theme::semibold(12.0), palette.dim);
                            ui.add(
                                egui::TextEdit::singleline(&mut tag_str)
                                    .hint_text("ex: VIP, Decisor")
                                    .desired_width(200.0),
                            );

                            ui.add_space(8.0);
                            let can_save = !selected_chat.is_empty() && !selected_col.is_empty();
                            if ui.add_enabled(can_save, egui::Button::new("✓ Adicionar ao Funil").fill(palette.accent)).clicked() {
                                let sanitized = val_str.replace(',', ".").replace(' ', "");
                                let cents = (sanitized.parse::<f64>().unwrap_or(0.0) * 100.0).round().max(0.0) as i64;
                                let tags: Vec<String> = tag_str.split(',')
                                    .map(|s| s.trim().to_owned())
                                    .filter(|s| !s.is_empty())
                                    .collect();

                                let deal = CrmDeal {
                                    chat_id: selected_chat.clone(),
                                    column_id: selected_col.clone(),
                                    value_cents: cents,
                                    notes: String::new(),
                                    tags,
                                    updated_at: crate::util::now(),
                                };
                                app.actions.push(Action::SaveCrmDeal(deal));

                                // Reset form and close
                                selected_chat.clear();
                                val_str.clear();
                                tag_str.clear();
                                ui.ctx().data_mut(|d| d.insert_temp(add_deal_id, false));
                            }
                        });
                    });

                    ui.ctx().data_mut(|d| {
                        d.insert_temp(filter_contact_id, contact_filter);
                        d.insert_temp(selected_chat_id, selected_chat);
                        d.insert_temp(selected_col_id, selected_col);
                        d.insert_temp(val_id, val_str);
                        d.insert_temp(tag_id, tag_str);
                    });
                });
            });
        ui.add_space(8.0);
    }

    // 2. Popover: Adicionar Nova Etapa
    if add_stage_open {
        Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0, palette.surface_hover))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::text(ui, "Nova Etapa do Funil:", theme::semibold(13.0), palette.text);

                    let name_id = ui.id().with("new_stage_name_input");
                    let mut name = ui.ctx().data(|d| d.get_temp::<String>(name_id)).unwrap_or_default();
                    ui.add(
                        egui::TextEdit::singleline(&mut name)
                            .hint_text("Nome da etapa (ex: Contrato Enviado)")
                            .desired_width(220.0),
                    );

                    let color_id = ui.id().with("new_stage_color_input");
                    let mut color_hex = ui.ctx().data(|d| d.get_temp::<String>(color_id)).unwrap_or_else(|| "#3b82f6".to_owned());

                    let colors = [
                        ("#3b82f6", "Azul"),
                        ("#eab308", "Amarelo"),
                        ("#8b5cf6", "Roxo"),
                        ("#10b981", "Verde"),
                        ("#f97316", "Laranja"),
                        ("#6b7280", "Cinza"),
                    ];
                    for (hex, _label) in colors {
                        let is_active = color_hex == hex;
                        let c = parse_hex_color(hex).unwrap_or(palette.accent);
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
                        ui.painter().circle_filled(r.center(), if is_active { 8.0 } else { 6.0 }, c);
                        if is_active {
                            ui.painter().circle_stroke(r.center(), 9.0, Stroke::new(1.5, palette.text));
                        }
                        if resp.clicked() {
                            color_hex = hex.to_owned();
                        }
                    }

                    if ui.button("✓ Criar Etapa").clicked() && !name.trim().is_empty() {
                        let col = CrmColumn {
                            id: format!("col_{}", crate::util::now()),
                            title: name.trim().to_owned(),
                            color: color_hex.clone(),
                            order: app.crm_columns.len() as i32,
                        };
                        app.actions.push(Action::SaveCrmColumn(col));
                        name.clear();
                        ui.ctx().data_mut(|d| d.insert_temp(add_stage_id, false));
                    }

                    ui.ctx().data_mut(|d| {
                        d.insert_temp(name_id, name);
                        d.insert_temp(color_id, color_hex);
                    });
                });
            });
        ui.add_space(8.0);
    }
}

fn render_board(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let columns = app.crm_columns.clone();
    let search_lower = app.crm_search.trim().to_lowercase();

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;

                for col in &columns {
                    render_column(app, ui, palette, col, &search_lower);
                }
            });
        });
}

fn render_column(
    app: &mut App,
    ui: &mut egui::Ui,
    palette: &Palette,
    col: &CrmColumn,
    search: &str,
) {
    let deals_in_col: Vec<CrmDeal> = app
        .crm_deals
        .values()
        .filter(|d| d.column_id == col.id)
        .filter(|d| {
            if search.is_empty() {
                return true;
            }
            let contact_name = app
                .chat(&d.chat_id)
                .map(|c| app.chat_title(c).to_lowercase())
                .unwrap_or_else(|| d.chat_id.to_lowercase());
            let matches_name = contact_name.contains(search);
            let matches_tags = d.tags.iter().any(|t| t.to_lowercase().contains(search));
            let matches_notes = d.notes.to_lowercase().contains(search);
            matches_name || matches_tags || matches_notes
        })
        .cloned()
        .collect();

    let total_value_cents: i64 = deals_in_col.iter().map(|d| d.value_cents).sum();
    let count = deals_in_col.len();
    let dot_color = parse_hex_color(&col.color).unwrap_or(palette.accent);

    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(280.0);
            ui.set_min_height(380.0);

            // Column Header
            ui.horizontal(|ui| {
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 5.0, dot_color);

                theme::text(ui, &col.title, theme::bold(14.0), palette.text);

                // Badge count
                Frame::new()
                    .fill(palette.surface_hover)
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        theme::text(ui, &format!("{count}"), theme::medium(11.0), palette.dim);
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Delete column option (if more than 1 column exists)
                    if app.crm_columns.len() > 1 {
                        if theme::icon_button(ui, Icon::Trash, 12.0, palette.dim, palette.danger, "Excluir etapa").clicked() {
                            app.actions.push(Action::DeleteCrmColumn(col.id.clone()));
                        }
                    }

                    // Quick + button on column header
                    let add_deal_id = ui.id().with("kanban_add_deal_open");
                    let selected_col_id = ui.id().with("add_deal_selected_col");
                    if theme::icon_button(ui, Icon::Plus, 14.0, palette.secondary, palette.accent, "Adicionar negócio nesta etapa").clicked() {
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(add_deal_id, true);
                            d.insert_temp(selected_col_id, col.id.clone());
                        });
                    }
                });
            });

            // Column Total Value
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let formatted_total = format_currency(total_value_cents);
                theme::text(ui, &formatted_total, theme::bold(12.5), palette.accent);
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Cards Scroll Area
            egui::ScrollArea::vertical()
                .id_salt(format!("kanban_col_{}", col.id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;

                    for deal in &deals_in_col {
                        render_deal_card(app, ui, palette, deal);
                    }

                    if deals_in_col.is_empty() {
                        ui.add_space(24.0);
                        ui.vertical_centered(|ui| {
                            theme::text(ui, "Nenhum negócio aqui", theme::regular(12.0), palette.dim);
                            ui.add_space(6.0);

                            let add_deal_id = ui.id().with("kanban_add_deal_open");
                            let selected_col_id = ui.id().with("add_deal_selected_col");
                            if ui.button("+ Adicionar contato").clicked() {
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(add_deal_id, true);
                                    d.insert_temp(selected_col_id, col.id.clone());
                                });
                            }
                        });
                    }
                });
        });
}

fn render_deal_card(app: &mut App, ui: &mut egui::Ui, palette: &Palette, deal: &CrmDeal) {
    let title = app
        .chat(&deal.chat_id)
        .map(|c| app.chat_title(c))
        .unwrap_or_else(|| deal.chat_id.clone());

    let has_active_followup = app
        .crm_followups
        .iter()
        .any(|f| f.chat_id == deal.chat_id && !f.done);

    let followup_overdue = app
        .crm_followups
        .iter()
        .any(|f| f.chat_id == deal.chat_id && !f.done && f.remind_at <= crate::util::now());

    Frame::new()
        .fill(palette.bubble_in)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Card Header: Contact avatar icon + Title
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(22.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 11.0, palette.surface_hover);
                    theme::paint_icon(ui, Icon::User, r, 12.0, palette.accent);

                    ui.add_space(2.0);
                    theme::text(ui, &title, theme::bold(13.0), palette.text);

                    if has_active_followup {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let icon_color = if followup_overdue { palette.danger } else { palette.accent };
                            let label = if followup_overdue { "Lembrete atrasado!" } else { "Lembrete ativo" };
                            theme::icon(ui, Icon::Clock, 13.0, icon_color).on_hover_text(label);
                        });
                    }
                });

                // Deal Value
                if deal.value_cents > 0 {
                    ui.add_space(4.0);
                    let val_str = format_currency(deal.value_cents);
                    theme::text(ui, &val_str, theme::bold(13.5), palette.accent);
                }

                // Tags
                if !deal.tags.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                        for tag in &deal.tags {
                            Frame::new()
                                .fill(palette.surface_hover)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(5, 2))
                                .show(ui, |ui| {
                                    theme::text(ui, tag, theme::medium(10.5), palette.text);
                                });
                        }
                    });
                }

                // Notes snippet
                if !deal.notes.trim().is_empty() {
                    ui.add_space(4.0);
                    let snippet = if deal.notes.len() > 60 {
                        format!("{}...", &deal.notes[..60])
                    } else {
                        deal.notes.clone()
                    };
                    theme::text(ui, &snippet, theme::regular(11.0), palette.dim);
                }

                // Card Footer Actions
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    // Chat button
                    if ui.small_button("💬 Conversar").clicked() {
                        app.actions.push(Action::OpenChat(deal.chat_id.clone()));
                        app.dialog = None;
                    }

                    // Stage Transition ComboBox
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::ComboBox::from_id_salt(format!("move_col_{}", deal.chat_id))
                            .selected_text("Mover etapa ▾")
                            .width(110.0)
                            .show_ui(ui, |ui| {
                                for target_col in &app.crm_columns {
                                    if target_col.id != deal.column_id {
                                        let dot_color = parse_hex_color(&target_col.color).unwrap_or(palette.accent);
                                        ui.horizontal(|ui| {
                                            let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
                                            ui.painter().circle_filled(r.center(), 4.0, dot_color);
                                            if ui.selectable_label(false, &target_col.title).clicked() {
                                                let mut updated = deal.clone();
                                                updated.column_id = target_col.id.clone();
                                                updated.updated_at = crate::util::now();
                                                app.actions.push(Action::SaveCrmDeal(updated));
                                            }
                                        });
                                    }
                                }
                            });
                    });
                });
            });
        });
}
