//! Account-scoped CRM workspace. Projects reference chats by id and never copy messages.

use crate::app::App;
use crate::model::{Action, CrmProject, CrmTask, Page};
use crate::theme::{self, Icon};
use egui::{
    Align, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Stroke, Vec2, vec2,
};
use jiff::civil::Date;

#[derive(Clone, Copy)]
pub struct CrmPalette {
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
}

impl CrmPalette {
    pub fn from_app(_palette: &crate::theme::Palette) -> Self {
        Self {
            window: Color32::from_rgb(0x08, 0x0c, 0x0f),
            panel: Color32::from_rgb(0x0c, 0x12, 0x16),
            surface: Color32::from_rgb(0x13, 0x1b, 0x21),
            surface_hover: Color32::from_rgb(0x1a, 0x25, 0x2e),
            outline: Color32::from_rgb(0x20, 0x2c, 0x36),
            text: Color32::from_rgb(0xf3, 0xf6, 0xf8),
            secondary: Color32::from_rgb(0xa1, 0xb0, 0xba),
            dim: Color32::from_rgb(0x6f, 0x80, 0x8c),
            accent: Color32::from_rgb(0x00, 0xe6, 0x99),
            on_accent: Color32::from_rgb(0x06, 0x18, 0x11),
            danger: Color32::from_rgb(0xff, 0x47, 0x57),
            warning: Color32::from_rgb(0xff, 0xb8, 0x1c),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum WorkspaceView {
    #[default]
    Overview,
    Projects,
    Tasks,
    Timeline,
    Calls,
    Agenda,
    Funnel,
}

#[derive(Clone)]
struct ProjectDraft {
    name: String,
    description: String,
    kind: String,
    link_current_chat: bool,
    assignee: String,
    client_name: String,
    client_phone: String,
    client_email: String,
    start_date: Date,
    start_hour: u8,
    start_minute: u8,
}

impl Default for ProjectDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            kind: "client".into(),
            link_current_chat: true,
            assignee: String::new(),
            client_name: String::new(),
            client_phone: String::new(),
            client_email: String::new(),
            start_date: crate::util::today(),
            start_hour: 9,
            start_minute: 0,
        }
    }
}

