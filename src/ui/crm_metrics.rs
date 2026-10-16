//! Commercial Metrics, Conversion Dashboard, and Ad Attribution ROI reports.

use std::collections::HashMap;

use egui::{Align, Color32, CornerRadius, Frame, Layout, Margin, Stroke, Vec2, vec2};

use super::crm_sidecar::{format_currency, parse_hex_color};
use crate::app::App;
use crate::model::{Action, Dialog};
use crate::theme::{self, Icon, Palette};

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let tab_id = egui::Id::new("crm_metrics_selected_tab");
    let mut selected_tab: usize = ui.data_mut(|d| d.get_temp(tab_id)).unwrap_or(0);

    ui.vertical(|ui| {
        // Header
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::ListChecks, 20.0, palette.accent);
            theme::text(
                ui,
                "Dashboard Comercial & Atribuição de Anúncios",
                theme::bold(17.0),
                palette.text,
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::X, 18.0, palette.secondary, palette.text, "Fechar")
                    .clicked()
                {
                    app.dialog = None;
                }
            });
        });

        ui.add_space(8.0);

        // Tab Selector Bar
        ui.horizontal(|ui| {
            if ui
                .selectable_label(selected_tab == 0, "Funil Comercial & KPIs")
                .clicked()
            {
                selected_tab = 0;
            }
            if ui
                .selectable_label(selected_tab == 1, "Atribuição de Anúncios & ROI (CTWA)")
                .clicked()
            {
                selected_tab = 1;
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(8.0);

        if selected_tab == 0 {
            render_funnel_tab(app, ui, &palette);
        } else {
            render_attribution_tab(app, ui, &palette);
        }
    });

    ui.data_mut(|d| d.insert_temp(tab_id, selected_tab));
}

