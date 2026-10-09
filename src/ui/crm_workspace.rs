//! Account-scoped CRM workspace. Projects reference chats by id and never copy messages.

use crate::app::App;
use crate::model::{Action, CrmProject, CrmTask, Page};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum WorkspaceView {
    #[default]
    Overview,
    Projects,
    Tasks,
    Funnel,
}

#[derive(Clone)]
struct ProjectDraft {
    name: String,
    description: String,
    kind: String,
    link_current_chat: bool,
}

impl Default for ProjectDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            kind: "client".into(),
            link_current_chat: true,
        }
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let account_id = app.id.as_str().to_owned();
    let view_id = ui.id().with(("crm-workspace-view", &account_id));
    let draft_id = ui.id().with(("crm-project-draft", &account_id));
    let selected_id = ui.id().with(("crm-selected-project", &account_id));
    let mut view = ui.data_mut(|d| d.get_temp::<WorkspaceView>(view_id).unwrap_or_default());
    let mut draft = ui.data_mut(|d| d.get_temp::<ProjectDraft>(draft_id).unwrap_or_default());
    let mut selected = ui.data_mut(|d| d.get_temp::<Option<String>>(selected_id).flatten());

    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new("CRM").size(22.0).strong().color(palette.text));
            ui.label(
                RichText::new("Projetos, tarefas e relacionamento")
                    .size(12.0)
                    .color(palette.dim),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Voltar às conversas").clicked() {
                app.actions.push(Action::Open(Page::Chats));
            }
        });
    });
    ui.add_space(10.0);
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut view, WorkspaceView::Overview, "Meu dia");
        ui.selectable_value(&mut view, WorkspaceView::Projects, "Projetos");
        ui.selectable_value(&mut view, WorkspaceView::Tasks, "Tarefas");
        ui.selectable_value(&mut view, WorkspaceView::Funnel, "Funil comercial");
    });
    ui.separator();

    match view {
        WorkspaceView::Overview => overview(app, ui),
        WorkspaceView::Projects => projects(app, ui, &mut draft, &mut selected),
        WorkspaceView::Tasks => super::crm_tasks::show_workspace(app, ui),
        WorkspaceView::Funnel => {
            ui.add_space(24.0);
            ui.heading("Funil comercial");
            ui.label("O funil continua usando os negócios desta conta.");
            if ui.button("Abrir quadro do funil").clicked() {
                app.actions
                    .push(Action::ShowDialog(crate::model::Dialog::Kanban));
            }
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(view_id, view);
        d.insert_temp(draft_id, draft);
        d.insert_temp(selected_id, selected);
    });
}

fn overview(app: &mut App, ui: &mut egui::Ui) {
    let now = crate::util::now();
    let active_projects = app
        .crm_projects
        .iter()
        .filter(|p| p.status == "active")
        .count();
    let open_tasks = app.crm_tasks.iter().filter(|t| t.status != "done").count();
    let late = app
        .crm_tasks
        .iter()
        .filter(|t| t.status != "done" && t.due_at.is_some_and(|x| x < now))
        .count();
    let followups = app.crm_followups.iter().filter(|f| !f.done).count();
    ui.horizontal_wrapped(|ui| {
        metric(ui, "Projetos ativos", active_projects);
        metric(ui, "Tarefas abertas", open_tasks);
        metric(ui, "Atrasadas", late);
        metric(ui, "Follow-ups", followups);
    });
    ui.add_space(18.0);
    ui.heading("Próximas ações");
    let mut tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.status != "done")
        .collect();
    tasks.sort_by_key(|t| t.due_at.unwrap_or(i64::MAX));
    if tasks.is_empty() {
        ui.label("Nenhuma tarefa aberta nesta conta.");
    }
    for task in tasks.into_iter().take(8) {
        let project = task
            .project_id
            .as_ref()
            .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id));
        ui.horizontal(|ui| {
            ui.label("□");
            ui.label(RichText::new(&task.title).strong());
            if let Some(project) = project {
                ui.label(RichText::new(&project.name).small().weak());
            }
        });
    }
}

fn metric(ui: &mut egui::Ui, title: &str, value: usize) {
    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.label(RichText::new(value.to_string()).size(22.0).strong());
            ui.label(RichText::new(title).small());
        });
}