#[derive(Clone)]
struct ProjectEdit {
    open: bool,
    name: String,
    description: String,
    kind: String,
    assignee: String,
    client_name: String,
    client_phone: String,
    client_email: String,
    start_date: Date,
    start_hour: u8,
    start_minute: u8,
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
            assignee: project.assignee.clone(),
            client_name: project.client_name.clone(),
            client_phone: project.client_phone.clone(),
            client_email: project.client_email.clone(),
            start_date: project
                .start_at
                .and_then(crate::util::local_datetime)
                .map(|x| x.0)
                .unwrap_or_else(crate::util::today),
            start_hour: 9,
            start_minute: 0,
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
    let cp = CrmPalette::from_app(&app.palette);
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
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(14))
                    .show(ui, |ui| {
                        ui.set_width((sidebar_width - 28.0).max(140.0));
                        ui.set_min_height((available.y - 28.0).max(320.0));
                        ui.horizontal(|ui| {
                            theme::icon(ui, Icon::ListChecks, 24.0, cp.accent);
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new("ZapFast CRM")
                                    .size(19.0)
                                    .strong()
                                    .color(cp.text),
                            );
                        });
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("Central de Operações")
                                .size(12.0)
                                .color(cp.dim),
                        );
                        ui.add_space(14.0);
                        let cur = ui.cursor();
                        ui.painter().hline(
                            cur.left()..=(cur.left() + ui.available_width()),
                            cur.top(),
                            Stroke::new(1.0, cp.outline),
                        );
                        ui.add_space(14.0);
                        ui.label(RichText::new("OPERAÇÕES").size(10.5).strong().color(cp.dim));
                        ui.add_space(6.0);
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Overview,
                            Icon::Clock,
                            "Meu dia",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Tasks,
                            Icon::ListChecks,
                            "Tarefas",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Projects,
                            Icon::Users,
                            "Projetos",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Timeline,
                            Icon::Clock,
                            "Timeline",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Calls,
                            Icon::Phone,
                            "Calls de Venda",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Agenda,
                            Icon::Calendar,
                            "Agenda",
                            &cp,
                        );
                        nav_item(
                            ui,
                            &mut view,
                            WorkspaceView::Funnel,
                            Icon::Tag,
                            "Funil comercial",
                            &cp,
                        );
                        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::Lock, 13.0, cp.dim);
                                ui.label(
                                    RichText::new("Dados criptografados")
                                        .size(11.0)
                                        .color(cp.dim),
                                );
                            });
                            ui.add_space(6.0);
                            let back_btn = egui::Button::new(
                                RichText::new("← Voltar às Conversas")
                                    .size(13.0)
                                    .color(cp.secondary),
                            )
                            .fill(cp.surface)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(8))
                            .min_size(vec2(ui.available_width(), 38.0));
                            if ui.add(back_btn).clicked() {
                                app.actions.push(Action::Open(Page::Chats));
                            }
                        });
                    });
            },
        );

        ui.allocate_ui_with_layout(
            vec2(content_width, available.y),
            Layout::top_down(Align::Min),
            |ui| {
                Frame::new()
                    .fill(cp.window)
                    .inner_margin(Margin::symmetric(24, 18))
                    .show(ui, |ui| {
                        ui.set_width((content_width - 48.0).max(272.0));
                        ui.set_min_height((available.y - 36.0).max(320.0));
                        let (title, subtitle) = match view {
                            WorkspaceView::Overview => {
                                ("Meu dia", "Prioridades matinais e próximos passos")
                            }
                            WorkspaceView::Projects => {
                                ("Projetos", "Clientes, entregas e conversas vinculadas")
                            }
                            WorkspaceView::Tasks => (
                                "Tarefas",
                                "Quadro operacional de trabalho organizado por status",
                            ),
                            WorkspaceView::Timeline => {
                                ("Timeline", "Histórico operacional consolidado por dia")
                            }
                            WorkspaceView::Calls => (
                                "Calls de Venda",
                                "Reuniões comerciais, taxa de conversão e fechamentos",
                            ),
                            WorkspaceView::Agenda => {
                                ("Agenda", "Prazos, reuniões e follow-ups da conta")
                            }
                            WorkspaceView::Funnel => {
                                ("Funil comercial", "Negócios e etapas do pipeline de vendas")
                            }
                        };

                        crm_header(ui, &cp, title, subtitle, |ui| match view {
                            WorkspaceView::Overview => {
                                let btn = egui::Button::new(
                                    RichText::new("+ Nova tarefa")
                                        .size(13.0)
                                        .strong()
                                        .color(cp.on_accent),
                                )
                                .fill(cp.accent)
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(130.0, 34.0));
                                if ui.add(btn).clicked() {
                                    view = WorkspaceView::Tasks;
                                }
                            }
                            WorkspaceView::Funnel => {
                                let btn = egui::Button::new(
                                    RichText::new("Abrir quadro kanban")
                                        .size(13.0)
                                        .strong()
                                        .color(cp.on_accent),
                                )
                                .fill(cp.accent)
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(170.0, 34.0));
                                if ui.add(btn).clicked() {
                                    app.actions
                                        .push(Action::ShowDialog(crate::model::Dialog::Kanban));
                                }
                            }
                            _ => {}
                        });

                        ScrollArea::vertical().auto_shrink([false, false]).show(
                            ui,
                            |ui| match view {
                                WorkspaceView::Overview => overview(app, ui, &mut view, &cp),
                                WorkspaceView::Projects => {
                                    projects(app, ui, &mut draft, &mut selected, &cp)
                                }
                                WorkspaceView::Tasks => super::crm_tasks::show_workspace(app, ui),
                                WorkspaceView::Timeline => timeline(app, ui, &cp),
                                WorkspaceView::Calls => {
                                    super::crm_tasks::show_calls_workspace(app, ui)
                                }
                                WorkspaceView::Agenda => {
                                    super::crm_tasks::show_agenda_workspace(app, ui)
                                }
                                WorkspaceView::Funnel => funnel(app, ui, &cp),
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

fn nav_item(
    ui: &mut egui::Ui,
    view: &mut WorkspaceView,
    target: WorkspaceView,
    icon: Icon,
    label: &str,
    palette: &CrmPalette,
) {
    let selected = *view == target;
    let width = ui.available_width();
    let height = 38.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, height), egui::Sense::click());

    if response.clicked() {
        *view = target;
    }

    let is_hovered = response.hovered();
    let bg = if selected {
        palette.accent
    } else if is_hovered {
        palette.surface_hover
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    }

    let (icon_color, text_color) = if selected {
        (palette.on_accent, palette.on_accent)
    } else if is_hovered {
        (palette.accent, palette.text)
    } else {
        (palette.dim, palette.secondary)
    };

    let icon_size = 18.0;
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        Vec2::splat(icon_size),
    );
    icon.image(icon_color, icon_size).paint_at(ui, icon_rect);

    let text_pos = egui::pos2(rect.left() + 38.0, rect.center().y - 8.5);
    let galley = ui.painter().layout_no_wrap(
        label.to_owned(),
        if selected {
            theme::bold(14.0)
        } else {
            theme::medium(13.5)
        },
        text_color,
    );
    ui.painter().galley(text_pos, galley, text_color);

    ui.add_space(3.0);
}

fn crm_header<F>(
    ui: &mut egui::Ui,
    palette: &CrmPalette,
    title: &str,
    subtitle: &str,
    render_controls: F,
) where
    F: FnOnce(&mut egui::Ui),
{
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(24.0).strong().color(palette.text));
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(13.0).color(palette.secondary));
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            render_controls(ui);
        });
    });
    ui.add_space(10.0);
    let cur = ui.cursor();
    ui.painter().hline(
        cur.left()..=(cur.left() + ui.available_width()),
        cur.top(),
        Stroke::new(1.0, palette.outline),
    );
    ui.add_space(12.0);
}

pub(crate) fn empty_state(
    ui: &mut egui::Ui,
    palette: &CrmPalette,
    icon: Icon,
    title: &str,
    subtitle: &str,
    action_label: Option<&str>,
) -> bool {
    let mut clicked = false;
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(24, 32))
        .show(ui, |ui| {
            ui.set_min_width((ui.available_width() - 48.0).max(320.0));
            ui.vertical_centered(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(52.0), egui::Sense::hover());
                ui.painter().circle_filled(
                    rect.center(),
                    26.0,
                    palette.accent.gamma_multiply(0.12),
                );
                ui.painter().circle_stroke(
                    rect.center(),
                    26.0,
                    Stroke::new(1.0, palette.accent.gamma_multiply(0.35)),
                );
                let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(24.0));
                icon.image(palette.accent, 24.0).paint_at(ui, icon_rect);

                ui.add_space(12.0);
                ui.label(RichText::new(title).size(18.0).strong().color(palette.text));
                ui.add_space(4.0);
                ui.label(RichText::new(subtitle).size(13.5).color(palette.secondary));

                if let Some(label) = action_label {
                    ui.add_space(16.0);
                    let btn = egui::Button::new(
                        RichText::new(label)
                            .size(13.5)
                            .strong()
                            .color(palette.on_accent),
                    )
                    .fill(palette.accent)
                    .corner_radius(CornerRadius::same(8))
                    .min_size(vec2(160.0, 36.0));
                    if ui.add(btn).clicked() {
                        clicked = true;
                    }
                }
            });
        });
    clicked
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TimelineKind {
    MessageSent,
    MessageReceived,
    Call,
    Task,
    Note,
}

