//! Account-scoped CRM task and follow-up inbox.

use crate::app::App;
use crate::model::{Action, CrmDeal, CrmFollowup, CrmTask};
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AgendaViewMode {
    Day,
    Week,
    Month,
}

fn day_step(date: Date, delta: i32) -> Date {
    if delta == 0 {
        date
    } else if delta > 0 {
        let mut d = date;
        for _ in 0..delta {
            if let Ok(next) = d.tomorrow() {
                d = next;
            }
        }
        d
    } else {
        let mut d = date;
        for _ in 0..(-delta) {
            if let Ok(prev) = d.yesterday() {
                d = prev;
            }
        }
        d
    }
}

fn pt_month_name(month: i8) -> &'static str {
    match month {
        1 => "Janeiro",
        2 => "Fevereiro",
        3 => "Março",
        4 => "Abril",
        5 => "Maio",
        6 => "Junho",
        7 => "Julho",
        8 => "Agosto",
        9 => "Setembro",
        10 => "Outubro",
        11 => "Novembro",
        12 => "Dezembro",
        _ => "",
    }
}

fn pt_weekday_short(day_offset: usize) -> &'static str {
    match day_offset {
        0 => "Seg",
        1 => "Ter",
        2 => "Qua",
        3 => "Qui",
        4 => "Sex",
        5 => "Sáb",
        6 => "Dom",
        _ => "",
    }
}

fn pt_weekday_full(day_offset: usize) -> &'static str {
    match day_offset {
        0 => "Segunda-feira",
        1 => "Terça-feira",
        2 => "Quarta-feira",
        3 => "Quinta-feira",
        4 => "Sexta-feira",
        5 => "Sábado",
        6 => "Domingo",
        _ => "",
    }
}

fn format_range_title(view_mode: AgendaViewMode, anchor_date: Date) -> String {
    match view_mode {
        AgendaViewMode::Day => {
            let w_idx = anchor_date.weekday().to_monday_zero_offset() as usize;
            format!(
                "{}, {} de {} de {}",
                pt_weekday_full(w_idx),
                anchor_date.day(),
                pt_month_name(anchor_date.month()),
                anchor_date.year()
            )
        }
        AgendaViewMode::Week => {
            let mon_offset = anchor_date.weekday().to_monday_zero_offset() as i32;
            let mon = day_step(anchor_date, -mon_offset);
            let sun = day_step(mon, 6);
            if mon.month() == sun.month() {
                format!(
                    "{} a {} de {} de {}",
                    mon.day(),
                    sun.day(),
                    pt_month_name(mon.month()),
                    mon.year()
                )
            } else {
                format!(
                    "{} de {} a {} de {} de {}",
                    mon.day(),
                    pt_month_name(mon.month()),
                    sun.day(),
                    pt_month_name(sun.month()),
                    sun.year()
                )
            }
        }
        AgendaViewMode::Month => {
            format!("{} de {}", pt_month_name(anchor_date.month()), anchor_date.year())
        }
    }
}

#[derive(Clone)]
pub struct AgendaEventItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub date: Date,
    pub hour: u8,
    pub minute: u8,
    pub duration_minutes: u16,
    pub kind: String,
    pub status: String,
    pub color_bg: Color32,
    pub color_fg: Color32,
    pub raw_task: Option<CrmTask>,
    pub raw_followup: Option<CrmFollowup>,
}

fn get_day_events(app: &App, date: Date) -> Vec<AgendaEventItem> {
    let mut items = Vec::new();

    for task in &app.crm_tasks {
        if let Some(due) = task.due_at {
            if let Some((d, h, m)) = crate::util::local_datetime(due) {
                if d == date {
                    let is_call = matches!(task.kind.as_str(), "call" | "conversation");
                    let client_name = task
                        .chat_id
                        .as_ref()
                        .and_then(|id| app.chat(id))
                        .map(|c| app.chat_title(c))
                        .unwrap_or_default();
                    items.push(AgendaEventItem {
                        id: task.id.clone(),
                        title: task.title.clone(),
                        subtitle: if !client_name.is_empty() {
                            client_name
                        } else {
                            task.assignee.clone()
                        },
                        date,
                        hour: h,
                        minute: m,
                        duration_minutes: task.duration_minutes.unwrap_or(30),
                        kind: if is_call { "call".into() } else { "task".into() },
                        status: task.status.clone(),
                        color_bg: if is_call {
                            Color32::from_rgb(0x02, 0x84, 0xc7)
                        } else {
                            Color32::from_rgb(0x04, 0x78, 0x57)
                        },
                        color_fg: if is_call {
                            Color32::from_rgb(0x7d, 0xd3, 0xfc)
                        } else {
                            Color32::from_rgb(0x6e, 0xe7, 0xb7)
                        },
                        raw_task: Some(task.clone()),
                        raw_followup: None,
                    });
                }
            }
        }
    }

    for item in &app.crm_followups {
        if !item.done {
            if let Some((d, h, m)) = crate::util::local_datetime(item.remind_at) {
                if d == date {
                    let client_name = app
                        .chat(&item.chat_id)
                        .map(|c| app.chat_title(c))
                        .unwrap_or_else(|| item.chat_id.clone());
                    items.push(AgendaEventItem {
                        id: item.id.clone(),
                        title: item.title.clone(),
                        subtitle: client_name,
                        date,
                        hour: h,
                        minute: m,
                        duration_minutes: 15,
                        kind: "followup".into(),
                        status: "pending".into(),
                        color_bg: Color32::from_rgb(0xb4, 0x53, 0x09),
                        color_fg: Color32::from_rgb(0xfc, 0xd3, 0x4d),
                        raw_task: None,
                        raw_followup: Some(item.clone()),
                    });
                }
            }
        }
    }

    items.sort_by_key(|e| (e.hour, e.minute));
    items
}

