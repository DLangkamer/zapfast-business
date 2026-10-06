//! Commercial Metrics and Conversion Dashboard: pipeline KPIs, stage conversion
//! bars, average ticket size, and follow-up efficiency.

use egui::{vec2, Align, Color32, Layout, Margin, Rounding, Vec2};

use crate::app::App;
use crate::theme::{self, Icon, Palette};
use super::crm_sidecar::{format_currency, parse_hex_color};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::CircleAlert, 20.0, palette.accent);
            theme::text(ui, "Dashboard de Métricas e Conversão Comercial", theme::bold(17.0), palette.text);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::X, 18.0, palette.secondary, palette.text, "Fechar").clicked() {
                    app.dialog = None;
                }
            });
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(10.0);

        // Calculate KPIs
        let total_deals = app.crm_deals.len();
        let total_value: i64 = app.crm_deals.values().map(|d| d.value_cents).sum();
        let avg_ticket = if total_deals > 0 {
            total_value / total_deals as i64
        } else {
            0
        };

        // Closed deals (last column or "close" id)
        let closed_col_id = app.crm_columns.iter().find(|c| c.id == "close").map(|c| c.id.as_str()).unwrap_or("");
        let closed_deals: Vec<_> = app.crm_deals.values().filter(|d| d.column_id == closed_col_id).collect();
        let closed_value: i64 = closed_deals.iter().map(|d| d.value_cents).sum();

        let now = crate::util::now();
        let pending_followups = app.crm_followups.iter().filter(|f| !f.done).count();
        let overdue_followups = app.crm_followups.iter().filter(|f| !f.done && f.remind_at <= now).count();

        // 1. KPI Cards Row
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            render_kpi_card(ui, &palette, "Pipeline Total", &format_currency(total_value), palette.accent);
            render_kpi_card(ui, &palette, "Fechamentos Ganhos", &format_currency(closed_value), palette.accent);
            render_kpi_card(ui, &palette, "Ticket Médio", &format_currency(avg_ticket), palette.accent);
            render_kpi_card(ui, &palette, "Negócios no Funil", &format!("{total_deals}"), palette.text);
            render_kpi_card(ui, &palette, "Lembretes Pendentes", &format!("{pending_followups} ({overdue_followups} atrasados)"), if overdue_followups > 0 { palette.danger } else { palette.secondary });
        });

        ui.add_space(16.0);
        theme::text(ui, "Taxa de Conversão por Etapa do Funil", theme::semibold(14.0), palette.text);
        ui.add_space(6.0);

        // 2. Stage breakdown bars
        egui::ScrollArea::vertical()
            .max_height(260.0)
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                let columns = app.crm_columns.clone();
                for (idx, col) in columns.iter().enumerate() {
                    let in_col: Vec<_> = app.crm_deals.values().filter(|d| d.column_id == col.id).collect();
                    let count = in_col.len();
                    let col_value: i64 = in_col.iter().map(|d| d.value_cents).sum();

                    let percentage = if total_deals > 0 {
                        (count as f32 / total_deals as f32) * 100.0
                    } else {
                        0.0
                    };

                    egui::Frame::none()
                        .fill(palette.bubble_in)
                        .rounding(Rounding::same(6.0))
                        .inner_margin(Margin::same(10))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    let dot_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                                    let (dot_rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                    ui.painter().circle_filled(dot_rect.center(), 5.0, dot_color);

                                    theme::text(ui, &format!("{}. {}", idx + 1, col.title), theme::semibold(13.0), palette.text);

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        theme::text(ui, &format_currency(col_value), theme::medium(12.5), palette.accent);
                                        ui.add_space(8.0);
                                        theme::text(ui, &format!("{count} contatos ({percentage:.1}%)"), theme::regular(12.0), palette.dim);
                                    });
                                });

                                ui.add_space(4.0);

                                // Progress bar
                                let bar_width = ui.available_width().max(50.0);
                                let (rect, _) = ui.allocate_exact_size(vec2(bar_width, 8.0), egui::Sense::hover());
                                ui.painter().rect_filled(rect, 4.0, palette.surface_hover);

                                let fill_width = (bar_width * (percentage / 100.0)).max(2.0);
                                let fill_rect = egui::Rect::from_min_size(rect.min, vec2(fill_width, 8.0));
                                let bar_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                                ui.painter().rect_filled(fill_rect, 4.0, bar_color);
                            });
                        });
                }
            });
    });
}

fn render_kpi_card(ui: &mut egui::Ui, palette: &Palette, label: &str, value: &str, accent: Color32) {
    egui::Frame::none()
        .fill(palette.bubble_in)
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width(120.0);
            ui.vertical(|ui| {
                theme::text(ui, label, theme::regular(11.0), palette.dim);
                ui.add_space(2.0);
                theme::text(ui, value, theme::bold(14.0), accent);
            });
        });
}
