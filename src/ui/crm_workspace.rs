//! Account-scoped CRM workspace. Projects reference chats by id and never copy messages.

use crate::app::App;
use crate::model::{Action, CrmProject, CrmTask, Page};
use crate::theme::{self, Icon};
use egui::{Align, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, vec2};
use jiff::civil::Date;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum WorkspaceView {
    #[default]
    Overview,
    Projects,
    Tasks,
    Agenda,
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

#[derive(Clone)]
struct ProjectEdit {
    open: bool,
    name: String,
    description: String,
    kind: String,
    has_due: bool,
    due_date: Date,
    due_hour: u8,
    due_minute: u8,
}

impl ProjectEdit {
    fn from_project(project: &CrmProject) -> Self {
        Self {
            open: false,
            name: project.name.clone(),
            description: project.description.clone(),
            kind: project.kind.clone(),
            has_due: project.due_at.is_some(),
            due_date: project
                .due_at
                .and_then(crate::util::local_datetime)
                .map(|x| x.0)
                .unwrap_or_else(|| {
                    crate::util::today()
                        .tomorrow()
                        .unwrap_or_else(|_| crate::util::today())
                }),
            due_hour: project
                .due_at
                .and_then(crate::util::local_datetime)
                .map(|x| x.1)
                .unwrap_or(18),
            due_minute: project
                .due_at
                .and_then(crate::util::local_datetime)
                .map(|x| x.2)
                .unwrap_or(0),
        }
    }
}

#[derive(Clone)]
struct ConversationDraft {
    title: String,
    date: Date,
    hour: u8,
    minute: u8,
    duration_minutes: u16,
}

impl Default for ConversationDraft {
    fn default() -> Self {
        Self {
            title: "Conversa com cliente".into(),
            date: crate::util::today()
                .tomorrow()
                .unwrap_or_else(|_| crate::util::today()),
            hour: 9,
            minute: 0,
            duration_minutes: 30,
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

    let available = ui.available_size();
    let gap = ui.spacing().item_spacing.x;
    let (sidebar_width, content_width) = workspace_widths(available.x, gap);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2(sidebar_width, available.y),
            Layout::top_down(Align::Min),
            |ui| {
                Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.outline))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width((sidebar_width - 28.0).max(140.0));
                        ui.set_min_height((available.y - 28.0).max(320.0));
                        ui.horizontal(|ui| {
                            theme::icon(ui, Icon::ListChecks, 22.0, palette.accent);
                            ui.label(RichText::new("ZapFast CRM").size(18.0).strong());
                        });
                        ui.label(
                            RichText::new("Central de relacionamento")
                                .small()
                                .color(palette.dim),
                        );
                        ui.add_space(20.0);
                        nav(
                            ui,
                            &mut view,
                            WorkspaceView::Overview,
                            Icon::Clock,
                            "Meu dia",
                        );
                        nav(
                            ui,
                            &mut view,
                            WorkspaceView::Projects,
                            Icon::Users,
                            "Projetos",
                        );
                        nav(
                            ui,
                            &mut view,
                            WorkspaceView::Tasks,
                            Icon::ListChecks,
                            "Tarefas",
                        );
                        nav(
                            ui,
                            &mut view,
                            WorkspaceView::Agenda,
                            Icon::Calendar,
                            "Agenda",
                        );
                        nav(
                            ui,
                            &mut view,
                            WorkspaceView::Funnel,
                            Icon::Tag,
                            "Funil comercial",
                        );
                        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                            if ui
                                .add_sized([190.0, 36.0], egui::Button::new("← Conversas"))
                                .clicked()
                            {
                                app.actions.push(Action::Open(Page::Chats));
                            }
                            ui.label(
                                RichText::new("Dados isolados nesta conta")
                                    .small()
                                    .color(palette.dim),
                            );
                        });
                    });
            },
        );

        ui.allocate_ui_with_layout(
            vec2(content_width, available.y),
            Layout::top_down(Align::Min),
            |ui| {
                Frame::new()
                    .fill(palette.window)
                    .inner_margin(Margin::symmetric(24, 18))
                    .show(ui, |ui| {
                        ui.set_width((content_width - 48.0).max(272.0));
                        ui.set_min_height((available.y - 36.0).max(320.0));
                        let (title, subtitle) = match view {
                            WorkspaceView::Overview => ("Meu dia", "Prioridades e próximos passos"),
                            WorkspaceView::Projects => {
                                ("Projetos", "Clientes, entregas e conversas vinculadas")
                            }
                            WorkspaceView::Tasks => ("Tarefas", "Organize e acompanhe o trabalho"),
                            WorkspaceView::Agenda => ("Agenda", "Prazos e follow-ups da conta"),
                            WorkspaceView::Funnel => {
                                ("Funil comercial", "Negócios e etapas de venda")
                            }
                        };
                        ui.label(RichText::new(title).size(24.0).strong().color(palette.text));
                        ui.label(RichText::new(subtitle).size(12.0).color(palette.dim));
                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);
                        ScrollArea::vertical().auto_shrink([false, false]).show(
                            ui,
                            |ui| match view {
                                WorkspaceView::Overview => overview(app, ui, &mut view),
                                WorkspaceView::Projects => {
                                    projects(app, ui, &mut draft, &mut selected)
                                }
                                WorkspaceView::Tasks => super::crm_tasks::show_workspace(app, ui),
                                WorkspaceView::Agenda => {
                                    super::crm_tasks::show_agenda_workspace(app, ui)
                                }
                                WorkspaceView::Funnel => funnel(app, ui),
                            },
                        );
                    });
            },
        );
    });

    ui.data_mut(|d| {
        d.insert_temp(view_id, view);
        d.insert_temp(draft_id, draft);
        d.insert_temp(selected_id, selected);
    });
}