fn render_agenda_day_view(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    date: Date,
    _now: i64,
    draft: &mut TaskDraft,
    show_modal: &mut bool,
) {
    let events = get_day_events(app, date);
    let time_col_w = 54.0;
    let slot_w = (ui.available_width() - time_col_w - 20.0).max(240.0);
    let slot_h = 60.0;

    for hour in 7..=22 {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(time_col_w, slot_h), egui::Sense::hover());
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:02}:00", hour),
                theme::medium(12.5),
                cp.dim,
            );

            let hour_events: Vec<&AgendaEventItem> =
                events.iter().filter(|e| e.hour == hour).collect();
            let slot_rect = egui::Rect::from_min_size(ui.cursor().min, vec2(slot_w, slot_h));
            let slot_id = ui.id().with(("day_slot", date.to_string(), hour));
            let slot_resp = ui.interact(slot_rect, slot_id, egui::Sense::click());

            let is_hovered = slot_resp.hovered();

            Frame::new()
                .fill(if is_hovered && hour_events.is_empty() {
                    cp.surface_hover
                } else {
                    cp.surface
                })
                .stroke(Stroke::new(0.5, cp.outline.gamma_multiply(0.6)))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(10, 6))
                .show(ui, |ui| {
                    ui.set_width(slot_w - 20.0);
                    ui.set_min_height(slot_h - 12.0);

                    if hour_events.is_empty() {
                        if is_hovered {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("+ Agendar às {:02}:00", hour))
                                        .size(12.0)
                                        .color(cp.accent),
                                );
                            });
                        }
                    } else {
                        for ev in &hour_events {
                            ui.horizontal(|ui| {
                                let (bar_rect, _) =
                                    ui.allocate_exact_size(vec2(3.5, 28.0), egui::Sense::hover());
                                ui.painter().rect_filled(
                                    bar_rect,
                                    CornerRadius::same(2),
                                    ev.color_bg,
                                );

                                ui.add_space(4.0);
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(&ev.title)
                                                .size(13.5)
                                                .strong()
                                                .color(cp.text),
                                        );
                                        ui.add_space(6.0);
                                        Frame::new()
                                            .fill(ev.color_bg.gamma_multiply(0.25))
                                            .corner_radius(CornerRadius::same(4))
                                            .inner_margin(Margin::symmetric(6, 2))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:02}:{:02} ({} min)",
                                                        ev.hour, ev.minute, ev.duration_minutes
                                                    ))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(ev.color_fg),
                                                );
                                            });
                                    });
                                    if !ev.subtitle.is_empty() {
                                        ui.label(
                                            RichText::new(&ev.subtitle)
                                                .size(11.5)
                                                .color(cp.secondary),
                                        );
                                    }
                                });

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if let Some(ref t) = ev.raw_task {
                                        if ui.small_button("Editar").clicked() {
                                            *draft = TaskDraft {
                                                id: Some(t.id.clone()),
                                                title: t.title.clone(),
                                                description: t.description.clone(),
                                                priority: t.priority.clone(),
                                                status: if matches!(
                                                    t.status.as_str(),
                                                    "todo" | "doing" | "review"
                                                ) && matches!(
                                                    t.kind.as_str(),
                                                    "call" | "conversation"
                                                ) {
                                                    "scheduled".into()
                                                } else {
                                                    t.status.clone()
                                                },
                                                assignee: t.assignee.clone(),
                                                kind: t.kind.clone(),
                                                duration_minutes: t.duration_minutes.unwrap_or(30),
                                                call_result: t.call_result.clone(),
                                                next_step: t.next_step.clone(),
                                                revenue_reais: format_revenue_input(
                                                    t.revenue_cents,
                                                ),
                                                project_id: t.project_id.clone(),
                                                has_due: t.due_at.is_some(),
                                                due_date: date,
                                                due_hour: ev.hour,
                                                due_minute: ev.minute,
                                                chat_id: t.chat_id.clone(),
                                                link_chat: t.chat_id.is_some(),
                                            };
                                            *show_modal = true;
                                        }

                                        if ui
                                            .small_button("+1h")
                                            .on_hover_text("Adiar 1 hora")
                                            .clicked()
                                        {
                                            let mut updated = t.clone();
                                            if let Some(due) = updated.due_at {
                                                updated.due_at = Some(due + 3600);
                                                updated.updated_at = crate::util::now();
                                                app.actions.push(Action::SaveCrmTask(updated));
                                                ui.ctx().request_repaint();
                                            }
                                        }

                                        if ui
                                            .small_button("-1h")
                                            .on_hover_text("Adiantar 1 hora")
                                            .clicked()
                                        {
                                            let mut updated = t.clone();
                                            if let Some(due) = updated.due_at {
                                                updated.due_at = Some(due - 3600);
                                                updated.updated_at = crate::util::now();
                                                app.actions.push(Action::SaveCrmTask(updated));
                                                ui.ctx().request_repaint();
                                            }
                                        }
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    }
                });

            if slot_resp.clicked() && hour_events.is_empty() {
                *draft = TaskDraft {
                    due_date: date,
                    due_hour: hour,
                    due_minute: 0,
                    has_due: true,
                    kind: "call".into(),
                    status: "scheduled".into(),
                    duration_minutes: 30,
                    ..TaskDraft::default()
                };
                *show_modal = true;
            }
        });
        ui.add_space(3.0);
    }
}

fn render_agenda_week_view(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    anchor_date: Date,
    today: Date,
    _now: i64,
    draft: &mut TaskDraft,
    show_modal: &mut bool,
) {
    let avail_w = ui.available_width();
    let time_col_w = 48.0;
    let days_w = (avail_w - time_col_w - 14.0).max(560.0);
    let col_w = (days_w / 7.0).max(75.0);
    let row_h = 54.0;
    let total_week_w = time_col_w + col_w * 7.0 + 16.0;

    let mon_offset = anchor_date.weekday().to_monday_zero_offset() as i32;
    let monday = day_step(anchor_date, -mon_offset);
    let week_days: Vec<Date> = (0..7).map(|i| day_step(monday, i)).collect();

    let week_events: Vec<Vec<AgendaEventItem>> =
        week_days.iter().map(|d| get_day_events(app, *d)).collect();

    ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(total_week_w);

            ui.horizontal(|ui| {
        ui.allocate_exact_size(vec2(time_col_w, 36.0), egui::Sense::hover());
        for (i, d) in week_days.iter().enumerate() {
            let is_today = *d == today;
            let header_bg = if is_today {
                cp.accent.gamma_multiply(0.18)
            } else {
                cp.surface
            };
            let border_stroke = if is_today {
                Stroke::new(1.5, cp.accent)
            } else {
                Stroke::new(1.0, cp.outline)
            };
            Frame::new()
                .fill(header_bg)
                .stroke(border_stroke)
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(4, 4))
                .show(ui, |ui| {
                    ui.set_width(col_w - 8.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(format!("{} {:02}", pt_weekday_short(i), d.day()))
                                .size(12.5)
                                .strong()
                                .color(if is_today { cp.accent } else { cp.text }),
                        );
                    });
                });
        }
    });

    ui.add_space(4.0);

    for hour in 8..=20 {
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(time_col_w, row_h), egui::Sense::hover());
            ui.painter().text(
                r.center(),
                egui::Align2::CENTER_CENTER,
                format!("{:02}:00", hour),
                theme::medium(11.5),
                cp.dim,
            );

            for (day_idx, date) in week_days.iter().enumerate() {
                let cell_rect = egui::Rect::from_min_size(ui.cursor().min, vec2(col_w, row_h));
                let cell_events: Vec<&AgendaEventItem> = week_events[day_idx]
                    .iter()
                    .filter(|e| e.hour == hour)
                    .collect();

                let cell_id = ui.id().with(("week_cell", date.to_string(), hour));
                let cell_resp = ui.interact(cell_rect, cell_id, egui::Sense::click());

                let is_hovered = cell_resp.hovered();
                let bg_color = if is_hovered && cell_events.is_empty() {
                    cp.surface_hover
                } else {
                    cp.surface
                };

                Frame::new()
                    .fill(bg_color)
                    .stroke(Stroke::new(0.5, cp.outline.gamma_multiply(0.5)))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(3, 3))
                    .show(ui, |ui| {
                        ui.set_width(col_w - 6.0);
                        ui.set_height(row_h - 6.0);

                        if cell_events.is_empty() {
                            if is_hovered {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(12.0);
                                    ui.label(RichText::new("+ Agendar").size(10.5).color(cp.accent));
                                });
                            }
                        } else {
                            for ev in cell_events.iter().take(2) {
                                let ev_chip = egui::Button::new(
                                    RichText::new(format!(
                                        "{} {:02}:{:02} {}",
                                        if ev.kind == "call" {
                                            "📞"
                                        } else if ev.kind == "task" {
                                            "✓"
                                        } else {
                                            "🔔"
                                        },
                                        ev.hour,
                                        ev.minute,
                                        ev.title
                                    ))
                                    .size(10.5)
                                    .strong()
                                    .color(ev.color_fg),
                                )
                                .fill(ev.color_bg.gamma_multiply(0.35))
                                .stroke(Stroke::new(1.0, ev.color_bg))
                                .corner_radius(CornerRadius::same(4));

                                if ui
                                    .add_sized([col_w - 8.0, 20.0], ev_chip)
                                    .on_hover_text(format!(
                                        "{}\n{}: {}",
                                        ev.title, ev.kind, ev.subtitle
                                    ))
                                    .clicked()
                                {
                                    if let Some(ref t) = ev.raw_task {
                                        *draft = TaskDraft {
                                            id: Some(t.id.clone()),
                                            title: t.title.clone(),
                                            description: t.description.clone(),
                                            priority: t.priority.clone(),
                                            status: if matches!(
                                                t.status.as_str(),
                                                "todo" | "doing" | "review"
                                            ) && matches!(
                                                t.kind.as_str(),
                                                "call" | "conversation"
                                            ) {
                                                "scheduled".into()
                                            } else {
                                                t.status.clone()
                                            },
                                            assignee: t.assignee.clone(),
                                            kind: t.kind.clone(),
                                            duration_minutes: t.duration_minutes.unwrap_or(30),
                                            call_result: t.call_result.clone(),
                                            next_step: t.next_step.clone(),
                                            revenue_reais: format_revenue_input(t.revenue_cents),
                                            project_id: t.project_id.clone(),
                                            has_due: t.due_at.is_some(),
                                            due_date: *date,
                                            due_hour: ev.hour,
                                            due_minute: ev.minute,
                                            chat_id: t.chat_id.clone(),
                                            link_chat: t.chat_id.is_some(),
                                        };
                                        *show_modal = true;
                                    }
                                }
                            }
                            if cell_events.len() > 2 {
                                ui.label(
                                    RichText::new(format!("+{} mais", cell_events.len() - 2))
                                        .size(9.5)
                                        .color(cp.dim),
                                );
                            }
                        }
                    });

                if cell_resp.clicked() && cell_events.is_empty() {
                    *draft = TaskDraft {
                        due_date: *date,
                        due_hour: hour,
                        due_minute: 0,
                        has_due: true,
                        kind: "call".into(),
                        status: "scheduled".into(),
                        duration_minutes: 30,
                        ..TaskDraft::default()
                    };
                    *show_modal = true;
                }
            }
        });
    }
        });
}

