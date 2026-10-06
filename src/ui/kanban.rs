//! Visual Sales Kanban Board: customizable pipeline columns, deal cards,
//! stage transitions, search filter, and JSON backup export/import.

use egui::{vec2, Align, Layout, Margin, Rounding, Stroke, Vec2};

use crate::app::App;
use crate::model::{Action, CrmColumn, CrmDeal, Dialog};
use crate::theme::{self, Icon, Palette};
use super::crm_sidecar::{format_currency, parse_hex_color};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;

    ui.vertical(|ui| {
        // 1. Top bar: Title, Search, Metrics button, Backup buttons, New column, Close
        render_top_bar(app, ui, &palette);

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        // 2. Horizontal scroll area with pipeline columns
        render_board(app, ui, &palette);
    });
}

fn render_top_bar(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.horizontal(|ui| {
        theme::icon(ui, Icon::ListChecks, 22.0, palette.accent);
        theme::text(ui, "Funil de Vendas Visual (Kanban)", theme::bold(18.0), palette.text);

        ui.add_space(16.0);

        // Search in Kanban
        ui.add(
            egui::TextEdit::singleline(&mut app.crm_search)
                .hint_text("Buscar contato ou tag...")
                .desired_width(180.0),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Close dialog
            if theme::icon_button(ui, Icon::X, 18.0, palette.secondary, palette.text, "Fechar").clicked() {
                app.dialog = None;
            }

            // Metrics button
            if ui.button("📊 Métricas").clicked() {
                app.actions.push(Action::ShowDialog(Dialog::CrmMetrics));
            }

            // Import Backup button
            if ui.button("📥 Importar Backup").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON Backup", &["json"])
                    .set_title("Selecionar arquivo de backup do CRM")
                    .pick_file()
                {
                    app.actions.push(Action::ImportCrmBackup(path));
                }
            }

            // Export Backup button
            if ui.button("📤 Exportar Backup").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON Backup", &["json"])
                    .set_file_name("zapfast-crm-backup.json")
                    .set_title("Salvar backup do CRM")
                    .save_file()
                {
                    app.actions.push(Action::ExportCrmBackup(path));
                }
            }

            // + Nova Etapa button
            let new_col_id = ui.id().with("kanban_new_col_input");
            let mut new_col_name = ui.ctx().data(|d| d.get_temp::<String>(new_col_id)).unwrap_or_default();

            let add_col = ui.button("+ Nova Etapa");
            ui.add(
                egui::TextEdit::singleline(&mut new_col_name)
                    .hint_text("Nome da etapa...")
                    .desired_width(120.0),
            );

            if add_col.clicked() && !new_col_name.trim().is_empty() {
                let id = format!("col_{}", crate::util::now());
                let order = app.crm_columns.len() as i32;
                let col = CrmColumn {
                    id,
                    title: new_col_name.trim().to_owned(),
                    color: "#3b82f6".to_owned(),
                    order,
                };
                app.actions.push(Action::SaveCrmColumn(col));
                new_col_name.clear();
            }
            ui.ctx().data_mut(|d| d.insert_temp(new_col_id, new_col_name));
        });
    });
}

fn render_board(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let columns = app.crm_columns.clone();
    let search_lower = app.crm_search.trim().to_lowercase();

    egui::ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;

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

    egui::Frame::none()
        .fill(palette.bubble_in)
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(260.0);
            ui.set_height(ui.available_height().max(400.0));

            // Column Header
            ui.horizontal(|ui| {
                let dot_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                ui.painter().circle_filled(dot_rect.center(), 5.0, dot_color);

                theme::text(ui, &col.title, theme::semibold(14.0), palette.text);

                // Badge count
                theme::text(ui, &format!("({count})"), theme::regular(12.0), palette.dim);

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Delete column option (if more than 1 column exists)
                    if app.crm_columns.len() > 1 {
                        let del = theme::icon_button(ui, Icon::Trash, 12.0, palette.dim, palette.danger, "Excluir etapa");
                        if del.clicked() {
                            app.actions.push(Action::DeleteCrmColumn(col.id.clone()));
                        }
                    }
                });
            });

            // Total Column Value
            let formatted_total = format_currency(total_value_cents);
            theme::text(ui, &formatted_total, theme::medium(12.0), palette.accent);

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            // Cards scroll area
            egui::ScrollArea::vertical()
                .id_salt(format!("kanban_col_{}", col.id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    for deal in &deals_in_col {
                        render_deal_card(app, ui, palette, deal);
                    }

                    if deals_in_col.is_empty() {
                        ui.add_space(20.0);
                        ui.vertical_centered(|ui| {
                            theme::text(ui, "Nenhum contato nesta etapa", theme::regular(12.0), palette.dim);
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

    let frame = egui::Frame::none()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.surface_hover))
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::same(8));

    let res = frame.show(ui, |ui| {
        ui.vertical(|ui| {
            // Card top: Contact name & Avatar
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::User, 14.0, palette.secondary);
                theme::text(ui, &title, theme::semibold(13.0), palette.text);

                if has_active_followup {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        theme::icon(ui, Icon::Clock, 12.0, palette.accent);
                    });
                }
            });

            // Deal value
            if deal.value_cents > 0 {
                ui.add_space(2.0);
                let val_str = format_currency(deal.value_cents);
                theme::text(ui, &val_str, theme::medium(12.0), palette.accent);
            }

            // Tags
            if !deal.tags.is_empty() {
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
                    for tag in &deal.tags {
                        egui::Frame::none()
                            .fill(palette.surface_hover)
                            .rounding(Rounding::same(3.0))
                            .inner_margin(Margin::symmetric(4, 2))
                            .show(ui, |ui| {
                                theme::text(ui, tag, theme::regular(10.5), palette.text);
                            });
                    }
                });
            }

            // Card bottom actions: Open chat & Move stage
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.small_button("Abrir conversa").clicked() {
                    app.actions.push(Action::OpenChat(deal.chat_id.clone()));
                    app.dialog = None;
                }

                // Stage change popup/menu
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt(format!("move_col_{}", deal.chat_id))
                        .selected_text("Mover etapa")
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for target_col in &app.crm_columns {
                                if target_col.id != deal.column_id {
                                    if ui.selectable_label(false, &target_col.title).clicked() {
                                        let mut updated = deal.clone();
                                        updated.column_id = target_col.id.clone();
                                        updated.updated_at = crate::util::now();
                                        app.actions.push(Action::SaveCrmDeal(updated));
                                    }
                                }
                            }
                        });
                });
            });
        });
    });
}
