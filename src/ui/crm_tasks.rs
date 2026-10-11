//! Account-scoped CRM task and follow-up inbox.

use crate::app::App;
use crate::model::{Action, CrmFollowup, CrmTask};
use crate::theme::{self, Icon};
use crate::ui::crm_workspace::{CrmPalette, empty_state};
use egui::{
    Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2, vec2,
};
use jiff::civil::Date;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum TaskView {
    #[default]
    Today,
    Tasks,
    Calendar,
}

#[derive(Clone)]
struct TaskDraft {
    id: Option<String>,
    title: String,
    description: String,
    priority: String,
    status: String,
    assignee: String,
    kind: String,
    duration_minutes: u16,
    call_result: String,
    next_step: String,
    revenue_reais: String,
    project_id: Option<String>,
    chat_id: Option<String>,
    has_due: bool,
    due_date: Date,
    due_hour: u8,
    due_minute: u8,
    link_chat: bool,
}
impl Default for TaskDraft {
    fn default() -> Self {
        Self {
            id: None,
            title: String::new(),
            description: String::new(),
            priority: "normal".into(),
            status: "todo".into(),
            assignee: String::new(),
            kind: "task".into(),
            duration_minutes: 30,
            call_result: String::new(),
            next_step: String::new(),
            revenue_reais: String::new(),
            project_id: None,
            chat_id: None,
            has_due: true,
            due_date: crate::util::today()
                .tomorrow()
                .unwrap_or_else(|_| crate::util::today()),
            due_hour: 9,
            due_minute: 0,
            link_chat: true,
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    show_inner(app, ui, true, None);
}

/// Draws the task center as part of the full CRM page instead of a modal.
pub fn show_workspace(app: &mut App, ui: &mut egui::Ui) {
    let cp = CrmPalette::from_app(&app.palette);
    show_task_board(app, ui, &cp);
}

pub fn show_agenda_workspace(app: &mut App, ui: &mut egui::Ui) {
    let cp = CrmPalette::from_app(&app.palette);
    let account_id = app.id.as_str().to_owned();
    let draft_id = ui.id().with(("crm-task-draft", &account_id));
    let calendar_id = ui.id().with(("crm-calendar-open", &account_id));
    let mut draft = ui.data_mut(|d| d.get_temp::<TaskDraft>(draft_id).unwrap_or_default());
    let mut calendar_open = ui.data_mut(|d| d.get_temp::<bool>(calendar_id).unwrap_or(false));

    ui.horizontal(|ui| {
        let (btn_fill, btn_color) = if calendar_open {
            (cp.accent, cp.on_accent)
        } else {
            (cp.surface, cp.text)
        };
        let toggle_btn = egui::Button::new(
            RichText::new(if calendar_open {
                "Fechar calendário"
            } else {
                "📅 Escolher data"
            })
            .size(13.0)
            .strong()
            .color(btn_color),
        )
        .fill(btn_fill)
        .stroke(Stroke::new(
            1.0,
            if calendar_open { cp.accent } else { cp.outline },
        ))
        .corner_radius(CornerRadius::same(8))
        .min_size(vec2(140.0, 32.0));

        if ui.add(toggle_btn).clicked() {
            calendar_open = !calendar_open;
        }

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "O calendário expande somente quando você desejar navegar por outros meses e dias.",
            )
            .size(12.5)
            .color(cp.dim),
        );
    });
    ui.add_space(10.0);
    if calendar_open {
        show_calendar(app, ui, crate::util::now(), &mut draft);
    } else {
        show_day(
            app,
            ui,
            crate::util::today(),
            crate::util::now(),
            &mut draft,
        );
    }
    ui.data_mut(|d| {
        d.insert_temp(draft_id, draft);
        d.insert_temp(calendar_id, calendar_open);
    });
}

/// Sales-call dashboard with account-scoped indicators and records.
/// Sales-call dashboard with account-scoped indicators and records.
pub fn show_calls_workspace(app: &mut App, ui: &mut egui::Ui) {
    let cp = CrmPalette::from_app(&app.palette);
    let now = crate::util::now();
    let account_id = app.id.as_str().to_owned();
    let period_id = ui.id().with(("crm-call-period", &account_id));
    let form_id = ui.id().with(("crm-call-form", &account_id));
    let draft_id = ui.id().with(("crm-call-draft", &account_id));
    let search_id = ui.id().with(("crm-call-search", &account_id));
    let status_filter_id = ui.id().with(("crm-call-status-filter", &account_id));

    let mut period = ui.data_mut(|d| d.get_temp::<i64>(period_id).unwrap_or(90));
    let mut show_form = ui.data_mut(|d| d.get_temp::<bool>(form_id).unwrap_or(false));
    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
    let mut status_filter = ui.data_mut(|d| {
        d.get_temp::<String>(status_filter_id)
            .unwrap_or_else(|| "all".into())
    });
    let mut draft = ui.data_mut(|d| {
        d.get_temp::<TaskDraft>(draft_id)
            .unwrap_or_else(|| TaskDraft {
                kind: "call".into(),
                status: "scheduled".into(),
                ..TaskDraft::default()
            })
    });
    draft.kind = "call".into();

    let cutoff = (period > 0).then_some(now - period * 86_400);
    let all_calls: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|task| {
            matches!(task.kind.as_str(), "call" | "conversation")
                && cutoff.is_none_or(|cutoff| task.due_at.unwrap_or(task.created_at) >= cutoff)
        })
        .cloned()
        .collect();

    let total = all_calls.len();
    let scheduled = all_calls
        .iter()
        .filter(|call| {
            matches!(
                call.status.as_str(),
                "scheduled" | "todo" | "doing" | "review"
            )
        })
        .count();
    let held = all_calls
        .iter()
        .filter(|call| matches!(call.status.as_str(), "held" | "done" | "closed"))
        .count();
    let no_show = all_calls
        .iter()
        .filter(|call| call.status == "no_show")
        .count();
    let cancelled = all_calls
        .iter()
        .filter(|call| call.status == "cancelled")
        .count();
    let closings = all_calls
        .iter()
        .filter(|call| call.status == "closed" || call.revenue_cents > 0)
        .count();
    let revenue: i64 = all_calls.iter().map(|call| call.revenue_cents).sum();
    let conversion = if held == 0 {
        0.0
    } else {
        closings as f32 * 100.0 / held as f32
    };

    // 1. KPI Indicator Cards with Icons and Tailored Colors
    let avail_w = ui.available_width();
    let metric_w = ((avail_w - 7.0 * 8.0) / 8.0).max(115.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
        call_metric(
            ui,
            &cp,
            Icon::Phone,
            "Total",
            total.to_string(),
            cp.text,
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::Calendar,
            "Agendadas",
            scheduled.to_string(),
            Color32::from_rgb(0x60, 0xa5, 0xfa),
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::CircleCheck,
            "Realizadas",
            held.to_string(),
            Color32::from_rgb(0x34, 0xd3, 0x99),
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::CircleAlert,
            "No-show",
            no_show.to_string(),
            cp.warning,
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::Ban,
            "Canceladas",
            cancelled.to_string(),
            cp.danger,
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::CheckCheck,
            "Fechamentos",
            closings.to_string(),
            cp.accent,
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::Tag,
            "Conversão",
            format!("{conversion:.1}%"),
            cp.accent,
            metric_w,
        );
        call_metric(
            ui,
            &cp,
            Icon::ListChecks,
            "Receita",
            format_brl(revenue),
            cp.accent,
            metric_w,
        );
    });

    ui.add_space(14.0);

    // 2. Charts & Analytic Breakdown Cards (Side by Side)
    ui.columns(2, |uis| {
        // Card 1: Distribuição & Resultados
        Frame::new()
            .fill(cp.panel)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(&mut uis[0], |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleCheck, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Distribuição de Resultados")
                            .size(14.0)
                            .strong()
                            .color(cp.text),
                    );
                });
                ui.add_space(8.0);

                let held_pct = if total == 0 {
                    0.0
                } else {
                    held as f32 / total as f32
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Taxa de Realização:")
                            .size(12.0)
                            .color(cp.secondary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{held}/{total} ({:.0}%)", held_pct * 100.0))
                                .size(12.0)
                                .strong()
                                .color(cp.text),
                        );
                    });
                });
                ui.add_space(2.0);
                ui.add_sized(
                    [ui.available_width(), 6.0],
                    egui::ProgressBar::new(held_pct)
                        .fill(Color32::from_rgb(0x34, 0xd3, 0x99))
                        .animate(false),
                );

                ui.add_space(8.0);
                let close_pct = if held == 0 {
                    0.0
                } else {
                    closings as f32 / held as f32
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Conversão em Venda:")
                            .size(12.0)
                            .color(cp.secondary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{closings}/{held} ({:.1}%)", close_pct * 100.0))
                                .size(12.0)
                                .strong()
                                .color(cp.accent),
                        );
                    });
                });
                ui.add_space(2.0);
                ui.add_sized(
                    [ui.available_width(), 6.0],
                    egui::ProgressBar::new(close_pct)
                        .fill(cp.accent)
                        .animate(false),
                );
            });

        // Card 2: Bloco de Cancelamentos & No-Shows
        Frame::new()
            .fill(cp.panel)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(&mut uis[1], |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleAlert, 16.0, cp.danger);
                    ui.label(
                        RichText::new("Perdas: Cancelamentos & No-Shows")
                            .size(14.0)
                            .strong()
                            .color(cp.text),
                    );
                });
                ui.add_space(8.0);

                let lost_total = no_show + cancelled;
                let loss_pct = if total == 0 {
                    0.0
                } else {
                    lost_total as f32 / total as f32
                };
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Perda total: {lost_total} calls ({:.0}%)",
                            loss_pct * 100.0
                        ))
                        .size(12.0)
                        .color(cp.secondary),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let alert_label = if loss_pct > 0.25 {
                            "Taxa alta"
                        } else {
                            "Controlada"
                        };
                        let alert_color = if loss_pct > 0.25 {
                            cp.danger
                        } else {
                            cp.warning
                        };
                        Frame::new()
                            .fill(alert_color.gamma_multiply(0.15))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(alert_label)
                                        .size(11.0)
                                        .strong()
                                        .color(alert_color),
                                );
                            });
                    });
                });
                ui.add_space(4.0);
                ui.add_sized(
                    [ui.available_width(), 6.0],
                    egui::ProgressBar::new(loss_pct)
                        .fill(cp.danger)
                        .animate(false),
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "• {no_show} No-show(s)  • {cancelled} Cancelada(s)"
                        ))
                        .size(11.5)
                        .color(cp.dim),
                    );
                });
            });
    });

    ui.add_space(14.0);

    // 3. Filters & Action Bar
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("Buscar por título, cliente, responsável, resultado…")
                .desired_width(280.0),
        );

        egui::ComboBox::from_id_salt(("calls-period-filter", &account_id))
            .selected_text(if period == 0 {
                "Todo o período".into()
            } else {
                format!("Últimos {period} dias")
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut period, 30, "Últimos 30 dias");
                ui.selectable_value(&mut period, 90, "Últimos 90 dias");
                ui.selectable_value(&mut period, 365, "Último ano");
                ui.selectable_value(&mut period, 0, "Todo o período");
            });

        let status_text = match status_filter.as_str() {
            "scheduled" => "Agendadas",
            "held" => "Realizadas",
            "no_show" => "No-show",
            "cancelled" => "Canceladas",
            "closed" => "Fechamentos",
            _ => "Todos os status",
        };
        egui::ComboBox::from_id_salt(("calls-status-filter", &account_id))
            .selected_text(status_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut status_filter, "all".into(), "Todos os status");
                ui.selectable_value(&mut status_filter, "scheduled".into(), "Agendadas");
                ui.selectable_value(&mut status_filter, "held".into(), "Realizadas");
                ui.selectable_value(&mut status_filter, "no_show".into(), "No-show");
                ui.selectable_value(&mut status_filter, "cancelled".into(), "Canceladas");
                ui.selectable_value(&mut status_filter, "closed".into(), "Fechamentos");
            });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(14.0);
            let new_call_btn = egui::Button::new(
                RichText::new("+ Nova call")
                    .size(13.5)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(130.0, 36.0));

            if ui.add(new_call_btn).clicked() {
                draft = TaskDraft {
                    kind: "call".into(),
                    status: "scheduled".into(),
                    ..TaskDraft::default()
                };
                show_form = true;
            }

            ui.add_space(8.0);

            let export_csv_btn = egui::Button::new(
                RichText::new("📥 Exportar CSV")
                    .size(13.0)
                    .strong()
                    .color(cp.text),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(130.0, 36.0));

            if ui
                .add(export_csv_btn)
                .on_hover_text("Exportar todas as calls do período para planilha CSV")
                .clicked()
            {
                export_calls_csv(app, &all_calls);
            }
        });
    });

    ui.add_space(14.0);

    // 4. Floating Modal Window for Call Create/Edit
    if show_form {
        let mut is_open = true;
        let title = if draft.id.is_some() {
            "Editar Call de Venda"
        } else {
            "Nova Call de Venda"
        };
        egui::Window::new(title)
            .id(ui.id().with("crm-call-floating-modal"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(480.0, 540.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                render_call_modal(app, ui, &cp, &mut draft, now, &mut show_form);
            });
        if !is_open {
            show_form = false;
        }
    }

    // 5. Calls List with Colored Status Pills
    let needle = search.trim().to_lowercase();
    let filtered_calls: Vec<_> = all_calls
        .into_iter()
        .filter(|c| {
            (status_filter == "all"
                || (status_filter == "scheduled"
                    && matches!(c.status.as_str(), "scheduled" | "todo" | "doing" | "review"))
                || (status_filter == "held" && matches!(c.status.as_str(), "held" | "done"))
                || (status_filter == "closed" && (c.status == "closed" || c.revenue_cents > 0))
                || c.status == status_filter)
                && (needle.is_empty()
                    || c.title.to_lowercase().contains(&needle)
                    || c.assignee.to_lowercase().contains(&needle)
                    || c.call_result.to_lowercase().contains(&needle)
                    || c.next_step.to_lowercase().contains(&needle)
                    || c.description.to_lowercase().contains(&needle)
                    || c.chat_id
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&needle))
        })
        .collect();

    if filtered_calls.is_empty() {
        if empty_state(
            ui,
            &cp,
            Icon::Phone,
            "Nenhuma call de venda encontrada",
            "Agende reuniões com clientes e registre resultados e fechamentos.",
            Some("+ Nova call"),
        ) {
            draft = TaskDraft {
                kind: "call".into(),
                status: "scheduled".into(),
                ..TaskDraft::default()
            };
            show_form = true;
            ui.data_mut(|d| d.insert_temp(form_id, true));
        }
    } else {
        ScrollArea::vertical()
            .id_salt("crm_calls_scroll")
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for call in filtered_calls {
                    render_call_card(app, ui, &cp, &call, now, &mut draft, &mut show_form);
                }
            });
    }

    ui.data_mut(|d| {
        d.insert_temp(period_id, period);
        d.insert_temp(form_id, show_form);
        d.insert_temp(draft_id, draft);
        d.insert_temp(search_id, search);
        d.insert_temp(status_filter_id, status_filter);
    });
}