fn render_agenda_month_view(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    anchor_date: Date,
    today: Date,
    _now: i64,
    draft: &mut TaskDraft,
    show_modal: &mut bool,
) {
    let avail_w = ui.available_width();
    let col_w = ((avail_w - 24.0) / 7.0).max(65.0);
    let cell_h = 104.0;

    let first = anchor_date.first_of_month();
    let lead = usize::try_from(first.weekday().to_monday_zero_offset()).unwrap_or(0);
    let days_in_month = first.days_in_month() as usize;
    let cells_count = (lead + days_in_month).div_ceil(7) * 7;

    ui.horizontal(|ui| {
        for i in 0..7 {
            ui.add_sized(
                [col_w, 24.0],
                egui::Label::new(
                    RichText::new(pt_weekday_short(i))
                        .size(12.0)
                        .strong()
                        .color(cp.dim),
                ),
            );
        }
    });

    ui.add_space(4.0);

    for row_idx in 0..(cells_count / 7) {
        ui.horizontal(|ui| {
            for col_idx in 0..7 {
                let cell_idx = row_idx * 7 + col_idx;
                if cell_idx < lead || cell_idx >= lead + days_in_month {
                    Frame::new()
                        .fill(cp.panel.gamma_multiply(0.4))
                        .stroke(Stroke::new(0.5, cp.outline.gamma_multiply(0.3)))
                        .corner_radius(CornerRadius::same(4))
                        .show(ui, |ui| {
                            ui.set_width(col_w - 4.0);
                            ui.set_height(cell_h - 4.0);
                        });
                } else {
                    let day = (cell_idx - lead + 1) as i8;
                    if let Ok(date) = Date::new(anchor_date.year(), anchor_date.month(), day) {
                        let is_today = date == today;
                        let day_events = get_day_events(app, date);
                        let mut event_clicked = false;

                        let cell_id = ui.id().with(("month_cell", date.to_string()));
                        let cell_rect =
                            egui::Rect::from_min_size(ui.cursor().min, vec2(col_w, cell_h));
                        let cell_resp = ui.interact(cell_rect, cell_id, egui::Sense::click());

                        let bg_color = if cell_resp.hovered() {
                            cp.surface_hover
                        } else if is_today {
                            cp.surface_hover.gamma_multiply(0.9)
                        } else {
                            cp.surface
                        };

                        let border_stroke = if is_today {
                            Stroke::new(1.5, cp.accent)
                        } else {
                            Stroke::new(0.5, cp.outline)
                        };

                        Frame::new()
                            .fill(bg_color)
                            .stroke(border_stroke)
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(5, 5))
                            .show(ui, |ui| {
                                ui.set_width(col_w - 4.0);
                                ui.set_height(cell_h - 4.0);

                                ui.horizontal(|ui| {
                                    if is_today {
                                        Frame::new()
                                            .fill(cp.accent)
                                            .corner_radius(CornerRadius::same(9))
                                            .inner_margin(Margin::symmetric(6, 1))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(day.to_string())
                                                        .size(11.5)
                                                        .strong()
                                                        .color(cp.on_accent),
                                                );
                                            });
                                    } else {
                                        ui.label(
                                            RichText::new(day.to_string())
                                                        .size(12.0)
                                                        .strong()
                                                        .color(cp.text),
                                        );
                                    }
                                });

                                ui.add_space(2.0);

                                for ev in day_events.iter().take(3) {
                                    let chip_btn = egui::Button::new(
                                        RichText::new(format!(
                                            "{} {:02}:{:02} {}",
                                            if ev.kind == "call" {
                                                "📞"
                                            } else if ev.kind == "task" {
                                                "✓"
                                            } else {
                                                "🔔"
                                            },
                                            ev.hour,
                                            ev.minute,
                                            ev.title
                                        ))
                                        .size(10.0)
                                        .color(ev.color_fg),
                                    )
                                    .fill(ev.color_bg.gamma_multiply(0.35))
                                    .corner_radius(CornerRadius::same(3));

                                    if ui
                                        .add_sized([col_w - 8.0, 18.0], chip_btn)
                                        .on_hover_text(format!("{}\n{}", ev.title, ev.subtitle))
                                        .clicked()
                                    {
                                        event_clicked = true;
                                        if let Some(ref t) = ev.raw_task {
                                            *draft = TaskDraft {
                                                id: Some(t.id.clone()),
                                                title: t.title.clone(),
                                                description: t.description.clone(),
                                                priority: t.priority.clone(),
                                                status: if matches!(
                                                    t.status.as_str(),
                                                    "todo" | "doing" | "review"
                                                ) && matches!(
                                                    t.kind.as_str(),
                                                    "call" | "conversation"
                                                ) {
                                                    "scheduled".into()
                                                } else {
                                                    t.status.clone()
                                                },
                                                assignee: t.assignee.clone(),
                                                kind: t.kind.clone(),
                                                duration_minutes: t.duration_minutes.unwrap_or(30),
                                                call_result: t.call_result.clone(),
                                                next_step: t.next_step.clone(),
                                                revenue_reais: format_revenue_input(
                                                    t.revenue_cents,
                                                ),
                                                project_id: t.project_id.clone(),
                                                has_due: t.due_at.is_some(),
                                                due_date: date,
                                                due_hour: ev.hour,
                                                due_minute: ev.minute,
                                                chat_id: t.chat_id.clone(),
                                                link_chat: t.chat_id.is_some(),
                                            };
                                            *show_modal = true;
                                        }
                                    }
                                }

                                if day_events.len() > 3 {
                                    ui.label(
                                        RichText::new(format!("+{} mais", day_events.len() - 3))
                                            .size(9.0)
                                            .color(cp.dim),
                                    );
                                }
                            });

                        if cell_resp.clicked() && !event_clicked {
                            *draft = TaskDraft {
                                due_date: date,
                                due_hour: 9,
                                due_minute: 0,
                                has_due: true,
                                kind: "call".into(),
                                status: "scheduled".into(),
                                duration_minutes: 30,
                                ..TaskDraft::default()
                            };
                            *show_modal = true;
                        }
                    }
                }
            }
        });
        ui.add_space(3.0);
    }
}