impl TimelineKind {
    fn label(&self) -> &'static str {
        match self {
            Self::MessageSent => "Mensagem enviada",
            Self::MessageReceived => "Mensagem recebida",
            Self::Call => "Call de Venda",
            Self::Task => "Tarefa",
            Self::Note => "Anotação",
        }
    }

    fn filter_category(&self) -> &'static str {
        match self {
            Self::MessageSent | Self::MessageReceived => "message",
            Self::Call => "call",
            Self::Task => "task",
            Self::Note => "note",
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::MessageSent => Icon::Send,
            Self::MessageReceived => Icon::MessageCircle,
            Self::Call => Icon::Phone,
            Self::Task => Icon::ListChecks,
            Self::Note => Icon::Tag,
        }
    }

    fn color(&self, cp: &CrmPalette) -> Color32 {
        match self {
            Self::MessageSent => Color32::from_rgb(56, 189, 248),
            Self::MessageReceived => Color32::from_rgb(16, 185, 129),
            Self::Call => Color32::from_rgb(129, 140, 248),
            Self::Task => cp.accent,
            Self::Note => Color32::from_rgb(245, 158, 11),
        }
    }
}

struct TimelineEvent {
    timestamp: i64,
    date: Date,
    time_str: String,
    kind: TimelineKind,
    title: String,
    entity: String,
    summary: String,
    status_badge: Option<(String, Color32)>,
}

fn format_day_header(date: Date, today: Date) -> String {
    let days = today.since(date).map(|s| s.get_days()).unwrap_or(i32::MAX);
    let month_name = match date.month() {
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
    };
    if days == 0 {
        format!("Hoje • {:02} de {}", date.day(), month_name)
    } else if days == 1 {
        format!("Ontem • {:02} de {}", date.day(), month_name)
    } else if date.year() == today.year() {
        format!("{:02} de {}", date.day(), month_name)
    } else {
        format!("{:02} de {} de {}", date.day(), month_name, date.year())
    }
}