fn call_metric(
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    icon: Icon,
    title: &str,
    value: String,
    val_color: Color32,
    width: f32,
) {
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.horizontal(|ui| {
                theme::icon(ui, icon, 16.0, val_color);
                ui.add_space(2.0);
                ui.label(RichText::new(title).size(12.0).color(cp.secondary));
            });
            ui.add_space(6.0);
            ui.label(RichText::new(&value).size(22.0).strong().color(val_color));
        });
}

fn call_status_pill(ui: &mut egui::Ui, status: &str, cp: &CrmPalette) {
    let (bg, fg, label) = match status {
        "scheduled" | "todo" | "doing" | "review" => (
            Color32::from_rgb(0x1e, 0x29, 0x3b),
            Color32::from_rgb(0x60, 0xa5, 0xfa),
            "Agendada",
        ),
        "held" | "done" => (
            Color32::from_rgb(0x06, 0x4e, 0x3b),
            Color32::from_rgb(0x34, 0xd3, 0x99),
            "Realizada",
        ),
        "no_show" => (
            Color32::from_rgb(0x45, 0x1a, 0x03),
            Color32::from_rgb(0xfb, 0xbf, 0x24),
            "No-show",
        ),
        "cancelled" => (
            Color32::from_rgb(0x45, 0x0a, 0x0a),
            Color32::from_rgb(0xf8, 0x71, 0x71),
            "Cancelada",
        ),
        "closed" => (cp.accent.gamma_multiply(0.18), cp.accent, "Fechamento"),
        _ => (cp.surface_hover, cp.dim, status),
    };
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(5))
        .inner_margin(Margin::symmetric(7, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.5).strong().color(fg));
        });
}

fn render_call_card(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    call: &CrmTask,
    _now: i64,
    draft: &mut TaskDraft,
    show_form: &mut bool,
) {
    let client_name = call
        .chat_id
        .as_ref()
        .and_then(|id| app.chat(id))
        .map(|c| app.chat_title(c));

    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    call_status_pill(ui, &call.status, cp);

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(&call.title)
                            .size(14.5)
                            .strong()
                            .color(cp.text),
                    );

                    if let Some(duration) = call.duration_minutes {
                        Frame::new()
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!("⏱ {duration} min"))
                                        .size(11.0)
                                        .color(cp.secondary),
                                );
                            });
                    }

                    if call.revenue_cents > 0 {
                        Frame::new()
                            .fill(cp.accent.gamma_multiply(0.15))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format_brl(call.revenue_cents))
                                        .size(11.5)
                                        .strong()
                                        .color(cp.accent),
                                );
                            });
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Editar").clicked() {
                            *draft = TaskDraft {
                                id: Some(call.id.clone()),
                                title: call.title.clone(),
                                description: call.description.clone(),
                                priority: call.priority.clone(),
                                status: call.status.clone(),
                                assignee: call.assignee.clone(),
                                kind: "call".into(),
                                duration_minutes: call.duration_minutes.unwrap_or(30),
                                call_result: call.call_result.clone(),
                                next_step: call.next_step.clone(),
                                revenue_reais: format_revenue_input(call.revenue_cents),
                                project_id: call.project_id.clone(),
                                has_due: call.due_at.is_some(),
                                due_date: call
                                    .due_at
                                    .and_then(crate::util::local_datetime)
                                    .map(|x| x.0)
                                    .unwrap_or_else(crate::util::today),
                                due_hour: call
                                    .due_at
                                    .and_then(crate::util::local_datetime)
                                    .map(|x| x.1)
                                    .unwrap_or(9),
                                due_minute: call
                                    .due_at
                                    .and_then(crate::util::local_datetime)
                                    .map(|x| x.2)
                                    .unwrap_or(0),
                                chat_id: call.chat_id.clone(),
                                link_chat: call.chat_id.is_some(),
                            };
                            *show_form = true;
                        }

                        let delete_id = egui::Id::new(("call-delete-confirm", &call.id));
                        let is_confirming =
                            ui.data_mut(|d| d.get_temp::<bool>(delete_id).unwrap_or(false));
                        if is_confirming {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Cancelar").size(12.0).color(cp.secondary),
                                    )
                                    .fill(cp.surface_hover)
                                    .corner_radius(CornerRadius::same(5)),
                                )
                                .clicked()
                            {
                                ui.data_mut(|d| d.insert_temp(delete_id, false));
                            }
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Excluir")
                                            .size(12.0)
                                            .strong()
                                            .color(Color32::WHITE),
                                    )
                                    .fill(cp.danger)
                                    .corner_radius(CornerRadius::same(5)),
                                )
                                .clicked()
                            {
                                app.actions.push(Action::DeleteCrmTask(call.id.clone()));
                                ui.data_mut(|d| d.insert_temp(delete_id, false));
                            }
                        } else if ui
                            .add(
                                egui::Button::new(RichText::new("Excluir").size(12.0).color(cp.danger))
                                    .fill(cp.surface_hover)
                                    .stroke(Stroke::new(1.0, cp.outline))
                                    .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            ui.data_mut(|d| d.insert_temp(delete_id, true));
                        }

                        if let Some(chat) = &call.chat_id {
                            if ui.small_button("Conversar").clicked() {
                                app.actions.push(Action::OpenChat(chat.clone()));
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                // Meta row: Date, Client, Assignee, Project
                ui.horizontal_wrapped(|ui| {
                    if let Some(d) = call.due_at {
                        theme::icon(ui, Icon::Clock, 13.0, cp.dim);
                        ui.label(RichText::new(format_when(d)).size(11.5).color(cp.secondary));
                        ui.add_space(8.0);
                    }
                    if let Some(c) = &client_name {
                        theme::icon(ui, Icon::Users, 13.0, cp.dim);
                        ui.label(
                            RichText::new(format!("Cliente: {c}"))
                                .size(11.5)
                                .color(cp.secondary),
                        );
                        ui.add_space(8.0);
                    }
                    if !call.assignee.is_empty() {
                        theme::icon(ui, Icon::User, 13.0, cp.dim);
                        ui.label(
                            RichText::new(format!("Responsável: {}", call.assignee))
                                .size(11.5)
                                .color(cp.dim),
                        );
                        ui.add_space(8.0);
                    }
                    if let Some(project) = call
                        .project_id
                        .as_ref()
                        .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                    {
                        theme::icon(ui, Icon::ListChecks, 13.0, cp.accent);
                        ui.label(
                            RichText::new(format!("Projeto: {}", project.name))
                                .size(11.5)
                                .color(cp.secondary),
                        );
                    }
                });

                if !call.call_result.is_empty() || !call.next_step.is_empty() {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if !call.call_result.is_empty() {
                            ui.label(
                                RichText::new(format!("Resultado: {}", call.call_result))
                                    .size(11.5)
                                    .color(cp.accent),
                            );
                            ui.add_space(10.0);
                        }
                        if !call.next_step.is_empty() {
                            ui.label(
                                RichText::new(format!("Próximo passo: {}", call.next_step))
                                    .size(11.5)
                                    .color(cp.secondary),
                            );
                        }
                    });
                }

                if !call.description.is_empty() {
                    ui.add_space(2.0);
                    ui.label(RichText::new(&call.description).size(11.5).color(cp.dim));
                }
            });
        });
}

