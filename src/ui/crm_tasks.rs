//! Account-scoped CRM task and follow-up inbox.

use crate::app::App;
use crate::model::{Action, CrmFollowup, CrmTask};
use crate::theme::{self, Icon};
use egui::{Align, CornerRadius, Frame, Layout, Margin, Stroke};

#[derive(Clone)]
struct TaskDraft {
    id: Option<String>,
    title: String,
    description: String,
    priority: String,
    due_days: i64,
    link_chat: bool,
}
impl Default for TaskDraft {
    fn default() -> Self {
        Self {
            id: None,
            title: String::new(),
            description: String::new(),
            priority: "normal".into(),
            due_days: 1,
            link_chat: true,
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let p = app.palette;
    let now = crate::util::now();
    let draft_id = ui.id().with("crm-task-draft");
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
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        summary(ui, &p, "Atrasados", late, late > 0);
        summary(ui, &p, "Tarefas", open, false);
        summary(ui, &p, "Concluídas", done, false);
        summary(ui, &p, "Follow-ups", followups.len(), false);
    });
    ui.separator();

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
            ui.horizontal(|ui| {
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
                egui::ComboBox::from_id_salt("task-due")
                    .selected_text(due_label(draft.due_days))
                    .show_ui(ui, |ui| {
                        for (v, l) in [
                            (-1, "Sem prazo"),
                            (0, "Hoje"),
                            (1, "Amanhã"),
                            (7, "Em 7 dias"),
                        ] {
                            ui.selectable_value(&mut draft.due_days, v, l);
                        }
                    });
            });
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
                            project_id: old.as_ref().and_then(|x| x.project_id.clone()),
                            title: draft.title.trim().into(),
                            description: draft.description.trim().into(),
                            status: old
                                .as_ref()
                                .map_or_else(|| "todo".into(), |x| x.status.clone()),
                            priority: draft.priority.clone(),
                            due_at: (draft.due_days >= 0).then_some(now + draft.due_days * 86_400),
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
            theme::text(ui, "Tarefas", theme::semibold(14.0), p.text);
            if app.crm_tasks.is_empty() {
                theme::text(ui, "Nenhuma tarefa criada.", theme::regular(12.0), p.dim);
            }
            for task in app.crm_tasks.clone() {
                task_row(app, ui, &task, now, &mut draft);
                ui.add_space(6.0);
            }
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
                    let mut meta = priority_label(&task.priority).to_owned();
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
                        draft.due_days = task
                            .due_at
                            .map_or(-1, |d| ((d - now).max(0) + 86_399) / 86_400);
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
fn due_label(v: i64) -> &'static str {
    match v {
        -1 => "Sem prazo",
        0 => "Hoje",
        1 => "Amanhã",
        7 => "Em 7 dias",
        _ => "Prazo definido",
    }
}
fn new_task_id() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task-{n}")
}