fn timeline(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette) {
    let account_id = app.id.as_str().to_owned();
    let filter_type_id = ui.id().with(("crm-timeline-type", &account_id));
    let search_id = ui.id().with(("crm-timeline-search", &account_id));

    let mut type_filter = ui.data_mut(|d| {
        d.get_temp::<String>(filter_type_id)
            .unwrap_or_else(|| "all".into())
    });
    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());

    // 1. Filter bar at top
    ui.horizontal(|ui| {
        let types = [
            ("all", "Todos"),
            ("message", "Mensagens"),
            ("call", "Calls"),
            ("task", "Tarefas"),
            ("note", "Anotações"),
        ];

        for (val, label) in types {
            let selected = type_filter == val;
            let (fill, text_color) = if selected {
                (cp.accent, cp.on_accent)
            } else {
                (cp.surface, cp.secondary)
            };

            let btn = egui::Button::new(RichText::new(label).size(12.5).strong().color(text_color))
                .fill(fill)
                .stroke(Stroke::new(
                    1.0,
                    if selected { cp.accent } else { cp.outline },
                ))
                .corner_radius(CornerRadius::same(7))
                .min_size(vec2(72.0, 30.0));

            if ui.add(btn).clicked() {
                type_filter = val.into();
            }
        }

        ui.add_space(8.0);

        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("Buscar por cliente, projeto, assunto...")
                .desired_width(240.0),
        );

        if !search.is_empty() {
            let clear_btn = egui::Button::new(RichText::new("✕").size(11.0).color(cp.dim))
                .fill(cp.surface)
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(24.0, 24.0));
            if ui.add(clear_btn).clicked() {
                search.clear();
            }
        }
    });

    ui.add_space(14.0);

    // 2. Gather events
    let mut events: Vec<TimelineEvent> = Vec::new();

    // 2a. Messages
    for (chat_id, conversation) in &app.conversations {
        let client = app
            .chat(chat_id)
            .map(|chat| app.chat_title(chat))
            .unwrap_or_else(|| "Conversa".into());
        for message in conversation.messages.iter().rev().take(30) {
            if let Some((date, hour, minute)) = crate::util::local_datetime(message.timestamp) {
                events.push(TimelineEvent {
                    timestamp: message.timestamp,
                    date,
                    time_str: format!("{:02}:{:02}", hour, minute),
                    kind: if message.from_me {
                        TimelineKind::MessageSent
                    } else {
                        TimelineKind::MessageReceived
                    },
                    title: client.clone(),
                    entity: client.clone(),
                    summary: message.summary(),
                    status_badge: None,
                });
            }
        }
    }

    // 2b. Tasks and Calls
    for task in &app.crm_tasks {
        let ts = task.due_at.unwrap_or(task.updated_at);
        if let Some((date, hour, minute)) = crate::util::local_datetime(ts) {
            let client_name = task
                .chat_id
                .as_ref()
                .and_then(|cid| app.chat(cid))
                .map(|chat| app.chat_title(chat));
            let project_name = task
                .project_id
                .as_ref()
                .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                .map(|p| p.name.clone());

            let entity = match (client_name, project_name) {
                (Some(c), Some(p)) => format!("{} • {}", c, p),
                (Some(c), None) => c,
                (None, Some(p)) => p,
                (None, None) => "Geral".to_string(),
            };

            let is_call = matches!(task.kind.as_str(), "call" | "conversation");
            if is_call {
                let badge = match task.status.as_str() {
                    "scheduled" => Some(("Agendada".into(), Color32::from_rgb(56, 189, 248))),
                    "held" => Some(("Realizada".into(), Color32::from_rgb(16, 185, 129))),
                    "closed" => Some(("Fechada".into(), cp.accent)),
                    "no_show" => Some(("No-show".into(), Color32::from_rgb(245, 158, 11))),
                    "cancelled" => Some(("Cancelada".into(), cp.danger)),
                    _ => None,
                };
                let summary = if !task.call_result.trim().is_empty() {
                    format!("Resultado: {} • {}", task.call_result, task.description)
                } else if !task.description.trim().is_empty() {
                    task.description.clone()
                } else {
                    "Reunião de alinhamento / vendas".to_string()
                };
                events.push(TimelineEvent {
                    timestamp: ts,
                    date,
                    time_str: format!("{:02}:{:02}", hour, minute),
                    kind: TimelineKind::Call,
                    title: task.title.clone(),
                    entity,
                    summary,
                    status_badge: badge,
                });
            } else {
                let badge = match task.status.as_str() {
                    "todo" => Some(("A fazer".into(), cp.dim)),
                    "doing" => Some(("Em andamento".into(), Color32::from_rgb(56, 189, 248))),
                    "review" => Some(("Revisão".into(), Color32::from_rgb(245, 158, 11))),
                    "done" => Some(("Concluída".into(), cp.accent)),
                    _ => None,
                };
                events.push(TimelineEvent {
                    timestamp: ts,
                    date,
                    time_str: format!("{:02}:{:02}", hour, minute),
                    kind: TimelineKind::Task,
                    title: task.title.clone(),
                    entity,
                    summary: if task.description.trim().is_empty() {
                        "Tarefa operacional".to_string()
                    } else {
                        task.description.clone()
                    },
                    status_badge: badge,
                });
            }
        }
    }

    // 2c. Notes from CRM Deals
    for deal in app
        .crm_deals
        .values()
        .filter(|deal| !deal.notes.trim().is_empty())
    {
        let ts = deal.updated_at;
        if let Some((date, hour, minute)) = crate::util::local_datetime(ts) {
            let client = app
                .chat(&deal.chat_id)
                .map(|chat| app.chat_title(chat))
                .unwrap_or_else(|| "Cliente".into());
            let stage_badge = if deal.value_cents > 0 {
                Some((
                    format!("R$ {:.2}", deal.value_cents as f64 / 100.0),
                    cp.accent,
                ))
            } else if !deal.column_id.trim().is_empty() {
                Some((deal.column_id.clone(), Color32::from_rgb(245, 158, 11)))
            } else {
                None
            };
            events.push(TimelineEvent {
                timestamp: ts,
                date,
                time_str: format!("{:02}:{:02}", hour, minute),
                kind: TimelineKind::Note,
                title: format!("Anotação: {}", client),
                entity: client,
                summary: deal.notes.clone(),
                status_badge: stage_badge,
            });
        }
    }

    // 2d. Follow-up reminders
    for followup in &app.crm_followups {
        let ts = followup.remind_at;
        if let Some((date, hour, minute)) = crate::util::local_datetime(ts) {
            let client = app
                .chat(&followup.chat_id)
                .map(|chat| app.chat_title(chat))
                .unwrap_or_else(|| "Contato".into());
            let badge = if followup.done {
                Some(("Follow-up concluído".into(), cp.accent))
            } else {
                Some(("Lembrete pendente".into(), Color32::from_rgb(245, 158, 11)))
            };
            events.push(TimelineEvent {
                timestamp: ts,
                date,
                time_str: format!("{:02}:{:02}", hour, minute),
                kind: TimelineKind::Note,
                title: followup.title.clone(),
                entity: client,
                summary: "Lembrete de follow-up cadastrado".to_string(),
                status_badge: badge,
            });
        }
    }

    // Sort descending by timestamp
    events.sort_by_key(|e| std::cmp::Reverse(e.timestamp));

    // 3. Filter events
    let search_lower = search.trim().to_lowercase();
    let filtered: Vec<_> = events
        .into_iter()
        .filter(|e| {
            let matches_type = match type_filter.as_str() {
                "all" => true,
                other => e.kind.filter_category() == other,
            };
            let matches_search = search_lower.is_empty()
                || e.entity.to_lowercase().contains(&search_lower)
                || e.title.to_lowercase().contains(&search_lower)
                || e.summary.to_lowercase().contains(&search_lower);
            matches_type && matches_search
        })
        .collect();

    if filtered.is_empty() {
        if empty_state(
            ui,
            cp,
            Icon::Clock,
            "Nenhum evento encontrado",
            "Não foram encontrados registros para os filtros selecionados.",
            Some("Limpar filtros"),
        ) {
            type_filter = "all".into();
            search.clear();
        }
        ui.data_mut(|d| {
            d.insert_temp(filter_type_id, type_filter);
            d.insert_temp(search_id, search);
        });
        return;
    }

    // 4. Group by Day
    let today = crate::util::today();
    let mut grouped: Vec<(Date, Vec<TimelineEvent>)> = Vec::new();
    for event in filtered.into_iter().take(150) {
        if let Some(last) = grouped.last_mut() {
            if last.0 == event.date {
                last.1.push(event);
                continue;
            }
        }
        grouped.push((event.date, vec![event]));
    }

    // 5. Render grouped timeline
    for (date, day_events) in grouped {
        // Day Separator Header
        ui.horizontal(|ui| {
            ui.label(RichText::new("●").size(10.0).color(cp.accent));
            ui.add_space(2.0);
            ui.label(
                RichText::new(format_day_header(date, today))
                    .size(15.0)
                    .strong()
                    .color(cp.text),
            );
            Frame::new()
                .fill(cp.surface)
                .corner_radius(CornerRadius::same(10))
                .inner_margin(Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("{} eventos", day_events.len()))
                            .size(11.0)
                            .color(cp.dim),
                    );
                });
        });

        ui.add_space(6.0);

        // Render each event in this day
        for event in day_events {
            let kind_color = event.kind.color(cp);

            ui.horizontal(|ui| {
                // Left Icon Node
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
                ui.painter().rect(
                    rect,
                    CornerRadius::same(8),
                    kind_color.gamma_multiply(0.16),
                    Stroke::new(1.0, kind_color.gamma_multiply(0.45)),
                    egui::epaint::StrokeKind::Inside,
                );
                theme::paint_icon(ui, event.kind.icon(), rect, 16.0, kind_color);

                ui.add_space(4.0);

                // Right Card Content
                Frame::new()
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width() - 8.0);

                        // Card Header
                        ui.horizontal(|ui| {
                            // Kind pill
                            Frame::new()
                                .fill(kind_color.gamma_multiply(0.14))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(Margin::symmetric(7, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(event.kind.label())
                                            .size(11.0)
                                            .strong()
                                            .color(kind_color),
                                    );
                                });

                            // Entity pill
                            if !event.entity.is_empty() {
                                ui.label(
                                    RichText::new(&event.entity)
                                        .size(12.5)
                                        .strong()
                                        .color(cp.text),
                                );
                            }

                            // Status badge
                            if let Some((badge_text, badge_color)) = &event.status_badge {
                                Frame::new()
                                    .fill(badge_color.gamma_multiply(0.14))
                                    .corner_radius(CornerRadius::same(5))
                                    .inner_margin(Margin::symmetric(7, 2))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(badge_text)
                                                .size(11.0)
                                                .strong()
                                                .color(*badge_color),
                                        );
                                    });
                            }

                            // Right-aligned time
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                ui.label(RichText::new(&event.time_str).size(12.0).color(cp.dim));
                            });
                        });

                        // Title if different from entity
                        if !event.title.is_empty() && event.title != event.entity {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(&event.title)
                                    .size(13.5)
                                    .strong()
                                    .color(cp.text),
                            );
                        }

                        // Summary
                        if !event.summary.is_empty() {
                            ui.add_space(2.0);
                            ui.label(RichText::new(&event.summary).size(12.5).color(cp.secondary));
                        }
                    });
            });

            ui.add_space(6.0);
        }

        ui.add_space(10.0);
    }

    ui.data_mut(|d| {
        d.insert_temp(filter_type_id, type_filter);
        d.insert_temp(search_id, search);
    });
}