fn render_funnel_tab(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    // Filter valid deals that belong to currently active pipeline stages
    let valid_deals: Vec<&crate::model::CrmDeal> = app
        .crm_deals
        .values()
        .filter(|d| app.crm_columns.iter().any(|c| c.id == d.column_id))
        .collect();

    let total_deals = valid_deals.len();
    let total_value: i64 = valid_deals.iter().map(|d| d.value_cents).sum();
    let avg_ticket = if total_deals > 0 {
        total_value / total_deals as i64
    } else {
        0
    };

    // Closed deals: check "close" ID, titles with "fechamento" / "ganho", or fallback to the last funnel stage
    let closed_col = app
        .crm_columns
        .iter()
        .find(|c| {
            let t = c.title.to_lowercase();
            c.id == "close"
                || c.id == "won"
                || t.contains("fechamento")
                || t.contains("ganho")
                || t.contains("pago")
                || t.contains("venda")
        })
        .or_else(|| app.crm_columns.last());
    let closed_col_id = closed_col.map(|c| c.id.as_str()).unwrap_or("");
    let closed_col_title = closed_col.map(|c| c.title.as_str()).unwrap_or("Fechamento");

    let closed_deals: Vec<_> = valid_deals
        .iter()
        .filter(|d| d.column_id == closed_col_id)
        .collect();
    let closed_value: i64 = closed_deals.iter().map(|d| d.value_cents).sum();

    let now = crate::util::now();
    let pending_followups = app.crm_followups.iter().filter(|f| !f.done).count();
    let overdue_followups = app
        .crm_followups
        .iter()
        .filter(|f| !f.done && f.remind_at <= now)
        .count();

    // 1. KPI Cards Row
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        render_kpi_card(
            ui,
            palette,
            "Pipeline Total",
            &format_currency(total_value),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            &format!("Ganhos ({closed_col_title})"),
            &format_currency(closed_value),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            "Ticket Médio",
            &format_currency(avg_ticket),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            "Negócios Ativos",
            &format!("{total_deals}"),
            palette.text,
        );
        render_kpi_card(
            ui,
            palette,
            "Lembretes Pendentes",
            &format!("{pending_followups} ({overdue_followups} atrasados)"),
            if overdue_followups > 0 {
                palette.danger
            } else {
                palette.secondary
            },
        );
    });

    ui.add_space(16.0);
    theme::text(
        ui,
        "Distribuição e Volume por Etapa do Funil",
        theme::semibold(14.0),
        palette.text,
    );
    ui.add_space(6.0);

    // 2. Stage breakdown bars
    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;

            let columns = app.crm_columns.clone();
            for (idx, col) in columns.iter().enumerate() {
                let in_col: Vec<_> = valid_deals
                    .iter()
                    .filter(|d| d.column_id == col.id)
                    .collect();
                let count = in_col.len();
                let col_value: i64 = in_col.iter().map(|d| d.value_cents).sum();

                let percentage = if total_deals > 0 {
                    (count as f32 / total_deals as f32) * 100.0
                } else {
                    0.0
                };

                Frame::new()
                    .fill(palette.bubble_in)
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(10))
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let dot_color =
                                    parse_hex_color(&col.color).unwrap_or(palette.accent);
                                let (dot_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
                                ui.painter()
                                    .circle_filled(dot_rect.center(), 5.0, dot_color);

                                theme::text(
                                    ui,
                                    &format!("{}. {}", idx + 1, col.title),
                                    theme::semibold(13.0),
                                    palette.text,
                                );

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    theme::text(
                                        ui,
                                        &format_currency(col_value),
                                        theme::medium(12.5),
                                        palette.accent,
                                    );
                                    ui.add_space(8.0);
                                    theme::text(
                                        ui,
                                        &format!("{count} contatos ({percentage:.1}%)"),
                                        theme::regular(12.0),
                                        palette.dim,
                                    );
                                });
                            });

                            ui.add_space(4.0);

                            // Progress bar
                            let bar_width = ui.available_width().max(50.0);
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(bar_width, 8.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 4.0, palette.surface_hover);

                            let fill_width = (bar_width * (percentage / 100.0)).max(2.0);
                            let fill_rect =
                                egui::Rect::from_min_size(rect.min, vec2(fill_width, 8.0));
                            let bar_color = parse_hex_color(&col.color).unwrap_or(palette.accent);
                            ui.painter().rect_filled(fill_rect, 4.0, bar_color);
                        });
                    });
            }
        });
}

#[derive(Default)]
struct AdStats {
    leads_count: usize,
    sales_count: usize,
    revenue_cents: i64,
}

