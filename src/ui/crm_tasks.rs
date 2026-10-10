//! Account-scoped CRM task and follow-up inbox.

use crate::app::App;
use crate::model::{Action, CrmFollowup, CrmTask};
use crate::theme::{self, Icon};
use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke};
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
    kind: String,
    duration_minutes: u16,
    project_id: Option<String>,
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
            kind: "task".into(),
            duration_minutes: 30,
            project_id: None,
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
    show_inner(app, ui, false, Some(TaskView::Tasks));
}

pub fn show_agenda_workspace(app: &mut App, ui: &mut egui::Ui) {
    let account_id = app.id.as_str().to_owned();
    let draft_id = ui.id().with(("crm-task-draft", &account_id));
    let mut draft = ui.data_mut(|d| d.get_temp::<TaskDraft>(draft_id).unwrap_or_default());
    show_calendar(app, ui, crate::util::now(), &mut draft);
    ui.data_mut(|d| d.insert_temp(draft_id, draft));
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

    Frame::new()
        .fill(p.surface)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            theme::text(
                ui,
                if draft.id.is_some() {
                    "Editar tarefa"
                } else {
                    "Nova tarefa"
                },
                theme::semibold(13.0),
                p.text,
            );
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("task-kind")
                    .selected_text(if draft.kind == "conversation" {
                        "Conversa agendada"
                    } else {
                        "Tarefa"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.kind, "task".into(), "Tarefa");
                        ui.selectable_value(
                            &mut draft.kind,
                            "conversation".into(),
                            "Conversa agendada",
                        );
                    });
                ui.add_sized(
                    [260.0, 28.0],
                    egui::TextEdit::singleline(&mut draft.title)
                        .hint_text("Ex.: preparar proposta"),
                );
                egui::ComboBox::from_id_salt("task-priority")
                    .selected_text(priority_label(&draft.priority))
                    .show_ui(ui, |ui| {
                        for (v, l) in [
                            ("low", "Baixa"),
                            ("normal", "Normal"),
                            ("high", "Alta"),
                            ("urgent", "Urgente"),
                        ] {
                            ui.selectable_value(&mut draft.priority, v.into(), l);
                        }
                    });
                ui.checkbox(&mut draft.has_due, "Definir prazo");
            });
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("task-project")
                    .selected_text(
                        draft
                            .project_id
                            .as_ref()
                            .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                            .map_or("Sem projeto", |p| p.name.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.project_id, None, "Sem projeto");
                        for project in &app.crm_projects {
                            ui.selectable_value(
                                &mut draft.project_id,
                                Some(project.id.clone()),
                                &project.name,
                            );
                        }
                    });
                if draft.kind == "conversation" {
                    ui.label("Duração");
                    egui::ComboBox::from_id_salt("conversation-duration")
                        .selected_text(format!("{} min", draft.duration_minutes))
                        .show_ui(ui, |ui| {
                            for minutes in [15, 30, 45, 60, 90, 120] {
                                ui.selectable_value(
                                    &mut draft.duration_minutes,
                                    minutes,
                                    format!("{minutes} min"),
                                );
                            }
                        });
                }
            });
            if draft.has_due {
                super::dialogs::date_time_picker(
                    ui,
                    app.locale,
                    &p,
                    "crm-task-due",
                    &mut draft.due_date,
                    &mut draft.due_hour,
                    &mut draft.due_minute,
                );
            }
            ui.add_sized(
                [ui.available_width(), 44.0],
                egui::TextEdit::multiline(&mut draft.description)
                    .hint_text("Descrição ou próximo passo (opcional)"),
            );
            ui.horizontal(|ui| {
                if app.open_chat.is_some() {
                    ui.checkbox(&mut draft.link_chat, "Vincular à conversa aberta");
                } else {
                    draft.link_chat = false;
                    theme::text(
                        ui,
                        "Abra uma conversa para vincular ao contato.",
                        theme::regular(11.0),
                        p.dim,
                    );
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            !draft.title.trim().is_empty(),
                            egui::Button::new(if draft.id.is_some() {
                                "Salvar alterações"
                            } else {
                                "Criar tarefa"
                            }),
                        )
                        .clicked()
                    {
                        let old = draft
                            .id
                            .as_ref()
                            .and_then(|id| app.crm_tasks.iter().find(|x| &x.id == id))
                            .cloned();
                        app.actions.push(Action::SaveCrmTask(CrmTask {
                            id: draft.id.clone().unwrap_or_else(new_task_id),
                            chat_id: if draft.link_chat {
                                app.open_chat.clone()
                            } else {
                                None
                            },
                            project_id: draft.project_id.clone(),
                            title: draft.title.trim().into(),
                            description: draft.description.trim().into(),
                            status: old
                                .as_ref()
                                .map_or_else(|| "todo".into(), |x| x.status.clone()),
                            priority: draft.priority.clone(),
                            kind: draft.kind.clone(),
                            duration_minutes: (draft.kind == "conversation")
                                .then_some(draft.duration_minutes),
                            due_at: draft.has_due.then(|| {
                                crate::util::to_unix_seconds(
                                    draft.due_date,
                                    draft.due_hour,
                                    draft.due_minute,
                                )
                                .unwrap_or(now)
                            }),
                            completed_at: old.as_ref().and_then(|x| x.completed_at),
                            created_at: old.as_ref().map_or(now, |x| x.created_at),
                            updated_at: now,
                        }));
                        draft = TaskDraft::default();
                    }
                    if draft.id.is_some() && ui.small_button("Cancelar edição").clicked() {
                        draft = TaskDraft::default();
                    }
                });
            });
        });

    ui.add_space(8.0);
    egui::ScrollArea::vertical()
        .max_height(440.0)
        .show(ui, |ui| {
            theme::text(ui, "Tarefas de trabalho", theme::semibold(14.0), p.text);
            if !app.crm_tasks.iter().any(|task| task.kind != "conversation") {
                theme::text(ui, "Nenhuma tarefa criada.", theme::regular(12.0), p.dim);
            }
            for task in app
                .crm_tasks
                .clone()
                .into_iter()
                .filter(|task| task.kind != "conversation")
            {
                task_row(app, ui, &task, now, &mut draft);
                ui.add_space(6.0);
            }
            ui.add_space(8.0);
            theme::text(ui, "Conversas agendadas", theme::semibold(14.0), p.text);
            if !app.crm_tasks.iter().any(|task| task.kind == "conversation") {
                theme::text(
                    ui,
                    "Nenhuma conversa agendada.",
                    theme::regular(12.0),
                    p.dim,
                );
            }
            for task in app
                .crm_tasks
                .clone()
                .into_iter()
                .filter(|task| task.kind == "conversation")
            {
                task_row(app, ui, &task, now, &mut draft);
                ui.add_space(6.0);
            }
            ui.add_space(8.0);
            theme::text(ui, "Follow-ups", theme::semibold(14.0), p.text);
            if followups.is_empty() {
                theme::text(
                    ui,
                    "Nenhum follow-up pendente.",
                    theme::regular(12.0),
                    p.dim,
                );
            }
            for followup in followups {
                followup_row(app, ui, &followup, now);
                ui.add_space(6.0);
            }
        });
    ui.data_mut(|d| d.insert_temp(draft_id, draft));
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
    let p = app.palette;
    ui.add_space(4.0);
    theme::text(
        ui,
        &crate::util::long_date(app.locale, date),
        theme::semibold(15.0),
        p.text,
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
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .max_height(470.0)
        .show(ui, |ui| {
            if tasks.is_empty() && followups.is_empty() {
                ui.add_space(28.0);
                ui.vertical_centered(|ui| {
                    theme::icon(ui, Icon::CircleCheck, 30.0, p.accent);
                    theme::text(
                        ui,
                        "Nenhum compromisso neste dia",
                        theme::semibold(14.0),
                        p.text,
                    );
                    theme::text(
                        ui,
                        "Use a aba Tarefas para criar um item com data e hora.",
                        theme::regular(12.0),
                        p.dim,
                    );
                });
            }
            for task in tasks {
                task_row(app, ui, &task, now, draft);
                ui.add_space(6.0);
            }
            for item in followups {
                followup_row(app, ui, &item, now);
                ui.add_space(6.0);
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
    let p = app.palette;
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
        if ui.button("‹").on_hover_text("Mês anterior").clicked() {
            month = crate::util::month_step(month, -1);
            selected = month;
        }
        theme::text(
            ui,
            &crate::util::month_heading(app.locale, month),
            theme::semibold(15.0),
            p.text,
        );
        if ui.button("›").on_hover_text("Próximo mês").clicked() {
            month = crate::util::month_step(month, 1);
            selected = month;
        }
        if ui.small_button("Hoje").clicked() {
            selected = crate::util::today();
            month = selected.first_of_month();
        }
    });
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
                    [cell_width, 18.0],
                    egui::Label::new(egui::RichText::new(heading).color(p.dim).strong()),
                );
            }
            ui.end_row();
            let first = month.first_of_month();
            let lead = usize::try_from(first.weekday().to_monday_zero_offset()).unwrap_or(0);
            let days = first.days_in_month() as usize;
            let cells = ((lead + days + 6) / 7) * 7;
            for index in 0..cells {
                if index < lead || index >= lead + days {
                    ui.add_sized([cell_width, 64.0], egui::Label::new(""));
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
                        let label = if count == 0 {
                            day.to_string()
                        } else {
                            format!("{day}\n{count} item{}", if count == 1 { "" } else { "s" })
                        };
                        let button = egui::Button::new(label).selected(selected == date);
                        if ui.add_sized([cell_width, 64.0], button).clicked() {
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
    ui.separator();
    show_day(app, ui, selected, now, draft);
}

fn task_row(app: &mut App, ui: &mut egui::Ui, task: &CrmTask, now: i64, draft: &mut TaskDraft) {
    let p = app.palette;
    let done = task.status == "done";
    let late = !done && task.due_at.is_some_and(|d| d <= now);
    let contact = task
        .chat_id
        .as_ref()
        .and_then(|id| app.chat(id))
        .map(|c| app.chat_title(c));
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(
            1.0,
            if late { p.danger } else { p.surface_hover },
        ))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::text(
                        ui,
                        &task.title,
                        theme::semibold(13.0),
                        if done { p.dim } else { p.text },
                    );
                    if !task.description.is_empty() {
                        theme::text(ui, &task.description, theme::regular(11.0), p.secondary);
                    }
                    let mut meta = if task.kind == "conversation" {
                        format!("Conversa • {} min", task.duration_minutes.unwrap_or(30))
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
                    if let Some(d) = task.due_at {
                        meta.push_str(&format!(" • {}", format_when(d)));
                    }
                    if let Some(c) = contact {
                        meta.push_str(&format!(" • {c}"));
                    }
                    theme::text(
                        ui,
                        &meta,
                        theme::regular(11.0),
                        if late { p.danger } else { p.dim },
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Excluir").clicked() {
                        app.actions.push(Action::DeleteCrmTask(task.id.clone()));
                    }
                    if ui.small_button("Editar").clicked() {
                        draft.id = Some(task.id.clone());
                        draft.title = task.title.clone();
                        draft.description = task.description.clone();
                        draft.priority = task.priority.clone();
                        draft.kind = task.kind.clone();
                        draft.duration_minutes = task.duration_minutes.unwrap_or(30);
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
                    if ui
                        .small_button(if done { "Reabrir" } else { "Concluir" })
                        .clicked()
                    {
                        let mut x = task.clone();
                        x.status = if done { "todo" } else { "done" }.into();
                        x.completed_at = if done { None } else { Some(now) };
                        x.updated_at = now;
                        app.actions.push(Action::SaveCrmTask(x));
                    }
                    if let Some(chat) = &task.chat_id {
                        if ui.small_button("Conversa").clicked() {
                            app.actions.push(Action::CloseDialog);
                            app.actions.push(Action::OpenChat(chat.clone()));
                        }
                    }
                });
            });
        });
}

fn followup_row(app: &mut App, ui: &mut egui::Ui, item: &CrmFollowup, now: i64) {
    let p = app.palette;
    let late = item.remind_at <= now;
    let contact = app
        .chat(&item.chat_id)
        .map(|c| app.chat_title(c))
        .unwrap_or_else(|| item.chat_id.trim_end_matches("@s.whatsapp.net").into());
    Frame::new()
        .fill(p.surface)
        .stroke(Stroke::new(
            1.0,
            if late { p.danger } else { p.surface_hover },
        ))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::text(ui, &item.title, theme::semibold(13.0), p.text);
                    theme::text(
                        ui,
                        &format!("{contact} • {}", format_when(item.remind_at)),
                        theme::regular(11.0),
                        if late { p.danger } else { p.dim },
                    );
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Concluir").clicked() {
                        app.actions
                            .push(Action::CompleteCrmFollowup(item.id.clone()));
                    }
                    if ui.small_button("+1 dia").clicked() {
                        app.actions.push(Action::SnoozeCrmFollowup {
                            id: item.id.clone(),
                            until: now + 86_400,
                        });
                    }
                    if ui.small_button("Abrir conversa").clicked() {
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
fn new_task_id() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task-{n}")
}