fn funnel(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette) {
    if empty_state(
        ui,
        cp,
        Icon::Tag,
        "Acompanhe seus negócios por etapa",
        "Abra o quadro comercial para mover contatos, registrar valores e consultar métricas de conversão.",
        Some("Abrir quadro comercial"),
    ) {
        app.actions
            .push(Action::ShowDialog(crate::model::Dialog::Kanban));
    }
}

fn overview(app: &mut App, ui: &mut egui::Ui, view: &mut WorkspaceView, cp: &CrmPalette) {
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
            cp,
            Icon::Users,
            "Projetos ativos",
            active_projects,
            metric_width,
        );
        metric(
            ui,
            cp,
            Icon::ListChecks,
            "Tarefas abertas",
            open_tasks,
            metric_width,
        );
        metric(ui, cp, Icon::CircleAlert, "Atrasadas", late, metric_width);
        metric(ui, cp, Icon::Bell, "Follow-ups", followups, metric_width);
    });
    ui.add_space(22.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Próximas ações")
                .size(18.0)
                .strong()
                .color(cp.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let see_all = egui::Button::new(
                RichText::new("Ver todas as tarefas →")
                    .size(13.0)
                    .strong()
                    .color(cp.accent),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6));
            if ui.add(see_all).clicked() {
                *view = WorkspaceView::Tasks;
            }
        });
    });
    ui.add_space(10.0);
    let mut tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.status != "done")
        .cloned()
        .collect();
    tasks.sort_by_key(|t| t.due_at.unwrap_or(i64::MAX));
    if tasks.is_empty() {
        if empty_state(
            ui,
            cp,
            Icon::CircleCheck,
            "Tudo em dia!",
            "Você não possui tarefas pendentes no momento. Crie uma nova tarefa para organizar suas próximas entregas.",
            Some("+ Nova tarefa"),
        ) {
            *view = WorkspaceView::Tasks;
        }
        return;
    }
    let mut to_complete: Option<CrmTask> = None;
    for task in tasks.into_iter().take(8) {
        let is_late = task.due_at.is_some_and(|d| d < now);
        let project_name = task
            .project_id
            .as_ref()
            .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
            .map(|p| p.name.clone());
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(
                1.0,
                if is_late { cp.danger } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(16, 12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Checkbox button to complete
                    let check_btn =
                        egui::Button::new(RichText::new("□").size(15.0).color(if is_late {
                            cp.danger
                        } else {
                            cp.accent
                        }))
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(5))
                        .min_size(vec2(26.0, 26.0));
                    if ui.add(check_btn).on_hover_text("Concluir tarefa").clicked() {
                        let mut updated = task.clone();
                        updated.status = "done".into();
                        updated.completed_at = Some(now);
                        updated.updated_at = now;
                        to_complete = Some(updated);
                    }
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&task.title)
                                .size(14.0)
                                .strong()
                                .color(cp.text),
                        );
                        if let Some(p_name) = &project_name {
                            ui.label(
                                RichText::new(format!("📁 {}", p_name))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if is_late {
                            Frame::new()
                                .fill(cp.danger.gamma_multiply(0.15))
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(8, 3))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Atrasada")
                                            .size(11.0)
                                            .strong()
                                            .color(cp.danger),
                                    );
                                });
                        }
                        if let Some(due) = task.due_at {
                            if let Some((date, _, _)) = crate::util::local_datetime(due) {
                                ui.label(
                                    RichText::new(format!("{:02}/{:02}", date.day(), date.month()))
                                        .size(12.0)
                                        .color(if is_late { cp.danger } else { cp.dim }),
                                );
                            }
                        }
                    });
                });
            });
        ui.add_space(8.0);
    }
    if let Some(t) = to_complete {
        app.actions.push(Action::SaveCrmTask(t));
    }
}