fn render_call_modal(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    draft: &mut TaskDraft,
    now: i64,
    show_form: &mut bool,
) {
    let err_id = ui.id().with("crm-call-modal-err");
    let show_err = ui.data_mut(|d| d.get_temp::<bool>(err_id).unwrap_or(false));

    ui.vertical(|ui| {
        ui.label(
            RichText::new("Título da call *")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let title_stroke = if show_err && draft.title.trim().is_empty() {
            Stroke::new(1.0, cp.danger)
        } else {
            Stroke::new(1.0, cp.outline)
        };
        Frame::new()
            .fill(cp.surface)
            .stroke(title_stroke)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut draft.title)
                        .hint_text("Ex.: Reunião de Alinhamento Comercial")
                        .desired_width(ui.available_width())
                        .frame(Frame::NONE),
                );
            });
        if show_err && draft.title.trim().is_empty() {
            ui.label(
                RichText::new("O título é obrigatório")
                    .size(11.0)
                    .color(cp.danger),
            );
        }

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Status da call")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                egui::ComboBox::from_id_salt("modal-call-status")
                    .selected_text(status_label(&draft.status))
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for (value, label) in [
                            ("scheduled", "Agendada"),
                            ("held", "Realizada"),
                            ("no_show", "No-show"),
                            ("cancelled", "Cancelada"),
                            ("closed", "Fechamento"),
                        ] {
                            ui.selectable_value(&mut draft.status, value.into(), label);
                        }
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Duração")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                egui::ComboBox::from_id_salt("modal-call-duration")
                    .selected_text(format!("{} min", draft.duration_minutes))
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        for minutes in [15, 30, 45, 60, 90, 120] {
                            ui.selectable_value(
                                &mut draft.duration_minutes,
                                minutes,
                                format!("{minutes} min"),
                            );
                        }
                    });
            });
        });

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Cliente / Contato")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                let current_client = draft
                    .chat_id
                    .as_ref()
                    .and_then(|id| app.chat(id))
                    .map(|c| app.chat_title(c))
                    .unwrap_or_else(|| {
                        if draft.chat_id.is_some() {
                            "Cliente selecionado".into()
                        } else {
                            "Nenhum cliente (Geral)".into()
                        }
                    });

                egui::ComboBox::from_id_salt("modal-call-client")
                    .selected_text(current_client)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.chat_id, None, "Nenhum cliente (Geral)");
                        for chat in &app.chats {
                            let title = app.chat_title(chat);
                            ui.selectable_value(&mut draft.chat_id, Some(chat.id.clone()), title);
                        }
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Projeto vinculado")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                let current_proj_name = draft
                    .project_id
                    .as_ref()
                    .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                    .map_or("Sem projeto", |p| p.name.as_str());

                egui::ComboBox::from_id_salt("modal-call-project")
                    .selected_text(current_proj_name)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.project_id, None, "Sem projeto");
                        for p in &app.crm_projects {
                            ui.selectable_value(&mut draft.project_id, Some(p.id.clone()), &p.name);
                        }
                    });
            });
        });

        ui.add_space(10.0);

        ui.vertical(|ui| {
            ui.label(
                RichText::new("Responsável")
                    .size(12.0)
                    .strong()
                    .color(cp.secondary),
            );
            ui.add_space(2.0);
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.assignee)
                            .hint_text("Nome do consultor")
                            .desired_width(ui.available_width())
                            .frame(Frame::NONE),
                    );
                });
        });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Data e horário da call")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let cal_open_id = ui.id().with("modal-call-cal-open");
        let mut cal_open = ui.data_mut(|d| d.get_temp::<bool>(cal_open_id).unwrap_or(false));
        let date_str = format!(
            "📅 {:02}/{:02}/{} às {:02}:{:02}",
            draft.due_date.day(),
            draft.due_date.month(),
            draft.due_date.year(),
            draft.due_hour,
            draft.due_minute
        );
        if ui
            .button(RichText::new(date_str).color(cp.accent))
            .clicked()
        {
            cal_open = !cal_open;
        }

        if cal_open {
            ui.add_space(4.0);
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    super::dialogs::date_time_picker(
                        ui,
                        app.locale,
                        &app.palette,
                        "crm-sales-call-date",
                        &mut draft.due_date,
                        &mut draft.due_hour,
                        &mut draft.due_minute,
                    );
                });
        }
        ui.data_mut(|d| d.insert_temp(cal_open_id, cal_open));

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Resultado da call")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                Frame::new()
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut draft.call_result)
                                .hint_text("Ex.: Apresentada proposta")
                                .desired_width(ui.available_width())
                                .frame(Frame::NONE),
                        );
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Receita fechada (R$)")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                Frame::new()
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut draft.revenue_reais)
                                .hint_text("Ex.: 2500,00")
                                .desired_width(ui.available_width())
                                .frame(Frame::NONE),
                        );
                    });
            });
        });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Próximo passo comercial")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut draft.next_step)
                        .hint_text("Ex.: Follow-up na sexta-feira às 14h")
                        .desired_width(ui.available_width())
                        .frame(Frame::NONE),
                );
            });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Anotações da reunião")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut draft.description)
                        .hint_text("Pontos discutidos, dores do cliente, objeções...")
                        .desired_width(ui.available_width())
                        .desired_rows(2)
                        .frame(Frame::NONE),
                );
            });

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Cancelar").size(13.0).color(cp.secondary))
                        .fill(cp.surface)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(100.0, 36.0)),
                )
                .clicked()
            {
                *show_form = false;
                *draft = TaskDraft {
                    kind: "call".into(),
                    status: "scheduled".into(),
                    ..TaskDraft::default()
                };
                ui.data_mut(|d| d.insert_temp(err_id, false));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    RichText::new("Salvar call")
                        .size(13.0)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(140.0, 36.0));

                if ui.add(save_btn).clicked() {
                    if draft.title.trim().is_empty() {
                        ui.data_mut(|d| d.insert_temp(err_id, true));
                        return;
                    }

                    let old = draft
                        .id
                        .as_ref()
                        .and_then(|id| app.crm_tasks.iter().find(|task| &task.id == id));

                    app.actions.push(Action::SaveCrmTask(CrmTask {
                        id: draft.id.clone().unwrap_or_else(new_task_id),
                        chat_id: draft.chat_id.clone().or_else(|| app.open_chat.clone()),
                        project_id: draft.project_id.clone(),
                        title: draft.title.trim().into(),
                        description: draft.description.trim().into(),
                        status: draft.status.clone(),
                        priority: "normal".into(),
                        kind: "call".into(),
                        duration_minutes: Some(draft.duration_minutes),
                        assignee: draft.assignee.trim().into(),
                        call_result: draft.call_result.trim().into(),
                        next_step: draft.next_step.trim().into(),
                        revenue_cents: parse_revenue_cents(&draft.revenue_reais),
                        due_at: crate::util::to_unix_seconds(
                            draft.due_date,
                            draft.due_hour,
                            draft.due_minute,
                        ),
                        completed_at: matches!(
                            draft.status.as_str(),
                            "held" | "no_show" | "cancelled" | "closed"
                        )
                        .then_some(now),
                        created_at: old.map_or(now, |task| task.created_at),
                        updated_at: now,
                    }));

                    *draft = TaskDraft {
                        kind: "call".into(),
                        status: "scheduled".into(),
                        ..TaskDraft::default()
                    };
                    *show_form = false;
                    ui.data_mut(|d| d.insert_temp(err_id, false));
                }
            });
        });
    });
}

fn format_brl(cents: i64) -> String {
    let value = cents as f64 / 100.0;
    format!("R$ {:.2}", value).replace('.', ",")
}

fn show_inner(app: &mut App, ui: &mut egui::Ui, modal: bool, forced: Option<TaskView>) {
    let p = app.palette;
    let now = crate::util::now();
    let account_id = app.id.as_str().to_owned();
    let draft_id = ui.id().with(("crm-task-draft", &account_id));
    let view_id = ui.id().with(("crm-task-view", &account_id));
    let mut view = ui.data_mut(|d| d.get_temp::<TaskView>(view_id).unwrap_or_default());
    if let Some(forced) = forced {
        view = forced;
    }
    let mut draft = ui.data_mut(|d| d.get_temp::<TaskDraft>(draft_id).unwrap_or_default());
    let mut followups: Vec<_> = app
        .crm_followups
        .iter()
        .filter(|x| !x.done)
        .cloned()
        .collect();
    followups.sort_by_key(|x| x.remind_at);
    let open = app.crm_tasks.iter().filter(|x| x.status != "done").count();
    let done = app.crm_tasks.iter().filter(|x| x.status == "done").count();
    let late = app
        .crm_tasks
        .iter()
        .filter(|x| x.status != "done" && x.due_at.is_some_and(|d| d <= now))
        .count()
        + followups.iter().filter(|x| x.remind_at <= now).count();

    if modal {
        ui.horizontal(|ui| {
            theme::icon(ui, Icon::Bell, 20.0, p.accent);
            theme::text(ui, "Central de tarefas", theme::bold(18.0), p.text);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if theme::icon_button(ui, Icon::X, 16.0, p.dim, p.text, "Fechar").clicked() {
                    app.actions.push(Action::CloseDialog);
                }
            });
        });
        theme::text(
            ui,
            "Tarefas e follow-ups ficam somente nesta conta do WhatsApp.",
            theme::regular(12.0),
            p.dim,
        );
    }
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        summary(ui, &p, "Atrasados", late, late > 0);
        summary(ui, &p, "Tarefas", open, false);
        summary(ui, &p, "Concluídas", done, false);
        summary(ui, &p, "Follow-ups", followups.len(), false);
    });
    let undo_id = egui::Id::new(("crm-task-undo", &account_id));
    if let Some(task) = ui.data_mut(|d| d.get_temp::<CrmTask>(undo_id)) {
        Frame::new()
            .fill(p.accent.gamma_multiply(0.12))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(Margin::symmetric(10, 7))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("‘{}’ foi excluída.", task.title));
                    if ui.button("Desfazer").clicked() {
                        app.actions.push(Action::SaveCrmTask(task));
                        ui.data_mut(|d| d.remove::<CrmTask>(undo_id));
                    }
                });
            });
    }
    ui.separator();

    if modal {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut view, TaskView::Today, "Meu dia");
            ui.selectable_value(&mut view, TaskView::Tasks, "Tarefas");
            ui.selectable_value(&mut view, TaskView::Calendar, "Agenda");
        });
    }
    ui.data_mut(|d| d.insert_temp(view_id, view));
    if view == TaskView::Today {
        show_day(app, ui, crate::util::today(), now, &mut draft);
        ui.data_mut(|d| d.insert_temp(draft_id, draft));
        return;
    }
    if view == TaskView::Calendar {
        show_calendar(app, ui, now, &mut draft);
        ui.data_mut(|d| d.insert_temp(draft_id, draft));
        return;
    }

    let cp = CrmPalette::from_app(&app.palette);
    show_task_board(app, ui, &cp);
}