fn render_attribution_tab(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    let period_id = egui::Id::new("crm_attribution_period_filter");
    let mut period_days: u32 = ui.data_mut(|d| d.get_temp(period_id)).unwrap_or(30);

    // Period selector header
    ui.horizontal(|ui| {
        theme::text(ui, "Período:", theme::medium(13.0), palette.text);
        if ui
            .selectable_label(period_days == 7, "Últimos 7 dias")
            .clicked()
        {
            period_days = 7;
        }
        if ui
            .selectable_label(period_days == 30, "Últimos 30 dias")
            .clicked()
        {
            period_days = 30;
        }
        if ui
            .selectable_label(period_days == 0, "Todo o Período")
            .clicked()
        {
            period_days = 0;
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if theme::soft_button(
                ui,
                palette,
                Some(Icon::Settings),
                "Regras de Texto (Plano B)…",
                false,
            )
            .clicked()
            {
                app.actions
                    .push(Action::ShowDialog(Dialog::TextAttributionRules));
            }
        });
    });

    ui.data_mut(|d| d.insert_temp(period_id, period_days));
    ui.add_space(8.0);

    let now = crate::util::now();
    let cutoff_timestamp = if period_days == 0 {
        0
    } else {
        now - (period_days as i64 * 86400)
    };

    // Identify closed column
    let closed_col = app.crm_columns.iter().find(|c| {
        let t = c.title.to_lowercase();
        c.id == "close"
            || c.id == "won"
            || t.contains("fechamento")
            || t.contains("ganho")
            || t.contains("pago")
            || t.contains("venda")
    });
    let closed_col_id = closed_col.map(|c| c.id.as_str()).unwrap_or("");

    // Aggregate statistics by campaign and ad
    // Key: (Campaign, AdName)
    let mut ad_groups: HashMap<(String, String), AdStats> = HashMap::new();
    let mut unidentified_stats = AdStats::default();

    // Scan all chats or deals
    for (chat_id, deal) in &app.crm_deals {
        let origin = app.contact_origins.get(chat_id);
        let contact_time = origin
            .map(|o| o.first_contact_at)
            .unwrap_or(deal.updated_at);

        if cutoff_timestamp > 0 && contact_time < cutoff_timestamp {
            continue;
        }

        let is_won = deal.column_id == closed_col_id;
        let revenue = if is_won { deal.value_cents } else { 0 };

        if let Some(orig) = origin.filter(|o| o.is_identified()) {
            let camp = if orig.campaign_name.trim().is_empty() {
                "Campanha não mapeada".to_string()
            } else {
                orig.campaign_name.clone()
            };
            let ad = orig.display_label();
            let entry = ad_groups.entry((camp, ad.to_string())).or_default();
            entry.leads_count += 1;
            if is_won {
                entry.sales_count += 1;
                entry.revenue_cents += revenue;
            }
        } else {
            unidentified_stats.leads_count += 1;
            if is_won {
                unidentified_stats.sales_count += 1;
                unidentified_stats.revenue_cents += revenue;
            }
        }
    }

    // Totals for Ads
    let total_ad_leads: usize = ad_groups.values().map(|s| s.leads_count).sum();
    let total_ad_sales: usize = ad_groups.values().map(|s| s.sales_count).sum();
    let total_ad_revenue: i64 = ad_groups.values().map(|s| s.revenue_cents).sum();
    let ad_conv_rate = if total_ad_leads > 0 {
        (total_ad_sales as f64 / total_ad_leads as f64) * 100.0
    } else {
        0.0
    };

    let unid_conv_rate = if unidentified_stats.leads_count > 0 {
        (unidentified_stats.sales_count as f64 / unidentified_stats.leads_count as f64) * 100.0
    } else {
        0.0
    };

    // KPI Cards for Ads
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        render_kpi_card(
            ui,
            palette,
            "Receita de Anúncios",
            &format_currency(total_ad_revenue),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            "Vendas de Anúncios",
            &format!("{total_ad_sales} vendas"),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            "Leads de Anúncios",
            &format!("{total_ad_leads} leads"),
            palette.text,
        );
        render_kpi_card(
            ui,
            palette,
            "Taxa de Conversão",
            &format!("{ad_conv_rate:.1}%"),
            palette.accent,
        );
        render_kpi_card(
            ui,
            palette,
            "Não Identificado / Orgânico",
            &format!(
                "{} leads • {}",
                unidentified_stats.leads_count,
                format_currency(unidentified_stats.revenue_cents)
            ),
            palette.dim,
        );
    });

    ui.add_space(14.0);
    theme::text(
        ui,
        "Performance por Campanha & Anúncio",
        theme::semibold(14.0),
        palette.text,
    );
    ui.add_space(6.0);

    // Attribution Data Table / Cards
    egui::ScrollArea::vertical()
        .max_height(280.0)
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;

            if ad_groups.is_empty() && unidentified_stats.leads_count == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        egui::RichText::new(
                            "Nenhum lead com negócio registrado no período selecionado.",
                        )
                        .italics()
                        .color(palette.dim),
                    );
                    ui.add_space(20.0);
                });
            } else {
                // Table header
                Frame::new()
                    .fill(palette.surface_hover)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.columns(5, |cols| {
                            cols[0].label(
                                egui::RichText::new("Campanha / Anúncio")
                                    .strong()
                                    .size(11.5)
                                    .color(palette.text),
                            );
                            cols[1].label(
                                egui::RichText::new("Leads")
                                    .strong()
                                    .size(11.5)
                                    .color(palette.text),
                            );
                            cols[2].label(
                                egui::RichText::new("Vendas")
                                    .strong()
                                    .size(11.5)
                                    .color(palette.text),
                            );
                            cols[3].label(
                                egui::RichText::new("Conv. %")
                                    .strong()
                                    .size(11.5)
                                    .color(palette.text),
                            );
                            cols[4].label(
                                egui::RichText::new("Receita")
                                    .strong()
                                    .size(11.5)
                                    .color(palette.text),
                            );
                        });
                    });

                // Sort ads by revenue descending
                let mut sorted_ads: Vec<_> = ad_groups.into_iter().collect();
                sorted_ads.sort_by_key(|(_, stats)| std::cmp::Reverse(stats.revenue_cents));

                for ((campaign, ad), stats) in sorted_ads {
                    let conv = if stats.leads_count > 0 {
                        (stats.sales_count as f64 / stats.leads_count as f64) * 100.0
                    } else {
                        0.0
                    };

                    Frame::new()
                        .fill(palette.bubble_in)
                        .stroke(Stroke::new(1.0, palette.outline))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.columns(5, |cols| {
                                cols[0].vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new(ad)
                                            .strong()
                                            .size(12.0)
                                            .color(palette.text),
                                    );
                                    ui.label(
                                        egui::RichText::new(campaign).size(10.5).color(palette.dim),
                                    );
                                });
                                cols[1].label(
                                    egui::RichText::new(format!("{}", stats.leads_count))
                                        .size(12.0),
                                );
                                cols[2].label(
                                    egui::RichText::new(format!("{}", stats.sales_count))
                                        .size(12.0),
                                );
                                cols[3].label(
                                    egui::RichText::new(format!("{conv:.1}%"))
                                        .size(12.0)
                                        .color(palette.accent),
                                );
                                cols[4].label(
                                    egui::RichText::new(format_currency(stats.revenue_cents))
                                        .strong()
                                        .size(12.0)
                                        .color(palette.accent),
                                );
                            });
                        });
                }

                // Row for Unidentified / Organic comparison
                Frame::new()
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0, palette.surface_hover))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        ui.columns(5, |cols| {
                            cols[0].vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Não identificado / Orgânico")
                                        .strong()
                                        .size(12.0)
                                        .color(palette.dim),
                                );
                                ui.label(
                                    egui::RichText::new("Sem anúncio ou parâmetros de rastreio")
                                        .size(10.5)
                                        .color(palette.dim),
                                );
                            });
                            cols[1].label(
                                egui::RichText::new(format!("{}", unidentified_stats.leads_count))
                                    .size(12.0)
                                    .color(palette.dim),
                            );
                            cols[2].label(
                                egui::RichText::new(format!("{}", unidentified_stats.sales_count))
                                    .size(12.0)
                                    .color(palette.dim),
                            );
                            cols[3].label(
                                egui::RichText::new(format!("{unid_conv_rate:.1}%"))
                                    .size(12.0)
                                    .color(palette.dim),
                            );
                            cols[4].label(
                                egui::RichText::new(format_currency(
                                    unidentified_stats.revenue_cents,
                                ))
                                .size(12.0)
                                .color(palette.dim),
                            );
                        });
                    });
            }
        });
}

fn render_kpi_card(
    ui: &mut egui::Ui,
    palette: &Palette,
    label: &str,
    value: &str,
    accent: Color32,
) {
    Frame::new()
        .fill(palette.bubble_in)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(115.0);
            ui.vertical(|ui| {
                theme::text(ui, label, theme::regular(11.0), palette.dim);
                ui.add_space(2.0);
                theme::text(ui, value, theme::bold(13.5), accent);
            });
        });
}