fn metric(
    ui: &mut egui::Ui,
    palette: &CrmPalette,
    icon: Icon,
    title: &str,
    value: usize,
    width: f32,
) {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.horizontal(|ui| {
                theme::icon(ui, icon, 18.0, palette.accent);
                ui.add_space(2.0);
                ui.label(RichText::new(title).size(12.5).color(palette.secondary));
            });
            ui.add_space(6.0);
            ui.label(
                RichText::new(value.to_string())
                    .size(28.0)
                    .strong()
                    .color(palette.text),
            );
        });
}

fn projects(
    app: &mut App,
    ui: &mut egui::Ui,
    draft: &mut ProjectDraft,
    selected: &mut Option<String>,
    cp: &CrmPalette,
) {
    if let Some(id) = selected.clone() {
        if let Some(project) = app.crm_projects.iter().find(|p| p.id == id).cloned() {
            project_detail(app, ui, project, selected, cp);
            return;
        }
        *selected = None;
    }

    let current = app.current_chat().cloned();
    let account_id = app.id.as_str().to_owned();
    let form_id = ui.id().with(("crm-project-form-open", &account_id));
    let search_id = ui.id().with(("crm-project-search", &account_id));
    let status_id = ui.id().with(("crm-project-status-filter", &account_id));
    let mut show_form = ui.data_mut(|d| d.get_temp::<bool>(form_id).unwrap_or(false));
    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
    let mut status = ui.data_mut(|d| {
        d.get_temp::<String>(status_id)
            .unwrap_or_else(|| "all".into())
    });

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut search)
                .hint_text("Buscar projeto…")
                .desired_width(280.0),
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
            let btn = egui::Button::new(
                RichText::new("+ Novo projeto")
                    .size(13.5)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(140.0, 36.0));

            if ui.add(btn).clicked() {
                show_form = true;
            }
        });
    });

    ui.add_space(14.0);

    // Floating Modal Window for New Project
    if show_form {
        let mut is_open = true;
        egui::Window::new("Novo Projeto")
            .id(ui.id().with("crm-project-floating-modal"))
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
                render_project_form_modal(app, ui, cp, draft, &current, &mut show_form);
            });
        if !is_open {
            show_form = false;
            *draft = ProjectDraft::default();
        }
    }

    ui.data_mut(|d| {
        d.insert_temp(form_id, show_form);
        d.insert_temp(search_id, search.clone());
        d.insert_temp(status_id, status.clone());
    });

    let needle = search.trim().to_lowercase();
    let filtered_projects: Vec<_> = app
        .crm_projects
        .iter()
        .filter(|project| {
            (status == "all" || project.status == status)
                && (needle.is_empty()
                    || project.name.to_lowercase().contains(&needle)
                    || project.description.to_lowercase().contains(&needle)
                    || project.client_name.to_lowercase().contains(&needle)
                    || project.assignee.to_lowercase().contains(&needle))
        })
        .cloned()
        .collect();

    if app.crm_projects.is_empty() {
        if empty_state(
            ui,
            cp,
            Icon::Users,
            "Nenhum projeto cadastrado",
            "Crie seu primeiro projeto para gerenciar tarefas, prazos e conversas vinculadas.",
            Some("+ Novo projeto"),
        ) {
            ui.data_mut(|d| d.insert_temp(form_id, true));
        }
        return;
    }

    if filtered_projects.is_empty() {
        ui.add_space(20.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Nenhum projeto encontrado para esta busca.")
                    .size(14.0)
                    .color(cp.dim),
            );
        });
        return;
    }

    // Grid of cards
    let avail_w = ui.available_width();
    let num_cols = if avail_w > 940.0 {
        3
    } else if avail_w > 560.0 {
        2
    } else {
        1
    };
    let gap = 14.0;
    let card_w = ((avail_w - (num_cols as f32 - 1.0) * gap) / num_cols as f32).max(240.0);

    for row_projects in filtered_projects.chunks(num_cols) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for project in row_projects {
                render_project_card(app, ui, cp, project, card_w, selected);
            }
        });
        ui.add_space(gap);
    }
}