pub fn show_task_board(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette) {
    let now = crate::util::now();
    let account_id = app.id.as_str().to_owned();

    let search_id = ui.id().with(("crm-task-search", &account_id));
    let project_filter_id = ui.id().with(("crm-task-proj-filter", &account_id));
    let priority_filter_id = ui.id().with(("crm-task-prio-filter", &account_id));
    let form_open_id = ui.id().with(("crm-task-form-open", &account_id));
    let draft_id = ui.id().with(("crm-task-draft", &account_id));
    let drag_id = egui::Id::new(("crm-task-drag-id", &account_id));

    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
    let mut project_filter =
        ui.data_mut(|d| d.get_temp::<Option<String>>(project_filter_id).flatten());
    let mut priority_filter = ui.data_mut(|d| {
        d.get_temp::<String>(priority_filter_id)
            .unwrap_or_else(|| "all".into())
    });
    let mut show_form = ui.data_mut(|d| d.get_temp::<bool>(form_open_id).unwrap_or(false));
    let mut draft = ui.data_mut(|d| d.get_temp::<TaskDraft>(draft_id).unwrap_or_default());
    let mut dragged_id = ui.ctx().data(|d| d.get_temp::<String>(drag_id));

    let view_mode_id = ui.id().with(("task-board-view-mode", &account_id));
    let mut view_mode = ui.data_mut(|d| {
        d.get_temp::<String>(view_mode_id)
            .unwrap_or_else(|| "kanban".into())
    });

    // 1. Top Filters & Action Row (responsive layout)
    let avail = ui.available_width();
    if avail < 760.0 {
        // Compact 2-row layout: Search & Switcher, then Filters & "+ Nova tarefa"
        ui.horizontal(|ui| {
            let search_w = (avail - 180.0).max(120.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, descrição...")
                    .desired_width(search_w),
            );

            let is_kanban = view_mode == "kanban";
            let kanban_btn = egui::Button::new(RichText::new("⊞ Kanban").size(12.0).strong().color(
                if is_kanban {
                    cp.on_accent
                } else {
                    cp.secondary
                },
            ))
            .fill(if is_kanban { cp.accent } else { cp.surface })
            .stroke(Stroke::new(
                1.0,
                if is_kanban { cp.accent } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(76.0, 30.0));
            if ui.add(kanban_btn).clicked() {
                view_mode = "kanban".into();
            }

            let is_list = view_mode == "list";
            let list_btn = egui::Button::new(
                RichText::new("☰ Lista")
                    .size(12.0)
                    .strong()
                    .color(if is_list { cp.on_accent } else { cp.secondary }),
            )
            .fill(if is_list { cp.accent } else { cp.surface })
            .stroke(Stroke::new(
                1.0,
                if is_list { cp.accent } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(68.0, 30.0));
            if ui.add(list_btn).clicked() {
                view_mode = "list".into();
            }
        });

        ui.add_space(6.0);

        ui.horizontal(|ui| {
            let selected_proj_title = project_filter
                .as_ref()
                .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                .map(|p| p.name.as_str())
                .unwrap_or("Todos os projetos");

            egui::ComboBox::from_id_salt(("task-board-proj-filter", &account_id))
                .selected_text(selected_proj_title)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut project_filter, None, "Todos os projetos");
                    for p in &app.crm_projects {
                        ui.selectable_value(&mut project_filter, Some(p.id.clone()), &p.name);
                    }
                });

            let prio_title = match priority_filter.as_str() {
                "urgent" => "Urgente",
                "high" => "Alta",
                "normal" => "Normal",
                "low" => "Baixa",
                _ => "Prioridade",
            };
            egui::ComboBox::from_id_salt(("task-board-prio-filter", &account_id))
                .selected_text(prio_title)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut priority_filter, "all".into(), "Todas as prioridades");
                    ui.selectable_value(&mut priority_filter, "urgent".into(), "Urgente");
                    ui.selectable_value(&mut priority_filter, "high".into(), "Alta");
                    ui.selectable_value(&mut priority_filter, "normal".into(), "Normal");
                    ui.selectable_value(&mut priority_filter, "low".into(), "Baixa");
                });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let new_task_btn = egui::Button::new(
                    RichText::new("+ Nova tarefa")
                        .size(13.5)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(130.0, 32.0));

                if ui.add(new_task_btn).clicked() {
                    draft = TaskDraft::default();
                    show_form = true;
                }
            });
        });
    } else {
        ui.horizontal(|ui| {
            let search_w = (avail - 490.0).clamp(160.0, 260.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, descrição, cliente...")
                    .desired_width(search_w),
            );

            let selected_proj_title = project_filter
                .as_ref()
                .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                .map(|p| p.name.as_str())
                .unwrap_or("Todos os projetos");

            egui::ComboBox::from_id_salt(("task-board-proj-filter", &account_id))
                .selected_text(selected_proj_title)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut project_filter, None, "Todos os projetos");
                    for p in &app.crm_projects {
                        ui.selectable_value(&mut project_filter, Some(p.id.clone()), &p.name);
                    }
                });

            let prio_title = match priority_filter.as_str() {
                "urgent" => "Urgente",
                "high" => "Alta",
                "normal" => "Normal",
                "low" => "Baixa",
                _ => "Todas as prioridades",
            };
            egui::ComboBox::from_id_salt(("task-board-prio-filter", &account_id))
                .selected_text(prio_title)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut priority_filter, "all".into(), "Todas as prioridades");
                    ui.selectable_value(&mut priority_filter, "urgent".into(), "Urgente");
                    ui.selectable_value(&mut priority_filter, "high".into(), "Alta");
                    ui.selectable_value(&mut priority_filter, "normal".into(), "Normal");
                    ui.selectable_value(&mut priority_filter, "low".into(), "Baixa");
                });

            ui.add_space(4.0);

            // View Mode Switcher
            let is_kanban = view_mode == "kanban";
            let kanban_btn = egui::Button::new(RichText::new("⊞ Kanban").size(12.0).strong().color(
                if is_kanban {
                    cp.on_accent
                } else {
                    cp.secondary
                },
            ))
            .fill(if is_kanban { cp.accent } else { cp.surface })
            .stroke(Stroke::new(
                1.0,
                if is_kanban { cp.accent } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(78.0, 32.0));
            if ui.add(kanban_btn).clicked() {
                view_mode = "kanban".into();
            }

            let is_list = view_mode == "list";
            let list_btn = egui::Button::new(
                RichText::new("☰ Lista")
                    .size(12.0)
                    .strong()
                    .color(if is_list { cp.on_accent } else { cp.secondary }),
            )
            .fill(if is_list { cp.accent } else { cp.surface })
            .stroke(Stroke::new(
                1.0,
                if is_list { cp.accent } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(72.0, 32.0));
            if ui.add(list_btn).clicked() {
                view_mode = "list".into();
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                let new_task_btn = egui::Button::new(
                    RichText::new("+ Nova tarefa")
                        .size(13.5)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(136.0, 34.0));

                if ui.add(new_task_btn).clicked() {
                    draft = TaskDraft::default();
                    show_form = true;
                }
            });
        });
    }

    ui.add_space(12.0);

    // 2. Floating Modal Window for Task Create/Edit
    if show_form {
        let mut is_open = true;
        let title = if draft.id.is_some() {
            "Editar Tarefa"
        } else {
            "Nova Tarefa"
        };
        egui::Window::new(title)
            .id(ui.id().with("crm-task-floating-modal"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(480.0, 520.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                render_task_form_modal(app, ui, cp, &mut draft, now, &mut show_form);
            });
        if !is_open {
            show_form = false;
            draft = TaskDraft::default();
        }
    }

    if view_mode == "list" {
        show_task_list(
            app,
            ui,
            cp,
            now,
            &search,
            project_filter.as_deref(),
            &priority_filter,
            &mut draft,
            &mut show_form,
        );
    } else {
        // 3. Kanban Columns occupying the rest of the available height
        let columns = [
            ("todo", "A fazer", Color32::from_rgb(0x3b, 0x82, 0xf6)),
            ("doing", "Em andamento", Color32::from_rgb(0xea, 0xb3, 0x08)),
            ("review", "Em revisão", Color32::from_rgb(0xa8, 0x55, 0xf7)),
            ("done", "Concluída", Color32::from_rgb(0x10, 0xb9, 0x81)),
        ];

        let avail_w = ui.available_width();
        let col_gap = 12.0;
        let total_min_w = 4.0 * 230.0 + 3.0 * col_gap + 18.0;
        let col_w = if avail_w > total_min_w {
            (avail_w - 3.0 * col_gap - 18.0) / 4.0
        } else {
            230.0
        };
        let col_h = (ui.available_height() - 20.0).max(460.0);

        let needle = search.trim().to_lowercase();
        let pointer_pos = ui.input(|i| i.pointer.interact_pos());
        let mouse_released = ui.input(|i| i.pointer.any_released());

        ScrollArea::horizontal()
            .id_salt("crm_tasks_kanban_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = col_gap;

                    for (status, label, col_color) in columns {
                        let tasks: Vec<CrmTask> = app
                            .crm_tasks
                            .iter()
                            .filter(|t| {
                                !matches!(t.kind.as_str(), "call" | "conversation")
                                    && t.status == status
                                    && (project_filter.is_none() || t.project_id == project_filter)
                                    && (priority_filter == "all" || t.priority == priority_filter)
                                    && (needle.is_empty()
                                        || t.title.to_lowercase().contains(&needle)
                                        || t.description.to_lowercase().contains(&needle)
                                        || t.assignee.to_lowercase().contains(&needle)
                                        || t.chat_id
                                            .as_deref()
                                            .unwrap_or("")
                                            .to_lowercase()
                                            .contains(&needle))
                            })
                            .cloned()
                            .collect();

                        let expected_rect =
                            egui::Rect::from_min_size(ui.cursor().min, vec2(col_w, col_h));
                        let is_drag_over = dragged_id.is_some()
                            && pointer_pos.is_some_and(|p| expected_rect.contains(p));

                        if is_drag_over && mouse_released {
                            if let Some(ref tid) = dragged_id {
                                if let Some(mut target) =
                                    app.crm_tasks.iter().find(|t| &t.id == tid).cloned()
                                {
                                    if target.status != status {
                                        target.status = status.to_string();
                                        target.completed_at =
                                            if status == "done" { Some(now) } else { None };
                                        target.updated_at = now;
                                        app.actions.push(Action::SaveCrmTask(target));
                                    }
                                }
                                ui.ctx().data_mut(|d| d.remove::<String>(drag_id));
                                dragged_id = None;
                            }
                        }

                        let border_stroke = if is_drag_over {
                            Stroke::new(1.5, cp.accent)
                        } else {
                            Stroke::new(1.0, cp.outline)
                        };

                        ui.allocate_ui_with_layout(
                            vec2(col_w, col_h),
                            Layout::top_down(Align::Min),
                            |ui| {
                                Frame::new()
                                    .fill(cp.panel)
                                    .stroke(border_stroke)
                                    .corner_radius(CornerRadius::same(10))
                                    .inner_margin(Margin::same(12))
                                    .show(ui, |ui| {
                                        ui.set_width(col_w - 24.0);
                                        ui.set_min_height(col_h - 24.0);

                                        // Column Header
                                        ui.horizontal(|ui| {
                                            let (dot_rect, _) = ui.allocate_exact_size(
                                                Vec2::splat(12.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().circle_filled(
                                                dot_rect.center(),
                                                4.5,
                                                col_color,
                                            );

                                            ui.label(
                                                RichText::new(label)
                                                    .size(14.0)
                                                    .strong()
                                                    .color(cp.text),
                                            );

                                            ui.add_space(4.0);
                                            Frame::new()
                                                .fill(cp.surface)
                                                .corner_radius(CornerRadius::same(10))
                                                .inner_margin(Margin::symmetric(7, 2))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(tasks.len().to_string())
                                                            .size(11.5)
                                                            .strong()
                                                            .color(cp.secondary),
                                                    );
                                                });
                                        });

                                        ui.add_space(8.0);
                                        let cur = ui.cursor();
                                        ui.painter().hline(
                                            cur.left()..=(cur.left() + ui.available_width()),
                                            cur.top(),
                                            Stroke::new(1.0, cp.outline),
                                        );
                                        ui.add_space(8.0);

                                        // Cards ScrollArea
                                        ScrollArea::vertical()
                                            .id_salt(format!("crm_col_scroll_{status}"))
                                            .auto_shrink([false, false])
                                            .show(ui, |ui| {
                                                ui.spacing_mut().item_spacing.y = 10.0;

                                                if tasks.is_empty() {
                                                    ui.add_space(28.0);
                                                    ui.vertical_centered(|ui| {
                                                        ui.label(
                                                            RichText::new("Nenhuma tarefa")
                                                                .size(12.5)
                                                                .color(cp.dim),
                                                        );
                                                    });
                                                }

                                                for task in &tasks {
                                                    render_kanban_card(
                                                        app,
                                                        ui,
                                                        cp,
                                                        task,
                                                        now,
                                                        &drag_id,
                                                        &dragged_id,
                                                        &mut draft,
                                                        &mut show_form,
                                                    );
                                                }
                                            });
                                    });
                            },
                        );
                    }

                    ui.add_space(18.0);
                });
            });

        // 4. Ghost Drag preview if dragging
        if let Some(ref tid) = dragged_id {
            if let Some(task) = app.crm_tasks.iter().find(|t| &t.id == tid) {
                if let Some(pos) = pointer_pos {
                    egui::Area::new(ui.id().with("crm_task_ghost_preview"))
                        .fixed_pos(pos + vec2(12.0, 12.0))
                        .order(egui::Order::Tooltip)
                        .interactable(false)
                        .show(ui.ctx(), |ui| {
                            Frame::new()
                                .fill(cp.surface)
                                .stroke(Stroke::new(1.5, cp.accent))
                                .corner_radius(CornerRadius::same(8))
                                .inner_margin(Margin::symmetric(14, 10))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&task.title)
                                            .size(13.5)
                                            .strong()
                                            .color(cp.text),
                                    );
                                    ui.label(
                                        RichText::new("Solte na coluna para mudar o status")
                                            .size(11.0)
                                            .color(cp.accent),
                                    );
                                });
                        });
                }
            }
        }

        if mouse_released && dragged_id.is_some() {
            ui.ctx().data_mut(|d| d.remove::<String>(drag_id));
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(search_id, search);
        d.insert_temp(project_filter_id, project_filter);
        d.insert_temp(priority_filter_id, priority_filter);
        d.insert_temp(view_mode_id, view_mode);
        d.insert_temp(form_open_id, show_form);
        d.insert_temp(draft_id, draft);
    });
}