fn projects(
    app: &mut App,
    ui: &mut egui::Ui,
    draft: &mut ProjectDraft,
    selected: &mut Option<String>,
) {
    if let Some(id) = selected.clone() {
        if let Some(project) = app.crm_projects.iter().find(|p| p.id == id).cloned() {
            project_detail(app, ui, project, selected);
            return;
        }
        *selected = None;
    }

    let current = app.current_chat().cloned();
    Frame::new()
        .fill(app.palette.surface)
        .stroke(Stroke::new(1.0, app.palette.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.label(RichText::new("Novo projeto").strong());
            if draft.name.is_empty() {
                if let Some(chat) = &current {
                    draft.name = app.chat_title(chat);
                }
            }
            ui.horizontal_wrapped(|ui| {
                ui.add_sized(
                    [280.0, 28.0],
                    egui::TextEdit::singleline(&mut draft.name).hint_text("Nome do projeto"),
                );
                egui::ComboBox::from_id_salt("crm-project-kind")
                    .selected_text(if draft.kind == "internal" {
                        "Interno"
                    } else {
                        "Cliente"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.kind, "client".into(), "Cliente");
                        ui.selectable_value(&mut draft.kind, "internal".into(), "Interno");
                    });
                if current.is_some() {
                    ui.checkbox(
                        &mut draft.link_current_chat,
                        "Vincular conversa/grupo aberto",
                    );
                }
            });
            ui.add_sized(
                [ui.available_width(), 42.0],
                egui::TextEdit::multiline(&mut draft.description).hint_text("Descrição e objetivo"),
            );
            if ui
                .add_enabled(
                    !draft.name.trim().is_empty(),
                    egui::Button::new("Criar projeto"),
                )
                .clicked()
            {
                let now = crate::util::now();
                let nonce = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let id = format!("project-{nonce}");
                let chat_ids = if draft.link_current_chat {
                    current
                        .as_ref()
                        .map(|c| vec![c.id.clone()])
                        .unwrap_or_default()
                } else {
                    Vec::new()
                };
                app.actions.push(Action::SaveCrmProject(CrmProject {
                    id,
                    name: draft.name.trim().into(),
                    description: draft.description.trim().into(),
                    kind: draft.kind.clone(),
                    status: "active".into(),
                    color: "#00a884".into(),
                    icon: "briefcase".into(),
                    start_at: Some(now),
                    due_at: None,
                    created_at: now,
                    updated_at: now,
                    chat_ids,
                }));
                *draft = ProjectDraft::default();
            }
        });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.heading("Projetos");
        ui.label(format!("{} nesta conta", app.crm_projects.len()));
    });
    ScrollArea::vertical().show(ui, |ui| {
        if app.crm_projects.is_empty() {
            ui.label("Crie o primeiro projeto ou abra um grupo e vincule-o acima.");
        }
        for project in app.crm_projects.clone() {
            let total = app
                .crm_tasks
                .iter()
                .filter(|t| t.project_id.as_deref() == Some(&project.id))
                .count();
            let done = app
                .crm_tasks
                .iter()
                .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.status == "done")
                .count();
            let linked_chat = project
                .chat_ids
                .first()
                .and_then(|id| app.chat(id))
                .cloned();
            let linked_title = linked_chat.as_ref().map(|chat| app.chat_title(chat));
            Frame::new()
                .fill(app.palette.surface)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if let Some(chat) = &linked_chat {
                            let picture = app.avatar(&chat.id);
                            super::widgets::avatar(
                                ui,
                                &app.palette,
                                linked_title.as_deref().unwrap_or(&chat.name),
                                &chat.id,
                                36.0,
                                picture.as_deref(),
                            );
                        }
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&project.name).strong().size(15.0));
                            ui.label(format!(
                                "{} conversa(s) • {done}/{total} tarefas concluídas",
                                project.chat_ids.len()
                            ));
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Abrir").clicked() {
                                *selected = Some(project.id.clone());
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    });
}

fn project_detail(
    app: &mut App,
    ui: &mut egui::Ui,
    mut project: CrmProject,
    selected: &mut Option<String>,
) {
    let confirm_id = ui.id().with(("crm-delete-project", &project.id));
    let mut confirm_delete = ui.data_mut(|d| d.get_temp::<bool>(confirm_id).unwrap_or(false));
    ui.horizontal(|ui| {
        if ui.button("← Projetos").clicked() {
            *selected = None;
        }
        ui.heading(&project.name);
        ui.label(if project.status == "active" {
            "Ativo"
        } else {
            &project.status
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if confirm_delete {
                if ui.button("Confirmar exclusão").clicked() {
                    app.actions
                        .push(Action::DeleteCrmProject(project.id.clone()));
                    *selected = None;
                    confirm_delete = false;
                }
                if ui.small_button("Cancelar").clicked() {
                    confirm_delete = false;
                }
            } else if ui.small_button("Excluir projeto").clicked() {
                confirm_delete = true;
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(confirm_id, confirm_delete));
    if !project.description.is_empty() {
        ui.label(&project.description);
    }
    ui.add_space(8.0);
    if let Some(chat) = app.current_chat().cloned() {
        let linked = project.chat_ids.contains(&chat.id);
        let label = if linked {
            "Desvincular conversa aberta"
        } else {
            "Vincular conversa/grupo aberto"
        };
        if ui.button(label).clicked() {
            if linked {
                project.chat_ids.retain(|id| id != &chat.id);
            } else {
                project.chat_ids.push(chat.id);
            }
            project.updated_at = crate::util::now();
            app.actions.push(Action::SaveCrmProject(project.clone()));
        }
    }
    ui.separator();
    ui.heading("Tarefas do projeto");
    let mut any = false;
    for task in app.crm_tasks.clone() {
        if task.project_id.as_deref() == Some(&project.id) {
            any = true;
            task_assignment_row(app, ui, task, None);
        }
    }
    if !any {
        ui.label("Nenhuma tarefa vinculada.");
    }
    ui.add_space(10.0);
    ui.label(RichText::new("Vincular tarefa existente").strong());
    for task in app
        .crm_tasks
        .clone()
        .into_iter()
        .filter(|t| t.project_id.is_none())
    {
        task_assignment_row(app, ui, task, Some(&project.id));
    }
}

fn task_assignment_row(app: &mut App, ui: &mut egui::Ui, mut task: CrmTask, assign: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(if task.status == "done" { "✓" } else { "□" });
        ui.label(&task.title);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if assign.is_some() {
                "Vincular"
            } else {
                "Remover do projeto"
            };
            if ui.small_button(label).clicked() {
                task.project_id = assign.map(str::to_owned);
                task.updated_at = crate::util::now();
                app.actions.push(Action::SaveCrmTask(task));
            }
        });
    });
}