fn render_project_card(
    app: &App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &CrmProject,
    width: f32,
    selected: &mut Option<String>,
) {
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
    let progress = if total == 0 {
        0.0
    } else {
        done as f32 / total as f32
    };

    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.vertical(|ui| {
                // Top row: Project status pill + Kind badge + Open button
                ui.horizontal(|ui| {
                    let (status_bg, status_fg, status_txt) = match project.status.as_str() {
                        "active" => (cp.accent.gamma_multiply(0.15), cp.accent, "Ativo"),
                        "paused" => (cp.warning.gamma_multiply(0.15), cp.warning, "Pausado"),
                        "completed" => (
                            Color32::from_rgb(0x1a, 0x2e, 0x3d),
                            Color32::from_rgb(0x5c, 0xa7, 0xdb),
                            "Concluído",
                        ),
                        _ => (cp.surface_hover, cp.dim, "Arquivado"),
                    };
                    Frame::new()
                        .fill(status_bg)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(status_txt)
                                    .size(11.0)
                                    .strong()
                                    .color(status_fg),
                            );
                        });

                    if project.kind == "internal" {
                        Frame::new()
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Interno").size(11.0).color(cp.secondary));
                            });
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let btn = egui::Button::new(
                            RichText::new("Abrir →")
                                .size(12.0)
                                .strong()
                                .color(cp.accent),
                        )
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(btn).clicked() {
                            *selected = Some(project.id.clone());
                        }
                    });
                });

                ui.add_space(8.0);

                // Project Title
                ui.label(
                    RichText::new(&project.name)
                        .size(16.0)
                        .strong()
                        .color(cp.text),
                );

                // Description
                if !project.description.is_empty() {
                    ui.add_space(2.0);
                    let snippet = if project.description.chars().count() > 70 {
                        format!(
                            "{}...",
                            project.description.chars().take(70).collect::<String>()
                        )
                    } else {
                        project.description.clone()
                    };
                    ui.label(RichText::new(snippet).size(12.0).color(cp.secondary));
                }

                ui.add_space(10.0);

                // Client & Assignee
                if !project.client_name.is_empty() {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Users, 13.0, cp.dim);
                        ui.label(
                            RichText::new(format!("Cliente: {}", project.client_name))
                                .size(11.5)
                                .color(cp.secondary),
                        );
                    });
                }
                if !project.assignee.is_empty() {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::User, 13.0, cp.dim);
                        ui.label(
                            RichText::new(format!("Responsável: {}", project.assignee))
                                .size(11.5)
                                .color(cp.dim),
                        );
                    });
                }

                // Due Date
                if let Some((date, _, _)) = project.due_at.and_then(crate::util::local_datetime) {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Clock, 13.0, cp.dim);
                        ui.label(
                            RichText::new(format!(
                                "Prazo: {}",
                                crate::util::long_date(app.locale, date)
                            ))
                            .size(11.5)
                            .color(cp.dim),
                        );
                    });
                }

                ui.add_space(10.0);

                // Progress Bar & Task Count
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{done}/{total} tarefas ({:.0}%)", progress * 100.0))
                            .size(11.5)
                            .color(cp.secondary),
                    );
                });
                ui.add_space(4.0);
                ui.add_sized(
                    [ui.available_width(), 6.0],
                    egui::ProgressBar::new(progress)
                        .fill(cp.accent)
                        .animate(false),
                );
            });
        });
}