fn show_task_list(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    now: i64,
    search: &str,
    project_filter: Option<&str>,
    priority_filter: &str,
    draft: &mut TaskDraft,
    show_form: &mut bool,
) {
    let needle = search.trim().to_lowercase();
    let tasks: Vec<CrmTask> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            !matches!(t.kind.as_str(), "call" | "conversation")
                && (project_filter.is_none() || t.project_id.as_deref() == project_filter)
                && (priority_filter == "all" || t.priority == priority_filter)
                && (needle.is_empty()
                    || t.title.to_lowercase().contains(&needle)
                    || t.description.to_lowercase().contains(&needle)
                    || t.assignee.to_lowercase().contains(&needle)
                    || t.chat_id
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&needle))
        })
        .cloned()
        .collect();

    if tasks.is_empty() {
        empty_state(
            ui,
            cp,
            Icon::ListChecks,
            "Nenhuma tarefa encontrada",
            "Crie uma nova tarefa ou ajuste os filtros para visualizar suas demandas operacionais.",
            None,
        );
        return;
    }

    ScrollArea::vertical()
        .id_salt("tasks_list_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let mut to_complete = None;
            for task in &tasks {
                let done = task.status == "done";
                let is_late = !done && task.due_at.is_some_and(|d| d <= now);
                let project_name = task
                    .project_id
                    .as_ref()
                    .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                    .map(|p| p.name.clone());
                let contact_name = task
                    .chat_id
                    .as_ref()
                    .and_then(|id| app.chat(id))
                    .map(|c| app.chat_title(c));

                Frame::new()
                    .fill(cp.surface)
                    .stroke(Stroke::new(
                        1.0,
                        if is_late { cp.danger } else { cp.outline },
                    ))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Checkbox toggle
                            let check_btn = egui::Button::new(
                                RichText::new(if done { "✓" } else { "□" })
                                    .size(15.0)
                                    .color(if done {
                                        cp.accent
                                    } else if is_late {
                                        cp.danger
                                    } else {
                                        cp.dim
                                    }),
                            )
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(5))
                            .min_size(vec2(28.0, 28.0));
                            if ui.add(check_btn).clicked() {
                                let mut updated = task.clone();
                                updated.status = if done { "todo" } else { "done" }.into();
                                updated.completed_at = if done { None } else { Some(now) };
                                updated.updated_at = now;
                                to_complete = Some(updated);
                            }

                            ui.add_space(4.0);

                            // Status badge
                            let (s_bg, s_fg, s_label) = match task.status.as_str() {
                                "todo" => (
                                    Color32::from_rgb(0x3b, 0x82, 0xf6).gamma_multiply(0.15),
                                    Color32::from_rgb(0x3b, 0x82, 0xf6),
                                    "A fazer",
                                ),
                                "doing" => (
                                    Color32::from_rgb(0xea, 0xb3, 0x08).gamma_multiply(0.15),
                                    Color32::from_rgb(0xea, 0xb3, 0x08),
                                    "Em andamento",
                                ),
                                "review" => (
                                    Color32::from_rgb(0xa8, 0x55, 0xf7).gamma_multiply(0.15),
                                    Color32::from_rgb(0xa8, 0x55, 0xf7),
                                    "Em revisão",
                                ),
                                "done" => (
                                    Color32::from_rgb(0x10, 0xb9, 0x81).gamma_multiply(0.15),
                                    Color32::from_rgb(0x10, 0xb9, 0x81),
                                    "Concluída",
                                ),
                                _ => (cp.surface_hover, cp.dim, task.status.as_str()),
                            };
                            Frame::new()
                                .fill(s_bg)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(s_label).size(11.0).strong().color(s_fg),
                                    );
                                });

                            ui.add_space(4.0);

                            // Priority badge
                            priority_badge(ui, &task.priority, cp);

                            ui.add_space(6.0);

                            // Title & Description (constrained width so actions never get pushed off-screen)
                            let row_avail = ui.available_width();
                            let title_max_w = (row_avail - 250.0).max(90.0);
                            ui.allocate_ui_with_layout(
                                vec2(title_max_w, 36.0),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&task.title)
                                                .size(13.5)
                                                .strong()
                                                .color(if done { cp.dim } else { cp.text }),
                                        )
                                        .truncate(),
                                    );
                                    if !task.description.is_empty() {
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&task.description)
                                                    .size(11.5)
                                                    .color(cp.secondary),
                                            )
                                            .truncate(),
                                        );
                                    }
                                },
                            );

                            // Right-aligned items: Project, Client, Due date, Assignee, Actions
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(4.0);

                                // Standard delete confirmation (Cancelar left, Excluir right)
                                let delete_id =
                                    egui::Id::new(("task-list-delete-confirm", &task.id));
                                let mut is_confirming =
                                    ui.data_mut(|d| d.get_temp::<bool>(delete_id).unwrap_or(false));
                                if is_confirming {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Cancelar")
                                                    .size(12.0)
                                                    .color(cp.secondary),
                                            )
                                            .fill(cp.surface_hover)
                                            .corner_radius(CornerRadius::same(5)),
                                        )
                                        .clicked()
                                    {
                                        is_confirming = false;
                                    }
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Excluir")
                                                    .size(12.0)
                                                    .strong()
                                                    .color(Color32::WHITE),
                                            )
                                            .fill(cp.danger)
                                            .corner_radius(CornerRadius::same(5)),
                                        )
                                        .clicked()
                                    {
                                        app.actions.push(Action::DeleteCrmTask(task.id.clone()));
                                        is_confirming = false;
                                    }
                                } else if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Excluir").size(12.0).color(cp.danger),
                                        )
                                        .fill(cp.surface_hover)
                                        .stroke(Stroke::new(1.0, cp.outline))
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
                                    is_confirming = true;
                                }
                                ui.data_mut(|d| d.insert_temp(delete_id, is_confirming));

                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new("Editar").size(12.0).color(cp.secondary),
                                        )
                                        .fill(cp.surface_hover)
                                        .corner_radius(CornerRadius::same(5)),
                                    )
                                    .clicked()
                                {
                                    *draft = TaskDraft {
                                        id: Some(task.id.clone()),
                                        title: task.title.clone(),
                                        description: task.description.clone(),
                                        priority: task.priority.clone(),
                                        status: task.status.clone(),
                                        assignee: task.assignee.clone(),
                                        kind: task.kind.clone(),
                                        duration_minutes: task.duration_minutes.unwrap_or(30),
                                        call_result: task.call_result.clone(),
                                        next_step: task.next_step.clone(),
                                        revenue_reais: format_revenue_input(task.revenue_cents),
                                        project_id: task.project_id.clone(),
                                        has_due: task.due_at.is_some(),
                                        due_date: task
                                            .due_at
                                            .and_then(crate::util::local_datetime)
                                            .map(|x| x.0)
                                            .unwrap_or_else(crate::util::today),
                                        due_hour: task
                                            .due_at
                                            .and_then(crate::util::local_datetime)
                                            .map(|x| x.1)
                                            .unwrap_or(9),
                                        due_minute: task
                                            .due_at
                                            .and_then(crate::util::local_datetime)
                                            .map(|x| x.2)
                                            .unwrap_or(0),
                                        chat_id: task.chat_id.clone(),
                                        link_chat: task.chat_id.is_some(),
                                    };
                                    *show_form = true;
                                }

                                if let Some(due) = task.due_at {
                                    if let Some((date, _, _)) = crate::util::local_datetime(due) {
                                        let text_color = if is_late { cp.danger } else { cp.dim };
                                        ui.label(
                                            RichText::new(format!(
                                                "📅 {:02}/{:02}",
                                                date.day(),
                                                date.month()
                                            ))
                                            .size(11.5)
                                            .color(text_color),
                                        );
                                    }
                                }

                                if !task.assignee.is_empty() {
                                    ui.label(
                                        RichText::new(format!("👤 {}", task.assignee))
                                            .size(11.5)
                                            .color(cp.dim),
                                    );
                                }

                                if let Some(c_name) = &contact_name {
                                    ui.label(
                                        RichText::new(format!("💬 {}", c_name))
                                            .size(11.5)
                                            .color(cp.secondary),
                                    );
                                }

                                if let Some(p_name) = &project_name {
                                    ui.label(
                                        RichText::new(format!("📁 {}", p_name))
                                            .size(11.5)
                                            .color(cp.accent),
                                    );
                                }
                            });
                        });
                    });
            }
            if let Some(t) = to_complete {
                app.actions.push(Action::SaveCrmTask(t));
            }
        });
}