pub fn show_agenda_workspace(app: &mut App, ui: &mut egui::Ui) {
    let cp = CrmPalette::from_app(&app.palette);
    let now = crate::util::now();
    let today = crate::util::today();
    let account_id = app.id.as_str().to_owned();

    let view_mode_id = ui.id().with(("crm-agenda-view-mode", &account_id));
    let anchor_date_id = ui.id().with(("crm-agenda-anchor-date", &account_id));
    let draft_id = ui.id().with(("crm-agenda-draft", &account_id));
    let modal_id = ui.id().with(("crm-agenda-modal-open", &account_id));

    let mut view_mode = ui.data_mut(|d| {
        d.get_temp::<AgendaViewMode>(view_mode_id)
            .unwrap_or(AgendaViewMode::Week)
    });
    let mut anchor_date = ui.data_mut(|d| d.get_temp::<Date>(anchor_date_id).unwrap_or(today));
    let mut draft = ui.data_mut(|d| d.get_temp::<TaskDraft>(draft_id).unwrap_or_default());
    let mut show_modal = ui.data_mut(|d| d.get_temp::<bool>(modal_id).unwrap_or(false));

    let bar_avail = ui.available_width();
    if bar_avail < 620.0 {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let today_btn = egui::Button::new(
                    RichText::new("Hoje").size(12.5).strong().color(
                        if anchor_date == today {
                            cp.on_accent
                        } else {
                            cp.text
                        },
                    ),
                )
                .fill(if anchor_date == today {
                    cp.accent
                } else {
                    cp.surface
                })
                .stroke(Stroke::new(
                    1.0,
                    if anchor_date == today {
                        cp.accent
                    } else {
                        cp.outline
                    },
                ))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(54.0, 30.0));
                if ui
                    .add(today_btn)
                    .on_hover_text("Ir para a data de hoje")
                    .clicked()
                {
                    anchor_date = today;
                    ui.ctx().request_repaint();
                }

                let prev_btn =
                    egui::Button::new(RichText::new("‹").size(16.0).strong().color(cp.text))
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(vec2(30.0, 30.0));
                if ui.add(prev_btn).clicked() {
                    anchor_date = match view_mode {
                        AgendaViewMode::Day => day_step(anchor_date, -1),
                        AgendaViewMode::Week => day_step(anchor_date, -7),
                        AgendaViewMode::Month => crate::util::month_step(anchor_date, -1),
                    };
                    ui.ctx().request_repaint();
                }

                let next_btn =
                    egui::Button::new(RichText::new("›").size(16.0).strong().color(cp.text))
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(vec2(30.0, 30.0));
                if ui.add(next_btn).clicked() {
                    anchor_date = match view_mode {
                        AgendaViewMode::Day => day_step(anchor_date, 1),
                        AgendaViewMode::Week => day_step(anchor_date, 7),
                        AgendaViewMode::Month => crate::util::month_step(anchor_date, 1),
                    };
                    ui.ctx().request_repaint();
                }

                for (mode, label) in [
                    (AgendaViewMode::Day, "Dia"),
                    (AgendaViewMode::Week, "Semana"),
                    (AgendaViewMode::Month, "Mês"),
                ] {
                    let is_sel = view_mode == mode;
                    let btn = egui::Button::new(
                        RichText::new(label).size(12.0).strong().color(if is_sel {
                            cp.on_accent
                        } else {
                            cp.secondary
                        }),
                    )
                    .fill(if is_sel { cp.accent } else { cp.surface })
                    .stroke(Stroke::new(1.0, if is_sel { cp.accent } else { cp.outline }))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(52.0, 30.0));
                    if ui.add(btn).clicked() {
                        view_mode = mode;
                        ui.ctx().request_repaint();
                    }
                }
            });

            let range_title = format_range_title(view_mode, anchor_date);
            ui.label(RichText::new(range_title).size(15.0).strong().color(cp.text));
        });
    } else {
        ui.horizontal(|ui| {
            let today_btn = egui::Button::new(
                RichText::new("Hoje").size(13.0).strong().color(
                    if anchor_date == today {
                        cp.on_accent
                    } else {
                        cp.text
                    },
                ),
            )
            .fill(if anchor_date == today {
                cp.accent
            } else {
                cp.surface
            })
            .stroke(Stroke::new(
                1.0,
                if anchor_date == today {
                    cp.accent
                } else {
                    cp.outline
                },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(58.0, 32.0));
            if ui
                .add(today_btn)
                .on_hover_text("Ir para a data de hoje")
                .clicked()
            {
                anchor_date = today;
                ui.ctx().request_repaint();
            }

            ui.add_space(3.0);

            let prev_btn = egui::Button::new(RichText::new("‹").size(17.0).strong().color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(32.0, 32.0));
            if ui.add(prev_btn).clicked() {
                anchor_date = match view_mode {
                    AgendaViewMode::Day => day_step(anchor_date, -1),
                    AgendaViewMode::Week => day_step(anchor_date, -7),
                    AgendaViewMode::Month => crate::util::month_step(anchor_date, -1),
                };
                ui.ctx().request_repaint();
            }

            let next_btn = egui::Button::new(RichText::new("›").size(17.0).strong().color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(32.0, 32.0));
            if ui.add(next_btn).clicked() {
                anchor_date = match view_mode {
                    AgendaViewMode::Day => day_step(anchor_date, 1),
                    AgendaViewMode::Week => day_step(anchor_date, 7),
                    AgendaViewMode::Month => crate::util::month_step(anchor_date, 1),
                };
                ui.ctx().request_repaint();
            }

            ui.add_space(8.0);

            let range_title = format_range_title(view_mode, anchor_date);
            ui.label(RichText::new(range_title).size(16.0).strong().color(cp.text));

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(6.0);
                let new_btn = egui::Button::new(
                    RichText::new("+ Novo Agendamento")
                        .size(13.0)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(145.0, 32.0));
                if ui.add(new_btn).clicked() {
                    draft = TaskDraft {
                        due_date: anchor_date,
                        due_hour: 9,
                        due_minute: 0,
                        has_due: true,
                        kind: "call".into(),
                        status: "scheduled".into(),
                        duration_minutes: 30,
                        ..TaskDraft::default()
                    };
                    show_modal = true;
                    ui.ctx().request_repaint();
                }

                ui.add_space(8.0);

                for (mode, label) in [
                    (AgendaViewMode::Month, "Mês"),
                    (AgendaViewMode::Week, "Semana"),
                    (AgendaViewMode::Day, "Dia"),
                ] {
                    let is_sel = view_mode == mode;
                    let btn = egui::Button::new(
                        RichText::new(label).size(12.5).strong().color(if is_sel {
                            cp.on_accent
                        } else {
                            cp.secondary
                        }),
                    )
                    .fill(if is_sel { cp.accent } else { cp.surface })
                    .stroke(Stroke::new(1.0, if is_sel { cp.accent } else { cp.outline }))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(64.0, 32.0));
                    if ui.add(btn).clicked() {
                        view_mode = mode;
                        ui.ctx().request_repaint();
                    }
                }
            });
        });
    }

    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Legenda:").size(11.5).color(cp.dim));

        Frame::new()
            .fill(Color32::from_rgb(0x02, 0x84, 0xc7).gamma_multiply(0.2))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("📞 Calls de Venda")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0x38, 0xbd, 0xf8)),
                );
            });

        Frame::new()
            .fill(Color32::from_rgb(0x04, 0x78, 0x57).gamma_multiply(0.2))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("✓ Tarefas")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0x34, 0xd3, 0x99)),
                );
            });

        Frame::new()
            .fill(Color32::from_rgb(0xb4, 0x53, 0x09).gamma_multiply(0.2))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("🔔 Follow-ups")
                        .size(11.0)
                        .strong()
                        .color(Color32::from_rgb(0xfb, 0xbf, 0x24)),
                );
            });
    });

    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| match view_mode {
            AgendaViewMode::Day => {
                render_agenda_day_view(app, ui, &cp, anchor_date, now, &mut draft, &mut show_modal);
            }
            AgendaViewMode::Week => {
                render_agenda_week_view(
                    app,
                    ui,
                    &cp,
                    anchor_date,
                    today,
                    now,
                    &mut draft,
                    &mut show_modal,
                );
            }
            AgendaViewMode::Month => {
                render_agenda_month_view(
                    app,
                    ui,
                    &cp,
                    anchor_date,
                    today,
                    now,
                    &mut draft,
                    &mut show_modal,
                );
            }
        });

    if show_modal {
        let mut is_open = true;
        let title = if draft.id.is_some() {
            if draft.kind == "call" {
                "Editar Call de Venda"
            } else {
                "Editar Tarefa"
            }
        } else if draft.kind == "call" {
            "Nova Call de Venda"
        } else {
            "Nova Tarefa"
        };
        egui::Window::new(title)
            .id(ui.id().with("crm-agenda-floating-modal"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(520.0, 580.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                if draft.id.is_none() {
                    ui.horizontal(|ui| {
                        let is_call = draft.kind == "call";
                        if ui.selectable_label(is_call, "📞 Call de Venda").clicked() {
                            draft.kind = "call".into();
                            draft.status = "scheduled".into();
                        }
                        if ui.selectable_label(!is_call, "✓ Tarefa").clicked() {
                            draft.kind = "task".into();
                            draft.status = "todo".into();
                        }
                    });
                    ui.add_space(8.0);
                }
                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .show(ui, |ui| {
                        if draft.kind == "call" {
                            render_call_modal(app, ui, &cp, &mut draft, now, &mut show_modal);
                        } else {
                            render_task_form_modal(app, ui, &cp, &mut draft, now, &mut show_modal);
                        }
                    });
            });
        if !is_open {
            show_modal = false;
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(view_mode_id, view_mode);
        d.insert_temp(anchor_date_id, anchor_date);
        d.insert_temp(draft_id, draft);
        d.insert_temp(modal_id, show_modal);
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
    let lost = all_calls
        .iter()
        .filter(|call| call.status == "lost")
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
    let evaluated = (held + lost).max(closings + lost);
    let conversion = if evaluated == 0 {
        0.0
    } else {
        closings as f32 * 100.0 / evaluated as f32
    };

    // 1. KPI Indicator Cards with Icons and Tailored Colors
    let avail_w = ui.available_width();
    let cols = if avail_w > 1200.0 {
        9
    } else if avail_w > 900.0 {
        5
    } else if avail_w > 650.0 {
        4
    } else if avail_w > 450.0 {
        3
    } else {
        2
    };
    let spacing = 8.0;
    let metric_w = ((avail_w - (cols as f32 - 1.0) * spacing) / cols as f32).max(110.0);

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
            Icon::CircleAlert,
            "Perdidas",
            lost.to_string(),
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

    // 2. Charts & Analytic Breakdown Cards (Responsive: Side by Side on wide screens, stacked on narrow)
    let render_card_distribution = |ui: &mut egui::Ui| {
        Frame::new()
            .fill(cp.panel)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
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
    };

    let render_card_losses = |ui: &mut egui::Ui| {
        Frame::new()
            .fill(cp.panel)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleAlert, 16.0, cp.danger);
                    ui.label(
                        RichText::new("Perdas: Descarte, Cancelamentos & No-Shows")
                            .size(14.0)
                            .strong()
                            .color(cp.text),
                    );
                });
                ui.add_space(8.0);

                let lost_total = lost + no_show + cancelled;
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
                            "• {lost} Perdida(s)  • {no_show} No-show(s)  • {cancelled} Cancelada(s)"
                        ))
                        .size(11.5)
                        .color(cp.dim),
                    );
                });
            });
    };

    if avail_w < 640.0 {
        render_card_distribution(ui);
        ui.add_space(10.0);
        render_card_losses(ui);
    } else {
        ui.columns(2, |uis| {
            render_card_distribution(&mut uis[0]);
            render_card_losses(&mut uis[1]);
        });
    }

    ui.add_space(14.0);

    // 3. Filters & Action Bar (Fully responsive: never clips buttons or cuts off at right edge)
    let bar_avail = ui.available_width();
    let status_text = match status_filter.as_str() {
        "scheduled" => "Agendadas",
        "held" => "Realizadas",
        "lost" => "Perdidas",
        "no_show" => "No-show",
        "cancelled" => "Canceladas",
        "closed" => "Fechamentos",
        _ => "Todos os status",
    };

    let mut show_period_filter = |ui: &mut egui::Ui| {
        egui::ComboBox::from_id_salt(("calls-period-filter", &account_id))
            .selected_text(if period == 0 {
                "Todo o período".into()
            } else {
                format!("Últimos {period} dias")
            })
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut period, 30, "Últimos 30 dias").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut period, 90, "Últimos 90 dias").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut period, 365, "Último ano").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut period, 0, "Todo o período").clicked() {
                    ui.ctx().request_repaint();
                }
            });
    };

    let mut show_status_filter = |ui: &mut egui::Ui| {
        egui::ComboBox::from_id_salt(("calls-status-filter", &account_id))
            .selected_text(status_text)
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut status_filter, "all".into(), "Todos os status").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "scheduled".into(), "Agendadas").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "held".into(), "Realizadas").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "closed".into(), "Fechamentos").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "lost".into(), "Perdidas").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "no_show".into(), "No-show").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(&mut status_filter, "cancelled".into(), "Canceladas").clicked() {
                    ui.ctx().request_repaint();
                }
            });
    };

    let mut new_call_clicked = false;
    let mut export_csv_clicked = false;

    if bar_avail >= 880.0 {
        ui.horizontal(|ui| {
            let search_w = (bar_avail - 520.0).clamp(160.0, 320.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, cliente, responsável, resultado…")
                    .desired_width(search_w),
            );

            show_period_filter(ui);
            show_status_filter(ui);

            ui.add_space(8.0);
            let btn_call = egui::Button::new(
                RichText::new("+ Nova call")
                    .size(13.5)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(130.0, 34.0));
            if ui.add(btn_call).clicked() {
                new_call_clicked = true;
            }

            let btn_export = egui::Button::new(
                RichText::new("📥 Exportar CSV")
                    .size(13.0)
                    .strong()
                    .color(cp.text),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(120.0, 34.0));
            if ui
                .add(btn_export)
                .on_hover_text("Exportar todas as calls do período para planilha CSV")
                .clicked()
            {
                export_csv_clicked = true;
            }
        });
    } else if bar_avail >= 560.0 {
        ui.horizontal(|ui| {
            let search_w = (bar_avail - 280.0).max(140.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, cliente…")
                    .desired_width(search_w),
            );

            let btn_call = egui::Button::new(
                RichText::new("+ Nova call")
                    .size(13.0)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(115.0, 32.0));
            if ui.add(btn_call).clicked() {
                new_call_clicked = true;
            }

            let btn_export = egui::Button::new(
                RichText::new("📥 CSV")
                    .size(12.5)
                    .strong()
                    .color(cp.text),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(75.0, 32.0));
            if ui.add(btn_export).clicked() {
                export_csv_clicked = true;
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            show_period_filter(ui);
            show_status_filter(ui);
        });
    } else {
        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("Buscar por título, cliente…")
                .desired_width(bar_avail),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let btn_call = egui::Button::new(
                RichText::new("+ Nova call")
                    .size(13.0)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2((bar_avail * 0.55).max(110.0), 32.0));
            if ui.add(btn_call).clicked() {
                new_call_clicked = true;
            }

            let btn_export = egui::Button::new(
                RichText::new("📥 CSV")
                    .size(12.5)
                    .strong()
                    .color(cp.text),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2((bar_avail * 0.4).max(70.0), 32.0));
            if ui.add(btn_export).clicked() {
                export_csv_clicked = true;
            }
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            show_period_filter(ui);
            show_status_filter(ui);
        });
    }

    if new_call_clicked {
        draft = TaskDraft {
            kind: "call".into(),
            status: "scheduled".into(),
            ..TaskDraft::default()
        };
        show_form = true;
        ui.ctx().request_repaint();
    }

    if export_csv_clicked {
        export_calls_csv(app, &all_calls);
    }

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
            .resizable(true)
            .default_width(520.0)
            .default_height(620.0)
            .max_height(760.0)
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
            ui.ctx().request_repaint();
        }
    } else {
        ui.spacing_mut().item_spacing.y = 8.0;
        for call in filtered_calls {
            render_call_card(app, ui, &cp, &call, now, &mut draft, &mut show_form);
        }
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
        "lost" => (
            Color32::from_rgb(0x45, 0x0a, 0x0a),
            Color32::from_rgb(0xf8, 0x71, 0x71),
            "Perdida",
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
                let card_avail = ui.available_width();
                if card_avail < 780.0 {
                    ui.horizontal(|ui| {
                        call_status_pill(ui, &call.status, cp);
                        ui.add_space(4.0);
                        let title_w = (card_avail - 100.0).max(80.0);
                        ui.allocate_ui_with_layout(
                            vec2(title_w, 22.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&call.title)
                                            .size(14.0)
                                            .strong()
                                            .color(cp.text),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
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
                            ui.add_space(14.0);
                            if ui.small_button("Editar").clicked() {
                                *draft = TaskDraft {
                                    id: Some(call.id.clone()),
                                    title: call.title.clone(),
                                    description: call.description.clone(),
                                    priority: call.priority.clone(),
                                    status: if matches!(call.status.as_str(), "todo" | "doing" | "review") {
                                        "scheduled".into()
                                    } else {
                                        call.status.clone()
                                    },
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
                                ui.ctx().request_repaint();
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
                                    ui.ctx().request_repaint();
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
                                    ui.ctx().request_repaint();
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
                                ui.data_mut(|d| d.insert_temp(delete_id, true));
                                ui.ctx().request_repaint();
                            }

                            if let Some(chat) = &call.chat_id {
                                if ui.small_button("Conversar").clicked() {
                                    app.actions.push(Action::OpenChat(chat.clone()));
                                }
                            }
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        call_status_pill(ui, &call.status, cp);
                        ui.add_space(6.0);

                        let title_w = (card_avail - 380.0).max(120.0);
                        ui.allocate_ui_with_layout(
                            vec2(title_w, 24.0),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&call.title)
                                            .size(14.5)
                                            .strong()
                                            .color(cp.text),
                                    )
                                    .truncate(),
                                );
                            },
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
                            ui.add_space(14.0);
                            if ui.small_button("Editar").clicked() {
                                *draft = TaskDraft {
                                    id: Some(call.id.clone()),
                                    title: call.title.clone(),
                                    description: call.description.clone(),
                                    priority: call.priority.clone(),
                                    status: if matches!(call.status.as_str(), "todo" | "doing" | "review") {
                                        "scheduled".into()
                                    } else {
                                        call.status.clone()
                                    },
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
                                ui.ctx().request_repaint();
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
                                    ui.ctx().request_repaint();
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
                                    ui.ctx().request_repaint();
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
                                ui.data_mut(|d| d.insert_temp(delete_id, true));
                                ui.ctx().request_repaint();
                            }

                            if let Some(chat) = &call.chat_id {
                                if ui.small_button("Conversar").clicked() {
                                    app.actions.push(Action::OpenChat(chat.clone()));
                                }
                            }
                        });
                    });
                }

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

                render_call_outcome_bar(app, ui, cp, call);
            });
        });
}