fn workspace_widths(available: f32, gap: f32) -> (f32, f32) {
    let sidebar = 218.0_f32.min((available * 0.32).max(170.0));
    let content = (available - sidebar - gap).max(320.0);
    (sidebar, content)
}

fn nav(
    ui: &mut egui::Ui,
    view: &mut WorkspaceView,
    target: WorkspaceView,
    icon: Icon,
    label: &str,
) {
    let selected = *view == target;
    let response = ui.add_sized([190.0, 40.0], egui::Button::new(label).selected(selected));
    let _ = icon;
    if response.clicked() {
        *view = target;
    }
    ui.add_space(3.0);
}

fn funnel(app: &mut App, ui: &mut egui::Ui) {
    empty_panel(
        ui,
        Icon::Tag,
        "Acompanhe seus negócios por etapa",
        "Abra o quadro para mover contatos, registrar valores e consultar métricas.",
    );
    ui.vertical_centered(|ui| {
        if ui
            .add_sized([220.0, 38.0], egui::Button::new("Abrir quadro comercial"))
            .clicked()
        {
            app.actions
                .push(Action::ShowDialog(crate::model::Dialog::Kanban));
        }
    });
}

fn overview(app: &mut App, ui: &mut egui::Ui, view: &mut WorkspaceView) {
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
    let metric_width = ((ui.available_width() - 36.0) / 4.0).max(145.0);
    ui.horizontal_wrapped(|ui| {
        metric(
            ui,
            Icon::Users,
            "Projetos ativos",
            active_projects,
            metric_width,
        );
        metric(
            ui,
            Icon::ListChecks,
            "Tarefas abertas",
            open_tasks,
            metric_width,
        );
        metric(ui, Icon::CircleAlert, "Atrasadas", late, metric_width);
        metric(ui, Icon::Bell, "Follow-ups", followups, metric_width);
    });
    ui.add_space(20.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Próximas ações").size(17.0).strong());
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.small_button("Ver todas as tarefas →").clicked() {
                *view = WorkspaceView::Tasks;
            }
        });
    });
    let mut tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.status != "done")
        .collect();
    tasks.sort_by_key(|t| t.due_at.unwrap_or(i64::MAX));
    if tasks.is_empty() {
        empty_panel(
            ui,
            Icon::CircleCheck,
            "Tudo em dia",
            "Crie uma tarefa ou um projeto para começar a organizar o trabalho.",
        );
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

fn metric(ui: &mut egui::Ui, icon: Icon, title: &str, value: usize, width: f32) {
    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.horizontal(|ui| {
                theme::icon(ui, icon, 18.0, ui.visuals().hyperlink_color);
                ui.label(RichText::new(title).small());
            });
            ui.label(RichText::new(value.to_string()).size(26.0).strong());
        });
}