fn priority_badge(ui: &mut egui::Ui, priority: &str, cp: &CrmPalette) {
    let (bg, fg, label) = match priority {
        "urgent" => (cp.danger.gamma_multiply(0.2), cp.danger, "Urgente"),
        "high" => (cp.warning.gamma_multiply(0.2), cp.warning, "Alta"),
        "low" => (cp.surface_hover, cp.dim, "Baixa"),
        _ => (
            Color32::from_rgb(0x1a, 0x2e, 0x3d),
            Color32::from_rgb(0x5c, 0xa7, 0xdb),
            "Normal",
        ),
    };
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(11.0).strong().color(fg));
        });
}

fn render_kanban_card(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    task: &CrmTask,
    now: i64,
    drag_id: &egui::Id,
    dragged_id: &Option<String>,
    draft: &mut TaskDraft,
    show_form: &mut bool,
) {
    let is_dragging_this = dragged_id.as_deref() == Some(&task.id);
    let done = task.status == "done";
    let is_late = !done && task.due_at.is_some_and(|d| d <= now);

    let border_color = if is_late { cp.danger } else { cp.outline };

    let card_bg = if is_dragging_this {
        cp.surface.gamma_multiply(0.4)
    } else {
        cp.surface
    };

    Frame::new()
        .fill(card_bg)
        .stroke(Stroke::new(1.0, border_color))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Top row: Priority badge + Drag handle + Quick column arrows
                ui.horizontal(|ui| {
                    priority_badge(ui, &task.priority, cp);

                    let (_handle_rect, handle_resp) =
                        ui.allocate_exact_size(vec2(18.0, 18.0), egui::Sense::drag());
                    theme::icon(ui, Icon::Ellipsis, 14.0, cp.dim);
                    if handle_resp.drag_started() {
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(*drag_id, task.id.clone()));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let next_status = match task.status.as_str() {
                            "todo" => Some("doing"),
                            "doing" => Some("review"),
                            "review" => Some("done"),
                            _ => None,
                        };
                        let prev_status = match task.status.as_str() {
                            "done" => Some("review"),
                            "review" => Some("doing"),
                            "doing" => Some("todo"),
                            _ => None,
                        };

                        if let Some(next) = next_status {
                            if ui
                                .small_button("→")
                                .on_hover_text("Avançar status")
                                .clicked()
                            {
                                let mut updated = task.clone();
                                updated.status = next.into();
                                updated.completed_at =
                                    if next == "done" { Some(now) } else { None };
                                updated.updated_at = now;
                                app.actions.push(Action::SaveCrmTask(updated));
                            }
                        }
                        if let Some(prev) = prev_status {
                            if ui
                                .small_button("←")
                                .on_hover_text("Voltar status")
                                .clicked()
                            {
                                let mut updated = task.clone();
                                updated.status = prev.into();
                                updated.completed_at = None;
                                updated.updated_at = now;
                                app.actions.push(Action::SaveCrmTask(updated));
                            }
                        }
                    });
                });

                ui.add_space(6.0);

                // Title with drag capability
                let title_text = RichText::new(&task.title)
                    .size(14.0)
                    .strong()
                    .color(if done { cp.dim } else { cp.text });
                let title_resp = ui.add(egui::Label::new(title_text).sense(egui::Sense::drag()));
                if title_resp.drag_started() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(*drag_id, task.id.clone()));
                }

                if !task.description.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&task.description)
                            .size(11.5)
                            .color(cp.secondary),
                    );
                }

                ui.add_space(6.0);

                // Badges: Projeto, Cliente, Responsável, Prazo
                if let Some(project) = task
                    .project_id
                    .as_ref()
                    .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::ListChecks, 13.0, cp.accent);
                        ui.label(RichText::new(&project.name).size(11.5).color(cp.secondary));
                    });
                }

                if let Some(chat_id) = &task.chat_id {
                    let client_name = app
                        .chat(chat_id)
                        .map(|c| app.chat_title(c))
                        .unwrap_or_else(|| chat_id.clone());
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Users, 13.0, cp.secondary);
                        ui.label(RichText::new(client_name).size(11.5).color(cp.secondary));
                    });
                }

                if !task.assignee.is_empty() {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::User, 13.0, cp.dim);
                        ui.label(RichText::new(&task.assignee).size(11.0).color(cp.dim));
                    });
                }

                if let Some(due) = task.due_at {
                    ui.horizontal(|ui| {
                        let (icon_color, text_color, prefix) = if is_late {
                            (cp.danger, cp.danger, "Atrasada: ")
                        } else {
                            (cp.dim, cp.dim, "Prazo: ")
                        };
                        theme::icon(ui, Icon::Clock, 13.0, icon_color);
                        ui.label(
                            RichText::new(format!("{prefix}{}", format_when(due)))
                                .size(11.5)
                                .color(text_color)
                                .strong(),
                        );
                    });
                }

                // Card Footer Actions
                ui.add_space(6.0);
                let cur = ui.cursor();
                ui.painter().hline(
                    cur.left()..=(cur.left() + ui.available_width()),
                    cur.top(),
                    Stroke::new(1.0, cp.outline),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui
                        .small_button(if done { "Reabrir" } else { "✓ Feito" })
                        .clicked()
                    {
                        let mut updated = task.clone();
                        updated.status = if done { "todo" } else { "done" }.into();
                        updated.completed_at = if done { None } else { Some(now) };
                        updated.updated_at = now;
                        app.actions.push(Action::SaveCrmTask(updated));
                    }

                    if ui.small_button("Editar").clicked() {
                        *draft = TaskDraft {
                            id: Some(task.id.clone()),
                            title: task.title.clone(),
                            description: task.description.clone(),
                            priority: task.priority.clone(),
                            status: task.status.clone(),
                            assignee: task.assignee.clone(),
                            kind: task.kind.clone(),
                            duration_minutes: task.duration_minutes.unwrap_or(30),
                            call_result: task.call_result.clone(),
                            next_step: task.next_step.clone(),
                            revenue_reais: format_revenue_input(task.revenue_cents),
                            project_id: task.project_id.clone(),
                            has_due: task.due_at.is_some(),
                            due_date: task
                                .due_at
                                .and_then(crate::util::local_datetime)
                                .map(|x| x.0)
                                .unwrap_or_else(crate::util::today),
                            due_hour: task
                                .due_at
                                .and_then(crate::util::local_datetime)
                                .map(|x| x.1)
                                .unwrap_or(9),
                            due_minute: task
                                .due_at
                                .and_then(crate::util::local_datetime)
                                .map(|x| x.2)
                                .unwrap_or(0),
                            chat_id: task.chat_id.clone(),
                            link_chat: task.chat_id.is_some(),
                        };
                        *show_form = true;
                    }

                    let delete_id = egui::Id::new(("task-delete-confirm", &task.id));
                    let is_confirming =
                        ui.data_mut(|d| d.get_temp::<bool>(delete_id).unwrap_or(false));
                    if is_confirming {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Cancelar").size(12.0).color(cp.secondary),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            ui.data_mut(|d| d.insert_temp(delete_id, false));
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Excluir")
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::WHITE),
                                )
                                .fill(cp.danger)
                                .corner_radius(CornerRadius::same(5)),
                            )
                            .clicked()
                        {
                            app.actions.push(Action::DeleteCrmTask(task.id.clone()));
                            ui.data_mut(|d| d.insert_temp(delete_id, false));
                        }
                    } else if ui
                        .add(
                            egui::Button::new(RichText::new("Excluir").size(12.0).color(cp.danger))
                                .fill(cp.surface_hover)
                                .stroke(Stroke::new(1.0, cp.outline))
                                .corner_radius(CornerRadius::same(5)),
                        )
                        .clicked()
                    {
                        ui.data_mut(|d| d.insert_temp(delete_id, true));
                    }

                    if let Some(chat) = &task.chat_id {
                        if ui.small_button("Conversa").clicked() {
                            app.actions.push(Action::OpenChat(chat.clone()));
                        }
                    }
                });
            });
        });
}