fn render_project_form_modal(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    draft: &mut ProjectDraft,
    current: &Option<crate::model::Chat>,
    show_form: &mut bool,
) {
    let err_id = ui.id().with("crm-project-modal-err");
    let show_err = ui.data_mut(|d| d.get_temp::<bool>(err_id).unwrap_or(false));

    ui.vertical(|ui| {
        ui.label(
            RichText::new("Nome do projeto *")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let name_stroke = if show_err && draft.name.trim().is_empty() {
            Stroke::new(1.0, cp.danger)
        } else {
            Stroke::new(1.0, cp.outline)
        };
        Frame::new()
            .fill(cp.surface)
            .stroke(name_stroke)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::symmetric(8, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut draft.name)
                        .hint_text("Ex.: Redesenho Central de Atendimento")
                        .desired_width(ui.available_width())
                        .frame(Frame::NONE),
                );
            });
        if show_err && draft.name.trim().is_empty() {
            ui.label(
                RichText::new("O nome do projeto é obrigatório")
                    .size(11.0)
                    .color(cp.danger),
            );
        }

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Tipo de projeto")
                        .size(12.0)
                        .strong()
                        .color(cp.secondary),
                );
                ui.add_space(2.0);
                egui::ComboBox::from_id_salt("modal-proj-kind")
                    .selected_text(if draft.kind == "internal" {
                        "Interno"
                    } else {
                        "Cliente"
                    })
                    .width(ui.available_width() - 8.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut draft.kind, "client".into(), "Cliente");
                        ui.selectable_value(&mut draft.kind, "internal".into(), "Interno");
                    });
            });

            uis[1].vertical(|ui| {
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
        });

        ui.add_space(10.0);

        ui.columns(2, |uis| {
            uis[0].vertical(|ui| {
                ui.label(
                    RichText::new("Nome do cliente")
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
                            egui::TextEdit::singleline(&mut draft.client_name)
                                .hint_text("Empresa ou Contato")
                                .desired_width(ui.available_width())
                                .frame(Frame::NONE),
                        );
                    });
            });

            uis[1].vertical(|ui| {
                ui.label(
                    RichText::new("Telefone")
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
                            egui::TextEdit::singleline(&mut draft.client_phone)
                                .hint_text("+55...")
                                .desired_width(ui.available_width())
                                .frame(Frame::NONE),
                        );
                    });
            });
        });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Descrição e objetivo")
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
                        .hint_text("Resumo do escopo e metas do projeto...")
                        .desired_width(ui.available_width())
                        .desired_rows(3)
                        .frame(Frame::NONE),
                );
            });

        ui.add_space(10.0);

        ui.label(
            RichText::new("Data de início")
                .size(12.0)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(2.0);
        let cal_open_id = ui.id().with("modal-proj-cal-open");
        let mut cal_open = ui.data_mut(|d| d.get_temp::<bool>(cal_open_id).unwrap_or(false));
        let date_str = format!(
            "📅 {:02}/{:02}/{} às {:02}:{:02}",
            draft.start_date.day(),
            draft.start_date.month(),
            draft.start_date.year(),
            draft.start_hour,
            draft.start_minute
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
                        "project-modal-start-date",
                        &mut draft.start_date,
                        &mut draft.start_hour,
                        &mut draft.start_minute,
                    );
                });
        }
        ui.data_mut(|d| d.insert_temp(cal_open_id, cal_open));

        ui.add_space(8.0);

        if current.is_some() {
            ui.checkbox(
                &mut draft.link_current_chat,
                "Vincular conversa/grupo aberto",
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
                *draft = ProjectDraft::default();
                ui.data_mut(|d| d.insert_temp(err_id, false));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let save_btn = egui::Button::new(
                    RichText::new("Criar projeto")
                        .size(13.0)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(140.0, 36.0));

                if ui.add(save_btn).clicked() {
                    if draft.name.trim().is_empty() {
                        ui.data_mut(|d| d.insert_temp(err_id, true));
                        return;
                    }

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
                        color: "#00e699".into(),
                        icon: "briefcase".into(),
                        start_at: crate::util::to_unix_seconds(
                            draft.start_date,
                            draft.start_hour,
                            draft.start_minute,
                        ),
                        due_at: None,
                        created_at: now,
                        updated_at: now,
                        assignee: draft.assignee.trim().into(),
                        client_name: draft.client_name.trim().into(),
                        client_phone: draft.client_phone.trim().into(),
                        client_email: draft.client_email.trim().into(),
                        chat_ids,
                    }));

                    *draft = ProjectDraft::default();
                    *show_form = false;
                    ui.data_mut(|d| d.insert_temp(err_id, false));
                }
            });
        });
    });
}

fn project_detail(
    app: &mut App,
    ui: &mut egui::Ui,
    mut project: CrmProject,
    selected: &mut Option<String>,
    _cp: &CrmPalette,
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
                ui.horizontal_wrapped(|ui| {
                    ui.add_sized(
                        [220.0, 28.0],
                        egui::TextEdit::singleline(&mut edit.assignee).hint_text("Responsável"),
                    );
                    ui.add_sized(
                        [220.0, 28.0],
                        egui::TextEdit::singleline(&mut edit.client_name)
                            .hint_text("Nome do cliente"),
                    );
                    ui.add_sized(
                        [180.0, 28.0],
                        egui::TextEdit::singleline(&mut edit.client_phone).hint_text("Telefone"),
                    );
                    ui.add_sized(
                        [240.0, 28.0],
                        egui::TextEdit::singleline(&mut edit.client_email).hint_text("E-mail"),
                    );
                });
                ui.label("Data de início");
                super::dialogs::date_time_picker(
                    ui,
                    app.locale,
                    &app.palette,
                    "project-edit-start-date",
                    &mut edit.start_date,
                    &mut edit.start_hour,
                    &mut edit.start_minute,
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
                    project.assignee = edit.assignee.trim().into();
                    project.client_name = edit.client_name.trim().into();
                    project.client_phone = edit.client_phone.trim().into();
                    project.client_email = edit.client_email.trim().into();
                    project.start_at = crate::util::to_unix_seconds(
                        edit.start_date,
                        edit.start_hour,
                        edit.start_minute,
                    );
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
    ui.horizontal_wrapped(|ui| {
        if !project.assignee.is_empty() {
            ui.label(format!("Responsável: {}", project.assignee));
        }
        if !project.client_name.is_empty() {
            ui.label(format!("Cliente: {}", project.client_name));
        }
        if let Some((date, _, _)) = project.start_at.and_then(crate::util::local_datetime) {
            ui.label(format!(
                "Início: {:02}/{:02}/{}",
                date.day(),
                date.month(),
                date.year()
            ));
        }
    });
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
                    kind: "call".into(),
                    duration_minutes: Some(conversation.duration_minutes),
                    assignee: project.assignee.clone(),
                    call_result: String::new(),
                    next_step: String::new(),
                    revenue_cents: 0,
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
        task.project_id.as_deref() == Some(&project.id)
            && matches!(task.kind.as_str(), "call" | "conversation")
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
                assignee: project.assignee.clone(),
                call_result: String::new(),
                next_step: String::new(),
                revenue_cents: 0,
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
        if task.project_id.as_deref() == Some(&project.id)
            && !matches!(task.kind.as_str(), "call" | "conversation")
        {
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