fn render_call_outcome_bar(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette, call: &CrmTask) {
    let win_id = egui::Id::new(("call-win-prompt", &call.id));
    let lost_id = egui::Id::new(("call-lost-prompt", &call.id));
    let mut show_win = ui.data_mut(|d| d.get_temp::<bool>(win_id).unwrap_or(false));
    let mut show_lost = ui.data_mut(|d| d.get_temp::<bool>(lost_id).unwrap_or(false));

    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Ações rápidas:").size(11.5).color(cp.dim));

        // 1. Realizada
        let is_held = call.status == "held";
        let held_btn = egui::Button::new(
            RichText::new(if is_held { "✓ Realizada" } else { "Realizada" })
                .size(11.5)
                .strong()
                .color(if is_held { cp.on_accent } else { cp.text }),
        )
        .fill(if is_held { Color32::from_rgb(0x06, 0x4e, 0x3b) } else { cp.surface_hover })
        .stroke(Stroke::new(1.0, if is_held { Color32::from_rgb(0x34, 0xd3, 0x99) } else { cp.outline }))
        .corner_radius(CornerRadius::same(5));
        if ui.add(held_btn).on_hover_text("Marcar esta call como Realizada").clicked() {
            let mut updated = call.clone();
            updated.status = "held".into();
            if updated.call_result.is_empty() {
                updated.call_result = "Realizada".into();
            }
            updated.completed_at = Some(crate::util::now());
            updated.updated_at = crate::util::now();
            app.actions.push(Action::SaveCrmTask(updated));
            app.toast("Call marcada como Realizada!");
            ui.ctx().request_repaint();
        }

        // 2. Venda Fechada
        let is_closed = call.status == "closed";
        let won_btn = egui::Button::new(
            RichText::new(if is_closed { "💰 Venda fechada ✓" } else { "💰 Venda fechada" })
                .size(11.5)
                .strong()
                .color(if is_closed { cp.on_accent } else { cp.accent }),
        )
        .fill(if is_closed { cp.accent } else { cp.accent.gamma_multiply(0.16) })
        .stroke(Stroke::new(1.0, cp.accent))
        .corner_radius(CornerRadius::same(5));
        if ui.add(won_btn).on_hover_text("Registrar venda ganha, informar valor e atualizar funil/receita").clicked() {
            show_win = !show_win;
            show_lost = false;
            ui.data_mut(|d| {
                d.insert_temp(win_id, show_win);
                d.insert_temp(lost_id, false);
            });
            ui.ctx().request_repaint();
        }

        // 3. Perdida
        let is_lost = call.status == "lost";
        let lost_btn = egui::Button::new(
            RichText::new(if is_lost { "✕ Perdida ✓" } else { "✕ Perdida" })
                .size(11.5)
                .strong()
                .color(if is_lost { Color32::WHITE } else { cp.danger }),
        )
        .fill(if is_lost { cp.danger } else { cp.danger.gamma_multiply(0.14) })
        .stroke(Stroke::new(1.0, cp.danger))
        .corner_radius(CornerRadius::same(5));
        if ui.add(lost_btn).on_hover_text("Registrar perda da oportunidade com motivo").clicked() {
            show_lost = !show_lost;
            show_win = false;
            ui.data_mut(|d| {
                d.insert_temp(lost_id, show_lost);
                d.insert_temp(win_id, false);
            });
            ui.ctx().request_repaint();
        }
    });

    if show_win {
        let val_id = egui::Id::new(("call-win-val", &call.id));
        let default_val = if call.revenue_cents > 0 {
            format_revenue_input(call.revenue_cents)
        } else if let Some(chat_id) = &call.chat_id {
            app.crm_deals.get(chat_id).map(|d| format_revenue_input(d.value_cents)).unwrap_or_default()
        } else {
            String::new()
        };
        let mut val_str = ui.data_mut(|d| d.get_temp::<String>(val_id).unwrap_or(default_val));

        ui.add_space(4.0);
        Frame::new()
            .fill(cp.surface_hover)
            .stroke(Stroke::new(1.0, cp.accent))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Valor da Venda (R$):").size(12.0).strong().color(cp.text));
                    ui.add(egui::TextEdit::singleline(&mut val_str).hint_text("0,00").desired_width(110.0));

                    let confirm_btn = egui::Button::new(
                        RichText::new("✓ Confirmar Fechamento").size(12.0).strong().color(cp.on_accent)
                    ).fill(cp.accent).corner_radius(CornerRadius::same(5));

                    if ui.add(confirm_btn).clicked() {
                        let cents = parse_revenue_cents(&val_str);
                        let now = crate::util::now();
                        let mut updated_call = call.clone();
                        updated_call.status = "closed".into();
                        updated_call.call_result = "Venda Fechada".into();
                        updated_call.revenue_cents = cents;
                        updated_call.completed_at = Some(now);
                        updated_call.updated_at = now;
                        app.actions.push(Action::SaveCrmTask(updated_call));

                        if let Some(chat_id) = &call.chat_id {
                            let mut deal = app.crm_deals.get(chat_id).cloned().unwrap_or_else(|| CrmDeal {
                                chat_id: chat_id.clone(),
                                column_id: "close".into(),
                                value_cents: cents,
                                notes: String::new(),
                                tags: Vec::new(),
                                updated_at: now,
                            });
                            deal.value_cents = cents;
                            if let Some(close_col) = app.crm_columns.iter().find(|c| c.id == "close" || c.title.to_lowercase().contains("fechamento") || c.title.to_lowercase().contains("ganho")) {
                                deal.column_id = close_col.id.clone();
                            }
                            let formatted_val = format_revenue_input(cents);
                            let entry = format!("💰 Venda fechada via Call '{}': R$ {}\n", call.title, formatted_val);
                            deal.notes = format!("{entry}{}", deal.notes);
                            deal.updated_at = now;
                            app.actions.push(Action::SaveCrmDeal(deal));
                        }
                        app.toast("Venda fechada! Receita, funil e histórico do cliente atualizados.");
                        ui.data_mut(|d| d.insert_temp(win_id, false));
                        ui.ctx().request_repaint();
                    }

                    if ui.small_button("Cancelar").clicked() {
                        ui.data_mut(|d| d.insert_temp(win_id, false));
                        ui.ctx().request_repaint();
                    }
                });
            });
        ui.data_mut(|d| d.insert_temp(val_id, val_str));
    }

    if show_lost {
        let reason_id = egui::Id::new(("call-lost-reason", &call.id));
        let mut reason_str = ui.data_mut(|d| d.get_temp::<String>(reason_id).unwrap_or_default());

        ui.add_space(4.0);
        Frame::new()
            .fill(cp.surface_hover)
            .stroke(Stroke::new(1.0, cp.danger))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Motivo da Perda:").size(12.0).strong().color(cp.text));
                        ui.add(egui::TextEdit::singleline(&mut reason_str).hint_text("Ex: Preço alto, escolheu concorrente...").desired_width(240.0));

                        let confirm_btn = egui::Button::new(
                            RichText::new("✓ Registrar Perda").size(12.0).strong().color(Color32::WHITE)
                        ).fill(cp.danger).corner_radius(CornerRadius::same(5));

                        if ui.add(confirm_btn).clicked() {
                            let now = crate::util::now();
                            let reason = if reason_str.trim().is_empty() { "Não informado".to_string() } else { reason_str.trim().to_string() };
                            let mut updated_call = call.clone();
                            updated_call.status = "lost".into();
                            updated_call.call_result = format!("Perdida: {reason}");
                            updated_call.next_step = format!("Perda: {reason}");
                            updated_call.completed_at = Some(now);
                            updated_call.updated_at = now;
                            app.actions.push(Action::SaveCrmTask(updated_call));

                            if let Some(chat_id) = &call.chat_id {
                                if let Some(mut deal) = app.crm_deals.get(chat_id).cloned() {
                                    deal.notes = format!("✕ Call perdida '{}' - Motivo: {reason}\n{}", call.title, deal.notes);
                                    deal.updated_at = now;
                                    app.actions.push(Action::SaveCrmDeal(deal));
                                }
                            }
                            app.toast("Call registrada como Perdida.");
                            ui.data_mut(|d| d.insert_temp(lost_id, false));
                            ui.ctx().request_repaint();
                        }

                        if ui.small_button("Cancelar").clicked() {
                            ui.data_mut(|d| d.insert_temp(lost_id, false));
                            ui.ctx().request_repaint();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Sugestões:").size(11.0).color(cp.dim));
                        for chip in ["Preço / Sem orçamento", "Optou por concorrente", "Sem interesse no momento", "Não respondeu"] {
                            if ui.small_button(chip).clicked() {
                                reason_str = chip.to_string();
                            }
                        }
                    });
                });
            });
        ui.data_mut(|d| d.insert_temp(reason_id, reason_str));
    }
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
    egui::ScrollArea::vertical()
        .max_height(480.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
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
                        .selected_text(match draft.status.as_str() {
                            "scheduled" | "todo" | "doing" | "review" => "Agendada",
                            "held" | "done" => "Realizada",
                            "closed" => "Fechamento / Vendida",
                            "lost" => "Perdida",
                            "no_show" => "No-show",
                            "cancelled" => "Cancelada",
                            _ => "Agendada",
                        })
                        .width(ui.available_width() - 8.0)
                        .show_ui(ui, |ui| {
                            for (value, label) in [
                                ("scheduled", "Agendada"),
                                ("held", "Realizada"),
                                ("closed", "Fechamento / Vendida"),
                                ("lost", "Perdida"),
                                ("no_show", "No-show"),
                                ("cancelled", "Cancelada"),
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

            if draft.status == "lost" {
                ui.add_space(8.0);
                Frame::new()
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.danger.gamma_multiply(0.7)))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Motivo da Perda / Descarte *").size(12.0).strong().color(cp.danger));
                        ui.add_space(3.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut draft.next_step)
                                .hint_text("Descreva o motivo principal do descarte...")
                                .desired_width(ui.available_width())
                        );
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Sugestões:").size(11.0).color(cp.dim));
                            for chip in ["Preço / Sem orçamento", "Optou por concorrente", "Sem interesse no momento", "Não respondeu"] {
                                if ui.small_button(chip).clicked() {
                                    draft.next_step = chip.to_string();
                                    if draft.call_result.is_empty() {
                                        draft.call_result = format!("Perdida: {chip}");
                                    }
                                }
                            }
                        });
                    });
            }

            if draft.status == "closed" {
                ui.add_space(8.0);
                Frame::new()
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.accent.gamma_multiply(0.7)))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.label(RichText::new("💰 Valor Fechado da Venda (R$)").size(12.0).strong().color(cp.accent));
                        ui.add_space(3.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut draft.revenue_reais)
                                .hint_text("0,00")
                                .desired_width(ui.available_width())
                        );
                    });
            }

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
        });

        ui.add_space(12.0);

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

            if let Some(call_id) = draft.id.clone() {
                let confirm_delete_id = ui.id().with(("confirm-del-modal-call", &call_id));
                let confirming = ui.data_mut(|d| d.get_temp::<bool>(confirm_delete_id).unwrap_or(false));
                if confirming {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Confirmar Exclusão?")
                                    .size(12.5)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(cp.danger)
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(130.0, 36.0)),
                        )
                        .clicked()
                    {
                        app.actions.push(Action::DeleteCrmTask(call_id));
                        app.toast("Call excluída.");
                        *show_form = false;
                        *draft = TaskDraft {
                            kind: "call".into(),
                            status: "scheduled".into(),
                            ..TaskDraft::default()
                        };
                        ui.data_mut(|d| d.insert_temp(confirm_delete_id, false));
                        ui.ctx().request_repaint();
                    }
                    if ui.small_button("Não").clicked() {
                        ui.data_mut(|d| d.insert_temp(confirm_delete_id, false));
                        ui.ctx().request_repaint();
                    }
                } else if ui
                    .add(
                        egui::Button::new(
                            RichText::new("Excluir Call")
                                .size(12.5)
                                .color(cp.danger),
                        )
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.danger.gamma_multiply(0.6)))
                        .corner_radius(CornerRadius::same(8))
                        .min_size(vec2(100.0, 36.0)),
                    )
                    .on_hover_text("Excluir esta call permanentemente")
                    .clicked()
                {
                    ui.data_mut(|d| d.insert_temp(confirm_delete_id, true));
                    ui.ctx().request_repaint();
                }
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

                    let target_chat = draft.chat_id.clone().or_else(|| app.open_chat.clone());
                    let rev_cents = parse_revenue_cents(&draft.revenue_reais);
                    let mut final_status = draft.status.clone();
                    if matches!(final_status.as_str(), "todo" | "doing" | "review") {
                        final_status = "scheduled".into();
                    }

                    app.actions.push(Action::SaveCrmTask(CrmTask {
                        id: draft.id.clone().unwrap_or_else(new_task_id),
                        chat_id: target_chat.clone(),
                        project_id: draft.project_id.clone(),
                        title: draft.title.trim().into(),
                        description: draft.description.trim().into(),
                        status: final_status.clone(),
                        priority: "normal".into(),
                        kind: "call".into(),
                        duration_minutes: Some(draft.duration_minutes),
                        assignee: draft.assignee.trim().into(),
                        call_result: draft.call_result.trim().into(),
                        next_step: draft.next_step.trim().into(),
                        revenue_cents: rev_cents,
                        due_at: crate::util::to_unix_seconds(
                            draft.due_date,
                            draft.due_hour,
                            draft.due_minute,
                        ),
                        completed_at: matches!(
                            final_status.as_str(),
                            "held" | "no_show" | "cancelled" | "closed" | "lost"
                        )
                        .then_some(now),
                        created_at: old.map_or(now, |task| task.created_at),
                        updated_at: now,
                    }));

                    if let Some(ref chat_id) = target_chat {
                        if final_status == "closed" || rev_cents > 0 {
                            let mut deal = app.crm_deals.get(chat_id).cloned().unwrap_or_else(|| CrmDeal {
                                chat_id: chat_id.clone(),
                                column_id: "close".into(),
                                value_cents: rev_cents,
                                notes: String::new(),
                                tags: Vec::new(),
                                updated_at: now,
                            });
                            if rev_cents > 0 {
                                deal.value_cents = rev_cents;
                            }
                            if let Some(close_col) = app.crm_columns.iter().find(|c| c.id == "close" || c.title.to_lowercase().contains("fechamento") || c.title.to_lowercase().contains("ganho")) {
                                deal.column_id = close_col.id.clone();
                            }
                            let formatted_val = format_revenue_input(rev_cents);
                            let entry = format!("💰 Venda fechada via Call '{}': R$ {}\n", draft.title, formatted_val);
                            deal.notes = format!("{entry}{}", deal.notes);
                            deal.updated_at = now;
                            app.actions.push(Action::SaveCrmDeal(deal));
                        } else if final_status == "lost" {
                            if let Some(mut deal) = app.crm_deals.get(chat_id).cloned() {
                                let entry = format!("✕ Call perdida '{}' (Resultado: {}): {}\n", draft.title, draft.call_result, draft.next_step);
                                deal.notes = format!("{entry}{}", deal.notes);
                                deal.updated_at = now;
                                app.actions.push(Action::SaveCrmDeal(deal));
                            }
                        }
                    }

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

    // 1. Top Filters & Action Row (responsive layout: zero overlap between "+ Nova tarefa" and Kanban/Lista)
    let avail = ui.available_width();

    let render_switcher = |ui: &mut egui::Ui, view_mode: &mut String| {
        let is_kanban = view_mode.as_str() == "kanban";
        let kanban_btn = egui::Button::new(
            RichText::new("⊞ Kanban")
                .size(12.0)
                .strong()
                .color(if is_kanban { cp.on_accent } else { cp.secondary }),
        )
        .fill(if is_kanban { cp.accent } else { cp.surface })
        .stroke(Stroke::new(1.0, if is_kanban { cp.accent } else { cp.outline }))
        .corner_radius(CornerRadius::same(6))
        .min_size(vec2(76.0, 32.0));
        if ui.add(kanban_btn).clicked() {
            *view_mode = "kanban".into();
            ui.ctx().request_repaint();
        }

        let is_list = view_mode.as_str() == "list";
        let list_btn = egui::Button::new(
            RichText::new("☰ Lista")
                .size(12.0)
                .strong()
                .color(if is_list { cp.on_accent } else { cp.secondary }),
        )
        .fill(if is_list { cp.accent } else { cp.surface })
        .stroke(Stroke::new(1.0, if is_list { cp.accent } else { cp.outline }))
        .corner_radius(CornerRadius::same(6))
        .min_size(vec2(70.0, 32.0));
        if ui.add(list_btn).clicked() {
            *view_mode = "list".into();
            ui.ctx().request_repaint();
        }
    };

    let mut new_task_clicked = false;
    let render_new_task_btn = |ui: &mut egui::Ui, clicked: &mut bool| {
        let new_task_btn = egui::Button::new(
            RichText::new("+ Nova tarefa")
                .size(13.0)
                .strong()
                .color(cp.on_accent),
        )
        .fill(cp.accent)
        .corner_radius(CornerRadius::same(8))
        .min_size(vec2(125.0, 32.0));
        if ui.add(new_task_btn).clicked() {
            *clicked = true;
            ui.ctx().request_repaint();
        }
    };

    let render_combos = |ui: &mut egui::Ui, project_filter: &mut Option<String>, priority_filter: &mut String, app: &App| {
        let selected_proj_title = project_filter
            .as_ref()
            .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
            .map(|p| p.name.as_str())
            .unwrap_or("Todos os projetos");

        egui::ComboBox::from_id_salt(("task-board-proj-filter", &account_id))
            .selected_text(selected_proj_title)
            .show_ui(ui, |ui| {
                if ui.selectable_value(project_filter, None, "Todos os projetos").clicked() {
                    ui.ctx().request_repaint();
                }
                for p in &app.crm_projects {
                    if ui.selectable_value(project_filter, Some(p.id.clone()), &p.name).clicked() {
                        ui.ctx().request_repaint();
                    }
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
                if ui.selectable_value(priority_filter, "all".into(), "Todas as prioridades").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(priority_filter, "urgent".into(), "Urgente").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(priority_filter, "high".into(), "Alta").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(priority_filter, "normal".into(), "Normal").clicked() {
                    ui.ctx().request_repaint();
                }
                if ui.selectable_value(priority_filter, "low".into(), "Baixa").clicked() {
                    ui.ctx().request_repaint();
                }
            });
    };

    if avail >= 1040.0 {
        // Single line layout with plenty of room
        ui.horizontal(|ui| {
            let search_w = (avail - 680.0).clamp(160.0, 300.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, descrição, cliente...")
                    .desired_width(search_w),
            );

            render_combos(ui, &mut project_filter, &mut priority_filter, app);

            ui.add_space(4.0);
            render_switcher(ui, &mut view_mode);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(14.0); // Margin away from scrollbar
                render_new_task_btn(ui, &mut new_task_clicked);
            });
        });
    } else if avail >= 680.0 {
        // 2-row layout: Search & Switcher & New Task on row 1; Filters on row 2
        ui.horizontal(|ui| {
            let search_w = (avail - 360.0).max(140.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar por título, descrição...")
                    .desired_width(search_w),
            );

            render_switcher(ui, &mut view_mode);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(14.0);
                render_new_task_btn(ui, &mut new_task_clicked);
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            render_combos(ui, &mut project_filter, &mut priority_filter, app);
        });
    } else {
        // Narrow layout: 3 rows, zero clipping
        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("Buscar tarefas...")
                .desired_width(avail - 10.0),
        );

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            render_switcher(ui, &mut view_mode);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(10.0);
                render_new_task_btn(ui, &mut new_task_clicked);
            });
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            render_combos(ui, &mut project_filter, &mut priority_filter, app);
        });
    }

    if new_task_clicked {
        draft = TaskDraft::default();
        show_form = true;
        ui.ctx().request_repaint();
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
                    let card_avail = ui.available_width();
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

                    let mut render_actions = |ui: &mut egui::Ui| {
                        let delete_id = egui::Id::new(("task-list-delete-confirm", &task.id));
                        let mut is_confirming =
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
                                is_confirming = false;
                                ui.ctx().request_repaint();
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
                                ui.ctx().request_repaint();
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
                            ui.ctx().request_repaint();
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
                            ui.ctx().request_repaint();
                        }
                    };

                    let render_info = |ui: &mut egui::Ui| {
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
                    };

                    if card_avail < 780.0 {
                        ui.vertical(|ui| {
                            // Row 1: Checkbox, Status badge, Priority badge, Title
                            ui.horizontal(|ui| {
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
                                priority_badge(ui, &task.priority, cp);
                                ui.add_space(6.0);

                                let title_w = (ui.available_width() - 8.0).max(60.0);
                                ui.allocate_ui_with_layout(
                                    vec2(title_w, 24.0),
                                    Layout::left_to_right(Align::Center),
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
                                    },
                                );
                            });

                            ui.add_space(4.0);

                            // Row 2: Secondary info on left, action buttons on right
                            ui.horizontal(|ui| {
                                render_info(ui);

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.add_space(8.0);
                                    render_actions(ui);
                                });
                            });
                        });
                    } else {
                        // Wide single-row layout
                        ui.horizontal(|ui| {
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
                            priority_badge(ui, &task.priority, cp);
                            ui.add_space(6.0);

                            let row_avail = ui.available_width();
                            let title_max_w = (row_avail - 420.0).max(100.0);
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

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.add_space(8.0);
                                render_actions(ui);
                                render_info(ui);
                            });
                        });
                    }
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
                ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
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
                });
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
        "lost" => "Perdida",
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