fn render_task_form_modal(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    draft: &mut TaskDraft,
    now: i64,
    show_form: &mut bool,
) {
    let err_id = ui.id().with("crm-task-modal-err");
    let show_err = ui.data_mut(|d| d.get_temp::<bool>(err_id).unwrap_or(false));

    ui.vertical(|ui| {
        ui.label(
            RichText::new("Título da tarefa *")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let title_stroke = if show_err && draft.title.trim().is_empty() {
            Stroke::new(1.0, cp.danger)
        } else {
            Stroke::new(1.0, cp.outline)
        };
        Frame::new()
            .fill(cp.surface)
            .stroke(title_stroke)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut draft.title)
                        .hint_text("Ex.: Enviar proposta comercial")
                        .desired_width(ui.available_width())
                        .frame(Frame::NONE),
                );
            });
        if show_err && draft.title.trim().is_empty() {
            ui.label(
                RichText::new("O título é obrigatório")
                    .size(11.0)
                    .color(cp.danger),
            );
        }

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Prioridade")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                egui::ComboBox::from_id_salt("modal-task-priority")
                    .selected_text(priority_label(&draft.priority))
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.priority, "low".into(), "Baixa");
                        ui.selectable_value(&mut draft.priority, "normal".into(), "Normal");
                        ui.selectable_value(&mut draft.priority, "high".into(), "Alta");
                        ui.selectable_value(&mut draft.priority, "urgent".into(), "Urgente");
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Status / Coluna")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                egui::ComboBox::from_id_salt("modal-task-status")
                    .selected_text(status_label(&draft.status))
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.status, "todo".into(), "A fazer");
                        ui.selectable_value(&mut draft.status, "doing".into(), "Em andamento");
                        ui.selectable_value(&mut draft.status, "review".into(), "Em revisão");
                        ui.selectable_value(&mut draft.status, "done".into(), "Concluída");
                    });
            });
        });

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Cliente / Contato")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                let current_client = draft
                    .chat_id
                    .as_ref()
                    .and_then(|id| app.chat(id))
                    .map(|c| app.chat_title(c))
                    .unwrap_or_else(|| {
                        if draft.chat_id.is_some() {
                            "Cliente selecionado".into()
                        } else {
                            "Nenhum cliente (Geral)".into()
                        }
                    });

                egui::ComboBox::from_id_salt("modal-task-client")
                    .selected_text(current_client)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.chat_id, None, "Nenhum cliente (Geral)");
                        for chat in &app.chats {
                            let title = app.chat_title(chat);
                            ui.selectable_value(&mut draft.chat_id, Some(chat.id.clone()), title);
                        }
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Projeto vinculado")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                let current_proj_name = draft
                    .project_id
                    .as_ref()
                    .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                    .map_or("Sem projeto", |p| p.name.as_str());

                egui::ComboBox::from_id_salt("modal-task-project")
                    .selected_text(current_proj_name)
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.project_id, None, "Sem projeto");
                        for p in &app.crm_projects {
                            ui.selectable_value(&mut draft.project_id, Some(p.id.clone()), &p.name);
                        }
                    });
            });
        });

        ui.add_space(10.0);

        ui.vertical(|ui| {
            ui.label(
                RichText::new("Responsável")
                    .size(12.0)
                    .strong()
                    .color(cp.secondary),
            );
            ui.add_space(2.0);
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.assignee)
                            .hint_text("Nome do responsável")
                            .desired_width(ui.available_width())
                            .frame(Frame::NONE),
                    );
                });
        });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Prazo de entrega")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let cal_open_id = ui.id().with("modal-task-cal-open");
        let mut cal_open = ui.data_mut(|d| d.get_temp::<bool>(cal_open_id).unwrap_or(false));
        ui.horizontal(|ui| {
            ui.checkbox(&mut draft.has_due, "Definir prazo");
            if draft.has_due {
                let date_str = format!(
                    "📅 {:02}/{:02}/{} às {:02}:{:02}",
                    draft.due_date.day(),
                    draft.due_date.month(),
                    draft.due_date.year(),
                    draft.due_hour,
                    draft.due_minute
                );
                if ui
                    .button(RichText::new(date_str).color(cp.accent))
                    .clicked()
                {
                    cal_open = !cal_open;
                }
            }
        });

        if draft.has_due && cal_open {
            ui.add_space(4.0);
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(8))
                .show(ui, |ui| {
                    super::dialogs::date_time_picker(
                        ui,
                        app.locale,
                        &app.palette,
                        "crm-modal-task-due-picker",
                        &mut draft.due_date,
                        &mut draft.due_hour,
                        &mut draft.due_minute,
                    );
                });
        }
        ui.data_mut(|d| d.insert_temp(cal_open_id, cal_open));

        ui.add_space(10.0);

        ui.label(
            RichText::new("Descrição e anotações")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut draft.description)
                        .hint_text("Descreva os detalhes da tarefa...")
                        .desired_width(ui.available_width())
                        .desired_rows(3)
                        .frame(Frame::NONE),
                );
            });

        ui.add_space(8.0);

        if app.open_chat.is_some() {
            ui.checkbox(
                &mut draft.link_chat,
                "Vincular à conversa aberta no ZapFast",
            );
        }

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Button::new(RichText::new("Cancelar").size(13.0).color(cp.secondary))
                        .fill(cp.surface)
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(100.0, 36.0)),
                )
                .clicked()
            {
                *show_form = false;
                *draft = TaskDraft::default();
                ui.data_mut(|d| d.insert_temp(err_id, false));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let save_label = if draft.id.is_some() {
                    "Salvar alterações"
                } else {
                    "Criar tarefa"
                };
                let save_btn = egui::Button::new(
                    RichText::new(save_label)
                        .size(13.0)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(140.0, 36.0));

                if ui.add(save_btn).clicked() {
                    if draft.title.trim().is_empty() {
                        ui.data_mut(|d| d.insert_temp(err_id, true));
                        return;
                    }

                    let old = draft
                        .id
                        .as_ref()
                        .and_then(|id| app.crm_tasks.iter().find(|x| &x.id == id))
                        .cloned();

                    app.actions.push(Action::SaveCrmTask(CrmTask {
                        id: draft.id.clone().unwrap_or_else(new_task_id),
                        chat_id: draft.chat_id.clone().or_else(|| {
                            if draft.link_chat {
                                app.open_chat.clone()
                            } else {
                                old.as_ref().and_then(|x| x.chat_id.clone())
                            }
                        }),
                        project_id: draft.project_id.clone(),
                        title: draft.title.trim().into(),
                        description: draft.description.trim().into(),
                        status: draft.status.clone(),
                        priority: draft.priority.clone(),
                        kind: draft.kind.clone(),
                        duration_minutes: matches!(draft.kind.as_str(), "call" | "conversation")
                            .then_some(draft.duration_minutes),
                        assignee: draft.assignee.trim().into(),
                        call_result: draft.call_result.trim().into(),
                        next_step: draft.next_step.trim().into(),
                        revenue_cents: parse_revenue_cents(&draft.revenue_reais),
                        due_at: draft.has_due.then(|| {
                            crate::util::to_unix_seconds(
                                draft.due_date,
                                draft.due_hour,
                                draft.due_minute,
                            )
                            .unwrap_or(now)
                        }),
                        completed_at: if draft.status == "done" {
                            old.as_ref().and_then(|x| x.completed_at).or(Some(now))
                        } else {
                            None
                        },
                        created_at: old.as_ref().map_or(now, |x| x.created_at),
                        updated_at: now,
                    }));

                    *draft = TaskDraft::default();
                    *show_form = false;
                    ui.data_mut(|d| d.insert_temp(err_id, false));
                }
            });
        });
    });
}

fn summary(ui: &mut egui::Ui, p: &crate::theme::Palette, title: &str, value: usize, alert: bool) {
    Frame::new()
        .fill(if alert {
            p.danger.gamma_multiply(0.12)
        } else {
            p.surface
        })
        .corner_radius(CornerRadius::same(7))
        .inner_margin(Margin::symmetric(10, 6))
        .show(ui, |ui| {
            theme::text(
                ui,
                &format!("{title}: {value}"),
                theme::semibold(12.0),
                if alert { p.danger } else { p.text },
            )
        });
}

fn show_day(app: &mut App, ui: &mut egui::Ui, date: Date, now: i64, draft: &mut TaskDraft) {
    let cp = CrmPalette::from_app(&app.palette);
    ui.add_space(4.0);
    ui.label(
        RichText::new(&crate::util::long_date(app.locale, date))
            .size(16.0)
            .strong()
            .color(cp.text),
    );
    let tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|task| {
            task.due_at
                .and_then(crate::util::local_datetime)
                .is_some_and(|(due, _, _)| due == date)
        })
        .cloned()
        .collect();
    let followups: Vec<_> = app
        .crm_followups
        .iter()
        .filter(|item| {
            !item.done
                && crate::util::local_datetime(item.remind_at)
                    .is_some_and(|(due, _, _)| due == date)
        })
        .cloned()
        .collect();
    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .max_height(470.0)
        .show(ui, |ui| {
            if tasks.is_empty() && followups.is_empty() {
                empty_state(
                    ui,
                    &cp,
                    Icon::CircleCheck,
                    "Nenhum compromisso neste dia",
                    "Não há tarefas nem follow-ups previstos para esta data.",
                    None,
                );
            }
            for task in tasks {
                task_row(app, ui, &task, now, draft);
                ui.add_space(8.0);
            }
            for item in followups {
                followup_row(app, ui, &item, now);
                ui.add_space(8.0);
            }
        });
    if draft.id.is_some() {
        let account_id = app.id.as_str().to_owned();
        ui.data_mut(|d| {
            d.insert_temp(
                ui.id().with(("crm-task-view", &account_id)),
                TaskView::Tasks,
            )
        });
    }
}

fn show_calendar(app: &mut App, ui: &mut egui::Ui, now: i64, draft: &mut TaskDraft) {
    let cp = CrmPalette::from_app(&app.palette);
    let account_id = app.id.as_str().to_owned();
    let month_id = ui.id().with(("crm-agenda-month", &account_id));
    let selected_id = ui.id().with(("crm-agenda-selected", &account_id));
    let mut month = ui
        .data_mut(|d| d.get_temp::<Date>(month_id))
        .unwrap_or_else(|| crate::util::today().first_of_month());
    let mut selected = ui
        .data_mut(|d| d.get_temp::<Date>(selected_id))
        .unwrap_or_else(crate::util::today);

    ui.horizontal(|ui| {
        let prev_btn = egui::Button::new(RichText::new("‹").size(16.0).strong().color(cp.text))
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(32.0, 30.0));
        if ui.add(prev_btn).on_hover_text("Mês anterior").clicked() {
            month = crate::util::month_step(month, -1);
            selected = month;
        }

        ui.add_space(6.0);
        ui.label(
            RichText::new(&crate::util::month_heading(app.locale, month))
                .size(16.0)
                .strong()
                .color(cp.text),
        );
        ui.add_space(6.0);

        let next_btn = egui::Button::new(RichText::new("›").size(16.0).strong().color(cp.text))
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(32.0, 30.0));
        if ui.add(next_btn).on_hover_text("Próximo mês").clicked() {
            month = crate::util::month_step(month, 1);
            selected = month;
        }

        ui.add_space(8.0);
        let today_btn = egui::Button::new(RichText::new("Hoje").size(12.5).color(cp.secondary))
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(54.0, 30.0));
        if ui.add(today_btn).clicked() {
            selected = crate::util::today();
            month = selected.first_of_month();
        }
    });
    ui.add_space(8.0);

    ui.data_mut(|d| {
        d.insert_temp(month_id, month);
        d.insert_temp(selected_id, selected);
    });

    let headings = crate::util::weekday_headings(app.locale);
    let cell_width = ((ui.available_width() - 24.0) / 7.0).clamp(56.0, 180.0);
    egui::Grid::new(("crm-agenda-grid", &account_id))
        .num_columns(7)
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for heading in headings {
                ui.add_sized(
                    [cell_width, 22.0],
                    egui::Label::new(
                        egui::RichText::new(heading)
                            .size(12.0)
                            .color(cp.dim)
                            .strong(),
                    ),
                );
            }
            ui.end_row();
            let first = month.first_of_month();
            let lead = usize::try_from(first.weekday().to_monday_zero_offset()).unwrap_or(0);
            let days = first.days_in_month() as usize;
            let cells = (lead + days).div_ceil(7) * 7;
            for index in 0..cells {
                if index < lead || index >= lead + days {
                    ui.add_sized([cell_width, 60.0], egui::Label::new(""));
                } else {
                    let day = (index - lead + 1) as i8;
                    if let Ok(date) = Date::new(month.year(), month.month(), day) {
                        let task_count = app
                            .crm_tasks
                            .iter()
                            .filter(|task| {
                                task.status != "done"
                                    && task
                                        .due_at
                                        .and_then(crate::util::local_datetime)
                                        .is_some_and(|(d, _, _)| d == date)
                            })
                            .count();
                        let followup_count = app
                            .crm_followups
                            .iter()
                            .filter(|item| {
                                !item.done
                                    && crate::util::local_datetime(item.remind_at)
                                        .is_some_and(|(d, _, _)| d == date)
                            })
                            .count();
                        let count = task_count + followup_count;
                        let is_sel = selected == date;
                        let is_today = date == crate::util::today();

                        let fill = if is_sel {
                            cp.accent.gamma_multiply(0.2)
                        } else if is_today {
                            cp.surface_hover
                        } else {
                            cp.surface
                        };
                        let stroke = if is_sel {
                            Stroke::new(1.5, cp.accent)
                        } else if is_today {
                            Stroke::new(1.0, cp.accent.gamma_multiply(0.5))
                        } else {
                            Stroke::new(1.0, cp.outline)
                        };

                        let content = if count == 0 {
                            RichText::new(day.to_string()).size(13.0).color(if is_sel {
                                cp.accent
                            } else {
                                cp.text
                            })
                        } else {
                            RichText::new(format!(
                                "{day}\n{count} item{}",
                                if count == 1 { "" } else { "s" }
                            ))
                            .size(12.0)
                            .color(if is_sel {
                                cp.accent
                            } else {
                                cp.secondary
                            })
                        };

                        let btn = egui::Button::new(content)
                            .fill(fill)
                            .stroke(stroke)
                            .corner_radius(CornerRadius::same(8));

                        if ui.add_sized([cell_width, 58.0], btn).clicked() {
                            selected = date;
                        }
                    }
                }
                if (index + 1) % 7 == 0 {
                    ui.end_row();
                }
            }
        });
    ui.data_mut(|d| d.insert_temp(selected_id, selected));
    ui.add_space(12.0);
    ui.separator();
    ui.add_space(8.0);
    show_day(app, ui, selected, now, draft);
}