fn empty_panel(ui: &mut egui::Ui, icon: Icon, title: &str, subtitle: &str) {
    Frame::new()
        .fill(ui.visuals().faint_bg_color)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(24))
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 48.0).max(320.0));
            ui.vertical_centered(|ui| {
                theme::icon(ui, icon, 30.0, ui.visuals().hyperlink_color);
                ui.add_space(6.0);
                ui.label(RichText::new(title).size(16.0).strong());
                ui.label(RichText::new(subtitle).color(ui.visuals().weak_text_color()));
            });
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
    let account_id = app.id.as_str().to_owned();
    let form_id = ui.id().with(("crm-project-form-open", &account_id));
    let search_id = ui.id().with(("crm-project-search", &account_id));
    let status_id = ui.id().with(("crm-project-status-filter", &account_id));
    let mut show_form = ui.data_mut(|d| {
        d.get_temp::<bool>(form_id)
            .unwrap_or(app.crm_projects.is_empty())
    });
    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
    let mut status = ui.data_mut(|d| {
        d.get_temp::<String>(status_id)
            .unwrap_or_else(|| "all".into())
    });
    ui.horizontal(|ui| {
        ui.add_sized(
            [320.0, 34.0],
            egui::TextEdit::singleline(&mut search).hint_text("Buscar projeto…"),
        );
        egui::ComboBox::from_id_salt(("project-filter", &account_id))
            .selected_text(match status.as_str() {
                "active" => "Ativos",
                "paused" => "Pausados",
                "completed" => "Concluídos",
                _ => "Todos",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut status, "all".into(), "Todos");
                ui.selectable_value(&mut status, "active".into(), "Ativos");
                ui.selectable_value(&mut status, "paused".into(), "Pausados");
                ui.selectable_value(&mut status, "completed".into(), "Concluídos");
            });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui
                .add_sized(
                    [130.0, 34.0],
                    egui::Button::new(if show_form {
                        "Fechar"
                    } else {
                        "+ Novo projeto"
                    }),
                )
                .clicked()
            {
                show_form = !show_form;
            }
        });
    });
    ui.add_space(10.0);
    if show_form {
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
                    egui::TextEdit::multiline(&mut draft.description)
                        .hint_text("Descrição e objetivo"),
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
                    show_form = false;
                }
            });
    }
    ui.data_mut(|d| {
        d.insert_temp(form_id, show_form);
        d.insert_temp(search_id, search.clone());
        d.insert_temp(status_id, status.clone());
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
        let needle = search.trim().to_lowercase();
        for project in app.crm_projects.clone().into_iter().filter(|project| {
            (status == "all" || project.status == status)
                && (needle.is_empty()
                    || project.name.to_lowercase().contains(&needle)
                    || project.description.to_lowercase().contains(&needle))
        }) {
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
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&project.name).strong().size(15.0));
                                ui.label(
                                    RichText::new(project_status(&project.status))
                                        .small()
                                        .color(app.palette.accent),
                                );
                            });
                            ui.label(format!(
                                "{} conversa(s) • {done}/{total} tarefas concluídas",
                                project.chat_ids.len()
                            ));
                            if let Some((date, _, _)) =
                                project.due_at.and_then(crate::util::local_datetime)
                            {
                                ui.label(
                                    RichText::new(format!(
                                        "Prazo: {}",
                                        crate::util::long_date(app.locale, date)
                                    ))
                                    .small()
                                    .color(app.palette.dim),
                                );
                            }
                            let progress = if total == 0 {
                                0.0
                            } else {
                                done as f32 / total as f32
                            };
                            ui.add_sized([260.0, 8.0], egui::ProgressBar::new(progress));
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
    let edit_id = ui.id().with(("crm-edit-project", &project.id));
    let mut confirm_delete = ui.data_mut(|d| d.get_temp::<bool>(confirm_id).unwrap_or(false));
    let mut edit = ui.data_mut(|d| {
        d.get_temp::<ProjectEdit>(edit_id)
            .unwrap_or_else(|| ProjectEdit::from_project(&project))
    });
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
            if ui
                .small_button(if edit.open {
                    "Fechar edição"
                } else {
                    "Editar projeto"
                })
                .clicked()
            {
                edit.open = !edit.open;
            }
        });
    });
    ui.data_mut(|d| d.insert_temp(confirm_id, confirm_delete));
    if edit.open {
        let due_picker_id = format!("project-due-{}", project.id);
        Frame::new()
            .fill(app.palette.surface)
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add_sized(
                        [300.0, 30.0],
                        egui::TextEdit::singleline(&mut edit.name).hint_text("Nome do projeto"),
                    );
                    egui::ComboBox::from_id_salt(("edit-project-kind", &project.id))
                        .selected_text(if edit.kind == "internal" {
                            "Interno"
                        } else {
                            "Cliente"
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut edit.kind, "client".into(), "Cliente");
                            ui.selectable_value(&mut edit.kind, "internal".into(), "Interno");
                        });
                    ui.checkbox(&mut edit.has_due, "Definir prazo");
                });
                ui.add_sized(
                    [ui.available_width(), 54.0],
                    egui::TextEdit::multiline(&mut edit.description)
                        .hint_text("Descrição e objetivo"),
                );
                if edit.has_due {
                    super::dialogs::date_time_picker(
                        ui,
                        app.locale,
                        &app.palette,
                        &due_picker_id,
                        &mut edit.due_date,
                        &mut edit.due_hour,
                        &mut edit.due_minute,
                    );
                }
                if ui
                    .add_enabled(
                        !edit.name.trim().is_empty(),
                        egui::Button::new("Salvar alterações"),
                    )
                    .clicked()
                {
                    project.name = edit.name.trim().into();
                    project.description = edit.description.trim().into();
                    project.kind = edit.kind.clone();
                    project.due_at = edit.has_due.then(|| {
                        crate::util::to_unix_seconds(edit.due_date, edit.due_hour, edit.due_minute)
                            .unwrap_or(crate::util::now())
                    });
                    project.updated_at = crate::util::now();
                    app.actions.push(Action::SaveCrmProject(project.clone()));
                    edit.open = false;
                }
            });
        ui.add_space(8.0);
    }
    ui.data_mut(|d| d.insert_temp(edit_id, edit));
    if !project.description.is_empty() {
        ui.label(&project.description);
    }
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label("Situação:");
        let before = project.status.clone();
        egui::ComboBox::from_id_salt(("project-status", &project.id))
            .selected_text(project_status(&project.status))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut project.status, "active".into(), "Ativo");
                ui.selectable_value(&mut project.status, "paused".into(), "Pausado");
                ui.selectable_value(&mut project.status, "completed".into(), "Concluído");
                ui.selectable_value(&mut project.status, "archived".into(), "Arquivado");
            });
        if project.status != before {
            project.updated_at = crate::util::now();
            app.actions.push(Action::SaveCrmProject(project.clone()));
        }
    });
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
    if !project.chat_ids.is_empty() {
        ui.add_space(8.0);
        ui.label(RichText::new("Conversas vinculadas").strong());
        for chat_id in project.chat_ids.clone() {
            let title = app
                .chat(&chat_id)
                .map(|chat| app.chat_title(chat))
                .unwrap_or_else(|| chat_id.clone());
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::MessageCircle, 15.0, app.palette.dim);
                ui.label(title);
                if ui.small_button("Abrir conversa").clicked() {
                    app.actions.push(Action::OpenChat(chat_id.clone()));
                }
            });
        }
    }
    ui.separator();
    ui.heading("Agenda do projeto");
    ui.label("Marque uma conversa e abra o chat no horário combinado. Nenhuma mensagem é enviada automaticamente.");
    let conversation_id = ui.id().with(("project-conversation", &project.id));
    let mut conversation = ui.data_mut(|d| {
        d.get_temp::<ConversationDraft>(conversation_id)
            .unwrap_or_default()
    });
    Frame::new()
        .fill(app.palette.surface)
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.add_sized(
                    [280.0, 28.0],
                    egui::TextEdit::singleline(&mut conversation.title),
                );
                egui::ComboBox::from_id_salt(("project-conversation-duration", &project.id))
                    .selected_text(format!("{} min", conversation.duration_minutes))
                    .show_ui(ui, |ui| {
                        for minutes in [15, 30, 45, 60, 90, 120] {
                            ui.selectable_value(
                                &mut conversation.duration_minutes,
                                minutes,
                                format!("{minutes} min"),
                            );
                        }
                    });
            });
            super::dialogs::date_time_picker(
                ui,
                app.locale,
                &app.palette,
                "project-conversation-date",
                &mut conversation.date,
                &mut conversation.hour,
                &mut conversation.minute,
            );
            if ui
                .add_enabled(
                    !conversation.title.trim().is_empty() && !project.chat_ids.is_empty(),
                    egui::Button::new("Agendar conversa"),
                )
                .on_disabled_hover_text("Vincule uma conversa ou grupo ao projeto primeiro.")
                .clicked()
            {
                let now = crate::util::now();
                let nonce = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                app.actions.push(Action::SaveCrmTask(CrmTask {
                    id: format!("conversation-{nonce}"),
                    chat_id: project.chat_ids.first().cloned(),
                    project_id: Some(project.id.clone()),
                    title: conversation.title.trim().into(),
                    description: String::new(),
                    status: "todo".into(),
                    priority: "normal".into(),
                    kind: "conversation".into(),
                    duration_minutes: Some(conversation.duration_minutes),
                    due_at: crate::util::to_unix_seconds(
                        conversation.date,
                        conversation.hour,
                        conversation.minute,
                    ),
                    completed_at: None,
                    created_at: now,
                    updated_at: now,
                }));
                conversation = ConversationDraft::default();
            }
        });
    ui.data_mut(|d| d.insert_temp(conversation_id, conversation));
    for task in app.crm_tasks.clone().into_iter().filter(|task| {
        task.project_id.as_deref() == Some(&project.id) && task.kind == "conversation"
    }) {
        task_assignment_row(app, ui, task, None);
    }
    ui.separator();
    ui.heading("Tarefas do projeto");
    let new_task_id = ui.id().with(("new-project-task", &project.id));
    let mut new_task = ui.data_mut(|d| d.get_temp::<String>(new_task_id).unwrap_or_default());
    ui.horizontal(|ui| {
        ui.add_sized(
            [360.0, 30.0],
            egui::TextEdit::singleline(&mut new_task).hint_text("Nova tarefa deste projeto…"),
        );
        if ui
            .add_enabled(!new_task.trim().is_empty(), egui::Button::new("Adicionar"))
            .clicked()
        {
            let now = crate::util::now();
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            app.actions.push(Action::SaveCrmTask(CrmTask {
                id: format!("task-{nonce}"),
                chat_id: project.chat_ids.first().cloned(),
                project_id: Some(project.id.clone()),
                title: new_task.trim().into(),
                description: String::new(),
                status: "todo".into(),
                priority: "normal".into(),
                kind: "task".into(),
                duration_minutes: None,
                due_at: None,
                completed_at: None,
                created_at: now,
                updated_at: now,
            }));
            new_task.clear();
        }
    });
    ui.data_mut(|d| d.insert_temp(new_task_id, new_task));
    let mut any = false;
    for task in app.crm_tasks.clone() {
        if task.project_id.as_deref() == Some(&project.id) && task.kind != "conversation" {
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

fn project_status(status: &str) -> &str {
    match status {
        "paused" => "Pausado",
        "completed" => "Concluído",
        "archived" => "Arquivado",
        _ => "Ativo",
    }
}

#[cfg(test)]
mod tests {
    use super::workspace_widths;

    #[test]
    fn crm_workspace_reserves_visible_content_beside_navigation() {
        let (sidebar, content) = workspace_widths(1920.0, 8.0);
        assert_eq!(sidebar, 218.0);
        assert_eq!(content, 1694.0);

        let (sidebar, content) = workspace_widths(720.0, 8.0);
        assert!(sidebar >= 170.0);
        assert!(content >= 320.0);
        assert!(sidebar + content + 8.0 <= 720.0);
    }
}

fn task_assignment_row(app: &mut App, ui: &mut egui::Ui, mut task: CrmTask, assign: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(if task.status == "done" { "✓" } else { "□" });
        ui.vertical(|ui| {
            ui.label(RichText::new(&task.title).strong());
            if let Some((date, hour, minute)) = task.due_at.and_then(crate::util::local_datetime) {
                let duration = task
                    .duration_minutes
                    .map(|minutes| format!(" • {minutes} min"))
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!(
                        "{:02}/{:02}/{} {:02}:{:02}{duration}",
                        date.day(),
                        date.month(),
                        date.year(),
                        hour,
                        minute
                    ))
                    .small()
                    .color(app.palette.dim),
                );
            }
        });
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