fn task_row(app: &mut App, ui: &mut egui::Ui, task: &CrmTask, now: i64, draft: &mut TaskDraft) {
    let cp = CrmPalette::from_app(&app.palette);
    let done = task.status == "done";
    let late = !done && task.due_at.is_some_and(|d| d <= now);
    let contact = task
        .chat_id
        .as_ref()
        .and_then(|id| app.chat(id))
        .map(|c| app.chat_title(c));
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, if late { cp.danger } else { cp.outline }))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let left_w = (ui.available_width() - 170.0).max(120.0);
                ui.allocate_ui_with_layout(
                    vec2(left_w, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.label(
                            RichText::new(&task.title)
                                .size(14.0)
                                .strong()
                                .color(if done { cp.dim } else { cp.text }),
                        );
                        if !task.description.is_empty() {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(&task.description)
                                    .size(12.5)
                                    .color(cp.secondary),
                            );
                        }
                        if !task.call_result.is_empty() {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!("Resultado: {}", task.call_result))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }
                        if !task.next_step.is_empty() {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(format!("Próximo passo: {}", task.next_step))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }
                        ui.add_space(4.0);
                        let mut meta = if matches!(task.kind.as_str(), "call" | "conversation") {
                            format!("Call • {} min", task.duration_minutes.unwrap_or(30))
                        } else {
                            priority_label(&task.priority).to_owned()
                        };
                        if let Some(project) = task
                            .project_id
                            .as_ref()
                            .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                        {
                            meta.push_str(&format!(" • Projeto: {}", project.name));
                        }
                        meta.push_str(&format!(" • {}", status_label(&task.status)));
                        if !task.assignee.is_empty() {
                            meta.push_str(&format!(" • Responsável: {}", task.assignee));
                        }
                        if let Some(d) = task.due_at {
                            meta.push_str(&format!(" • {}", format_when(d)));
                        }
                        if let Some(c) = contact {
                            meta.push_str(&format!(" • {c}"));
                        }
                        ui.label(RichText::new(&meta).size(11.5).color(if late {
                            cp.danger
                        } else {
                            cp.dim
                        }));
                    },
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let confirm_id =
                        egui::Id::new(("crm-task-delete-confirm", app.id.as_str(), &task.id));
                    let confirming =
                        ui.data_mut(|d| d.get_temp::<bool>(confirm_id).unwrap_or(false));
                    if confirming {
                        let cancel_btn = egui::Button::new(
                            RichText::new("Cancelar").size(12.0).color(cp.secondary),
                        )
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(cancel_btn).clicked() {
                            ui.data_mut(|d| d.insert_temp(confirm_id, false));
                        }
                        let ok_btn = egui::Button::new(
                            RichText::new("Excluir")
                                .size(12.0)
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(cp.danger)
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(ok_btn).clicked() {
                            app.actions.push(Action::DeleteCrmTask(task.id.clone()));
                            let undo_id = egui::Id::new(("crm-task-undo", app.id.as_str()));
                            ui.data_mut(|d| {
                                d.insert_temp(undo_id, task.clone());
                                d.insert_temp(confirm_id, false);
                            });
                        }
                    } else {
                        let del_btn =
                            egui::Button::new(RichText::new("Excluir").size(12.0).color(cp.danger))
                                .fill(cp.surface_hover)
                                .stroke(Stroke::new(1.0, cp.outline))
                                .corner_radius(CornerRadius::same(6));
                        if ui.add(del_btn).clicked() {
                            ui.data_mut(|d| d.insert_temp(confirm_id, true));
                        }
                    }

                    let edit_btn =
                        egui::Button::new(RichText::new("Editar").size(12.0).color(cp.text))
                            .fill(cp.surface_hover)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(6));
                    if ui.add(edit_btn).clicked() {
                        draft.id = Some(task.id.clone());
                        draft.title = task.title.clone();
                        draft.description = task.description.clone();
                        draft.priority = task.priority.clone();
                        draft.status = task.status.clone();
                        draft.assignee = task.assignee.clone();
                        draft.kind = task.kind.clone();
                        draft.duration_minutes = task.duration_minutes.unwrap_or(30);
                        draft.call_result = task.call_result.clone();
                        draft.next_step = task.next_step.clone();
                        draft.revenue_reais = format_revenue_input(task.revenue_cents);
                        draft.project_id = task.project_id.clone();
                        draft.has_due = task.due_at.is_some();
                        if let Some((date, hour, minute)) =
                            task.due_at.and_then(crate::util::local_datetime)
                        {
                            draft.due_date = date;
                            draft.due_hour = hour;
                            draft.due_minute = minute;
                        }
                        draft.link_chat = task.chat_id.is_some();
                    }

                    let done_label = if done { "Reabrir" } else { "Concluir" };
                    let done_color = if done { cp.secondary } else { cp.accent };
                    let done_btn = egui::Button::new(
                        RichText::new(done_label)
                            .size(12.0)
                            .strong()
                            .color(done_color),
                    )
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(done_btn).clicked() {
                        let mut x = task.clone();
                        x.status = if done { "todo" } else { "done" }.into();
                        x.completed_at = if done { None } else { Some(now) };
                        x.updated_at = now;
                        app.actions.push(Action::SaveCrmTask(x));
                    }

                    if let Some(chat) = &task.chat_id {
                        let chat_btn = egui::Button::new(
                            RichText::new("Conversa").size(12.0).color(cp.accent),
                        )
                        .fill(cp.surface_hover)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(chat_btn).clicked() {
                            app.actions.push(Action::CloseDialog);
                            app.actions.push(Action::OpenChat(chat.clone()));
                        }
                    }
                });
            });
        });
}

fn followup_row(app: &mut App, ui: &mut egui::Ui, item: &CrmFollowup, now: i64) {
    let cp = CrmPalette::from_app(&app.palette);
    let late = item.remind_at <= now;
    let contact = app
        .chat(&item.chat_id)
        .map(|c| app.chat_title(c))
        .unwrap_or_else(|| item.chat_id.trim_end_matches("@s.whatsapp.net").into());
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, if late { cp.danger } else { cp.outline }))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(14, 11))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&item.title)
                            .size(14.0)
                            .strong()
                            .color(cp.text),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(format!("{contact} • {}", format_when(item.remind_at)))
                            .size(12.0)
                            .color(if late { cp.danger } else { cp.dim }),
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let done_btn = egui::Button::new(
                        RichText::new("Concluir")
                            .size(12.0)
                            .strong()
                            .color(cp.accent),
                    )
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(done_btn).clicked() {
                        app.actions
                            .push(Action::CompleteCrmFollowup(item.id.clone()));
                    }

                    let snooze_btn =
                        egui::Button::new(RichText::new("+1 dia").size(12.0).color(cp.secondary))
                            .fill(cp.surface_hover)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(6));
                    if ui.add(snooze_btn).clicked() {
                        app.actions.push(Action::SnoozeCrmFollowup {
                            id: item.id.clone(),
                            until: now + 86_400,
                        });
                    }

                    let chat_btn = egui::Button::new(
                        RichText::new("Abrir conversa").size(12.0).color(cp.text),
                    )
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6));
                    if ui.add(chat_btn).clicked() {
                        app.actions.push(Action::CloseDialog);
                        app.actions.push(Action::OpenChat(item.chat_id.clone()));
                    }
                });
            })
        });
}

fn format_when(ts: i64) -> String {
    crate::util::local_datetime(ts)
        .map(|(d, h, m)| {
            format!(
                "{:02}/{:02}/{} {:02}:{:02}",
                d.day(),
                d.month(),
                d.year(),
                h,
                m
            )
        })
        .unwrap_or_else(|| "data indisponível".into())
}
fn priority_label(v: &str) -> &str {
    match v {
        "low" => "Baixa",
        "high" => "Alta",
        "urgent" => "Urgente",
        _ => "Normal",
    }
}

fn status_label(value: &str) -> &str {
    match value {
        "doing" => "Em andamento",
        "review" => "Em revisão",
        "done" => "Concluída",
        "scheduled" => "Agendada",
        "held" => "Realizada",
        "no_show" => "No-show",
        "cancelled" => "Cancelada",
        "closed" => "Fechamento",
        _ => "A fazer",
    }
}

fn parse_revenue_cents(value: &str) -> i64 {
    let normalized = value.trim().replace('.', "").replace(',', ".");
    normalized
        .parse::<f64>()
        .map(|value| (value.max(0.0) * 100.0).round() as i64)
        .unwrap_or(0)
}

fn format_revenue_input(cents: i64) -> String {
    if cents <= 0 {
        String::new()
    } else {
        format!("{:.2}", cents as f64 / 100.0).replace('.', ",")
    }
}
fn new_task_id() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task-{n}")
}

fn export_calls_csv(app: &mut App, calls: &[CrmTask]) {
    if let Some(path) = rfd::FileDialog::new()
        .set_title("Salvar Relatório de Calls de Venda")
        .set_file_name("calls-vendas.csv")
        .add_filter("CSV (Planilha)", &["csv"])
        .save_file()
    {
        let mut csv = String::from(
            "Data;Hora;Título;Cliente;Projeto;Responsável;Status;Duração(min);Resultado;Receita(R$);Próximo Passo;Anotações\n",
        );
        for call in calls {
            let (date_str, time_str) = call
                .due_at
                .and_then(crate::util::local_datetime)
                .map(|(d, h, m)| {
                    (
                        format!("{:02}/{:02}/{}", d.day(), d.month(), d.year()),
                        format!("{:02}:{:02}", h, m),
                    )
                })
                .unwrap_or_else(|| ("-".into(), "-".into()));
            let client = call
                .chat_id
                .as_ref()
                .and_then(|id| app.chat(id))
                .map(|c| app.chat_title(c))
                .unwrap_or_else(|| "-".into());
            let project = call
                .project_id
                .as_ref()
                .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                .map(|p| p.name.clone())
                .unwrap_or_else(|| "-".into());
            let status = match call.status.as_str() {
                "scheduled" => "Agendada",
                "held" => "Realizada",
                "no_show" => "No-show",
                "cancelled" => "Cancelada",
                "closed" => "Fechamento",
                _ => &call.status,
            };
            let revenue = format!("{:.2}", call.revenue_cents as f64 / 100.0).replace('.', ",");
            let clean = |s: &str| s.replace(';', ",").replace('\n', " ").replace('\r', "");
            csv.push_str(&format!(
                "{};{};{};{};{};{};{};{};{};{};{};{}\n",
                date_str,
                time_str,
                clean(&call.title),
                clean(&client),
                clean(&project),
                clean(&call.assignee),
                status,
                call.duration_minutes.unwrap_or(30),
                clean(&call.call_result),
                revenue,
                clean(&call.next_step),
                clean(&call.description),
            ));
        }
        match std::fs::write(&path, csv) {
            Ok(()) => app.toast("Relatório CSV exportado com sucesso!"),
            Err(e) => app.toast_error(format!("Erro ao exportar CSV: {e}")),
        }
    }
}
