//! Account-scoped CRM workspace. Projects reference chats by id and never copy messages.

use super::crm_sidecar::{format_currency, parse_currency_cents, parse_hex_color};
use crate::app::App;
use crate::model::{Action, CrmColumn, CrmDeal, CrmProject, CrmTask, Page};
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
            link_current_chat: false,
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

    if let Some(req) = app.crm_requested_view.take() {
        match req.as_str() {
            "funnel" => view = WorkspaceView::Funnel,
            "tasks" => view = WorkspaceView::Tasks,
            "projects" => view = WorkspaceView::Projects,
            "timeline" => view = WorkspaceView::Timeline,
            "calls" => view = WorkspaceView::Calls,
            "agenda" => view = WorkspaceView::Agenda,
            _ => view = WorkspaceView::Overview,
        }
    }

    let available = ui.available_size();
    let gap = ui.spacing().item_spacing.x;
    let (sidebar_width, content_width) = workspace_widths(available.x, gap);

    let is_compact_sidebar = sidebar_width <= 70.0;
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            vec2(sidebar_width, available.y),
            Layout::top_down(Align::Min),
            |ui| {
                let pad_x = if is_compact_sidebar { 6 } else { 14 };
                let pad_y = if is_compact_sidebar { 10 } else { 14 };
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::symmetric(pad_x, pad_y))
                    .show(ui, |ui| {
                        ui.set_width((sidebar_width - (pad_x * 2) as f32).max(36.0));
                        ui.set_min_height((available.y - (pad_y * 2) as f32).max(320.0));
                        if is_compact_sidebar {
                            ui.vertical_centered(|ui| {
                                let (_rect, resp) =
                                    ui.allocate_exact_size(Vec2::splat(28.0), egui::Sense::hover());
                                theme::icon(ui, Icon::ListChecks, 22.0, cp.accent);
                                resp.on_hover_text("ZapFast CRM • Central de Operações");
                            });
                        } else {
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
                        }
                        ui.add_space(if is_compact_sidebar { 8.0 } else { 14.0 });
                        let cur = ui.cursor();
                        ui.painter().hline(
                            cur.left()..=(cur.left() + ui.available_width()),
                            cur.top(),
                            Stroke::new(1.0, cp.outline),
                        );
                        ui.add_space(if is_compact_sidebar { 8.0 } else { 14.0 });
                        if !is_compact_sidebar {
                            ui.label(RichText::new("OPERAÇÕES").size(10.5).strong().color(cp.dim));
                            ui.add_space(6.0);
                        }
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
                            if is_compact_sidebar {
                                let back_btn = egui::Button::new(
                                    RichText::new("←").size(15.0).strong().color(cp.secondary),
                                )
                                .fill(cp.surface)
                                .stroke(Stroke::new(1.0, cp.outline))
                                .corner_radius(CornerRadius::same(8))
                                .min_size(vec2(ui.available_width(), 34.0));
                                if ui
                                    .add(back_btn)
                                    .on_hover_text("Voltar às Conversas")
                                    .clicked()
                                {
                                    app.actions.push(Action::Open(Page::Chats));
                                    ui.ctx().request_repaint();
                                }
                            } else {
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
                                    ui.ctx().request_repaint();
                                }
                            }
                        });
                    });
            },
        );

        let content_margin_x = if content_width < 500.0 { 12 } else { 24 };
        let content_margin_y = if content_width < 500.0 { 10 } else { 18 };
        ui.allocate_ui_with_layout(
            vec2(content_width, available.y),
            Layout::top_down(Align::Min),
            |ui| {
                Frame::new()
                    .fill(cp.window)
                    .inner_margin(Margin::symmetric(content_margin_x, content_margin_y))
                    .show(ui, |ui| {
                        let pad = (content_margin_x * 2) as f32;
                        ui.set_width((content_width - pad).max(160.0));
                        ui.set_min_height((available.y - (content_margin_y * 2) as f32).max(200.0));
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
                                    ui.ctx().request_repaint();
                                }
                            }
                            WorkspaceView::Calls => {}
                            WorkspaceView::Funnel => {
                                let add_stage_id = egui::Id::new("funnel_add_stage_open");
                                let add_deal_id = egui::Id::new("funnel_add_deal_open");
                                let mut add_stage_open = ui
                                    .ctx()
                                    .data(|d| d.get_temp::<bool>(add_stage_id))
                                    .unwrap_or(false);
                                let mut add_deal_open = ui
                                    .ctx()
                                    .data(|d| d.get_temp::<bool>(add_deal_id))
                                    .unwrap_or(false);

                                ui.horizontal(|ui| {
                                    let btn_stage = egui::Button::new(
                                        RichText::new("+ Nova Etapa").size(13.0).color(cp.text),
                                    )
                                    .fill(cp.surface)
                                    .stroke(Stroke::new(1.0, cp.outline))
                                    .corner_radius(CornerRadius::same(8))
                                    .min_size(vec2(120.0, 34.0));
                                    if ui.add(btn_stage).clicked() {
                                        add_stage_open = !add_stage_open;
                                        ui.ctx().request_repaint();
                                    }

                                    let btn_deal = egui::Button::new(
                                        RichText::new("+ Novo Negócio")
                                            .size(13.0)
                                            .strong()
                                            .color(cp.on_accent),
                                    )
                                    .fill(cp.accent)
                                    .corner_radius(CornerRadius::same(8))
                                    .min_size(vec2(140.0, 34.0));
                                    if ui.add(btn_deal).clicked() {
                                        add_deal_open = !add_deal_open;
                                        ui.ctx().request_repaint();
                                    }
                                });

                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(add_stage_id, add_stage_open);
                                    d.insert_temp(add_deal_id, add_deal_open);
                                });
                            }
                            _ => {}
                        });

                        ScrollArea::vertical().auto_shrink([false, false]).show(
                            ui,
                            |ui| match view {
                                WorkspaceView::Overview => {
                                    overview(app, ui, &mut view, &mut selected, &cp)
                                }
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
    let sidebar = if available < 600.0 {
        150.0_f32.min((available * 0.28).max(120.0))
    } else {
        218.0_f32.min((available * 0.30).max(170.0))
    };
    let content = (available - sidebar - gap).max(160.0);
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
    let avail = ui.available_width();
    if avail < 500.0 {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(22.0).strong().color(palette.text));
            ui.add_space(2.0);
            ui.label(RichText::new(subtitle).size(12.0).color(palette.secondary));
            ui.add_space(8.0);
            render_controls(ui);
        });
    } else {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(24.0).strong().color(palette.text));
                ui.add_space(2.0);
                ui.label(RichText::new(subtitle).size(13.0).color(palette.secondary));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(18.0);
                render_controls(ui);
            });
        });
    }
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
    let client_filter_id = ui.id().with(("crm-timeline-client", &account_id));

    let mut type_filter = ui.data_mut(|d| {
        d.get_temp::<String>(filter_type_id)
            .unwrap_or_else(|| "all".into())
    });
    let mut search = ui.data_mut(|d| d.get_temp::<String>(search_id).unwrap_or_default());
    let mut client_filter = ui.data_mut(|d| {
        d.get_temp::<String>(client_filter_id)
            .unwrap_or_else(|| "all".into())
    });

    let mut client_options: Vec<String> = Vec::new();
    for p in &app.crm_projects {
        if !p.client_name.trim().is_empty() && !client_options.contains(&p.client_name) {
            client_options.push(p.client_name.clone());
        }
    }
    for c in &app.chats {
        let t = app.chat_title(c);
        if !t.trim().is_empty() && !client_options.contains(&t) {
            client_options.push(t);
        }
    }
    client_options.sort();

    // 1. Filter bar at top (Responsive: never clips on narrow screens)
    let avail = ui.available_width();
    let render_type_buttons = |ui: &mut egui::Ui, tf: &mut String| {
        let types = [
            ("all", "Todos"),
            ("message", "Mensagens"),
            ("call", "Calls"),
            ("task", "Tarefas"),
            ("note", "Anotações"),
        ];

        for (val, label) in types {
            let selected = *tf == val;
            let (fill, text_color) = if selected {
                (cp.accent, cp.on_accent)
            } else {
                (cp.surface, cp.secondary)
            };

            let btn = egui::Button::new(RichText::new(label).size(12.0).strong().color(text_color))
                .fill(fill)
                .stroke(Stroke::new(
                    1.0,
                    if selected { cp.accent } else { cp.outline },
                ))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(66.0, 28.0));

            if ui.add(btn).clicked() {
                *tf = val.into();
                ui.ctx().request_repaint();
            }
        }
    };

    let is_masked = app.settings.crm_timeline_mask_messages;
    let show_mask_toggle = |ui: &mut egui::Ui, app: &mut App| {
        let masked = app.settings.crm_timeline_mask_messages;
        let mask_btn = egui::Button::new(
            RichText::new(if masked {
                "👁 Ocultando texto"
            } else {
                "👁 Ocultar texto"
            })
            .size(11.5)
            .strong()
            .color(if masked { cp.accent } else { cp.secondary }),
        )
        .fill(if masked {
            cp.accent.gamma_multiply(0.16)
        } else {
            cp.surface
        })
        .stroke(Stroke::new(1.0, if masked { cp.accent } else { cp.outline }))
        .corner_radius(CornerRadius::same(6))
        .min_size(vec2(105.0, 28.0));
        if ui
            .add(mask_btn)
            .on_hover_text("Ocultar ou exibir o conteúdo das mensagens na Timeline por privacidade")
            .clicked()
        {
            app.settings.crm_timeline_mask_messages = !masked;
            app.save_settings();
            ui.ctx().request_repaint();
        }
    };

    if avail < 720.0 {
        ui.horizontal_wrapped(|ui| {
            render_type_buttons(ui, &mut type_filter);
            show_mask_toggle(ui, app);
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt(("timeline-client-filter", &account_id))
                .selected_text(if client_filter == "all" {
                    "Todos os clientes".to_owned()
                } else {
                    client_filter.clone()
                })
                .width(150.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut client_filter, "all".into(), "Todos os clientes")
                        .clicked()
                    {
                        ui.ctx().request_repaint();
                    }
                    for c_name in &client_options {
                        if ui
                            .selectable_value(&mut client_filter, c_name.clone(), c_name)
                            .clicked()
                        {
                            ui.ctx().request_repaint();
                        }
                    }
                });

            let search_w = (ui.available_width() - 36.0).max(100.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar…")
                    .desired_width(search_w),
            );

            if !search.is_empty() {
                let clear_btn = egui::Button::new(RichText::new("✕").size(11.0).color(cp.dim))
                    .fill(cp.surface)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(24.0, 24.0));
                if ui.add(clear_btn).clicked() {
                    search.clear();
                    ui.ctx().request_repaint();
                }
            }
        });
    } else {
        ui.horizontal(|ui| {
            render_type_buttons(ui, &mut type_filter);

            ui.add_space(6.0);
            show_mask_toggle(ui, app);

            ui.add_space(6.0);

            egui::ComboBox::from_id_salt(("timeline-client-filter", &account_id))
                .selected_text(if client_filter == "all" {
                    "Todos os clientes".to_owned()
                } else {
                    client_filter.clone()
                })
                .width(150.0)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_value(&mut client_filter, "all".into(), "Todos os clientes")
                        .clicked()
                    {
                        ui.ctx().request_repaint();
                    }
                    for c_name in &client_options {
                        if ui
                            .selectable_value(&mut client_filter, c_name.clone(), c_name)
                            .clicked()
                        {
                            ui.ctx().request_repaint();
                        }
                    }
                });

            ui.add_space(4.0);

            let search_w = (avail - 640.0).clamp(120.0, 240.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar evento…")
                    .desired_width(search_w),
            );

            if !search.is_empty() {
                let clear_btn = egui::Button::new(RichText::new("✕").size(11.0).color(cp.dim))
                    .fill(cp.surface)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(24.0, 24.0));
                if ui.add(clear_btn).clicked() {
                    search.clear();
                    ui.ctx().request_repaint();
                }
            }
        });
    }

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
                let summary = if is_masked {
                    "[Texto da mensagem protegido]".to_string()
                } else {
                    message.summary()
                };
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
                    summary,
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
                    "closed" => {
                        if task.revenue_cents > 0 {
                            Some((
                                format!("Venda: {}", format_currency(task.revenue_cents)),
                                cp.accent,
                            ))
                        } else {
                            Some(("Fechada".into(), cp.accent))
                        }
                    }
                    "lost" => Some(("Perdida".into(), cp.danger)),
                    "no_show" => Some(("No-show".into(), Color32::from_rgb(245, 158, 11))),
                    "cancelled" => Some(("Cancelada".into(), cp.danger)),
                    _ => None,
                };
                let summary = if task.revenue_cents > 0 || task.status == "closed" {
                    let desc = if !task.description.trim().is_empty() {
                        format!(" • {}", task.description)
                    } else if !task.call_result.trim().is_empty() {
                        format!(" • {}", task.call_result)
                    } else {
                        String::new()
                    };
                    if task.revenue_cents > 0 {
                        format!(
                            "💰 Venda fechada: {}{}",
                            format_currency(task.revenue_cents),
                            desc
                        )
                    } else {
                        format!("💰 Venda fechada{}", desc)
                    }
                } else if !task.call_result.trim().is_empty() {
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
    let client_lower = if client_filter == "all" {
        String::new()
    } else {
        client_filter.to_lowercase()
    };
    let filtered: Vec<_> = events
        .into_iter()
        .filter(|e| {
            let matches_type = match type_filter.as_str() {
                "all" => true,
                other => e.kind.filter_category() == other,
            };
            let matches_client = client_lower.is_empty()
                || e.entity.to_lowercase().contains(&client_lower)
                || e.title.to_lowercase().contains(&client_lower);
            let matches_search = search_lower.is_empty()
                || e.entity.to_lowercase().contains(&search_lower)
                || e.title.to_lowercase().contains(&search_lower)
                || e.summary.to_lowercase().contains(&search_lower);
            matches_type && matches_client && matches_search
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
            client_filter = "all".into();
            search.clear();
            ui.ctx().request_repaint();
        }
        ui.data_mut(|d| {
            d.insert_temp(filter_type_id, type_filter);
            d.insert_temp(client_filter_id, client_filter);
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

            ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
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

                ui.add_space(6.0);

                // Right Card Content - vertically bounded to content and responsive to window width
                let card_inner_w = (ui.available_width() - 36.0).max(160.0);
                Frame::new()
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(Margin::symmetric(14, 10))
                    .show(ui, |ui| {
                        ui.set_max_width(card_inner_w);
                        ui.set_width(card_inner_w);
                        ui.vertical(|ui| {
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

                                // Entity pill with truncation so right time is never pushed off
                                if !event.entity.is_empty() {
                                    let entity_max_w = (ui.available_width() - 90.0).max(50.0);
                                    ui.allocate_ui_with_layout(
                                        vec2(entity_max_w, 20.0),
                                        Layout::left_to_right(Align::Center),
                                        |ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&event.entity)
                                                        .size(12.5)
                                                        .strong()
                                                        .color(cp.text),
                                                )
                                                .truncate(),
                                            );
                                        },
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
                                    ui.add_space(10.0);
                                    ui.label(
                                        RichText::new(&event.time_str).size(12.0).color(cp.dim),
                                    );
                                });
                            });

                            // Title if different from entity
                            if !event.title.is_empty() && event.title != event.entity {
                                ui.add_space(3.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&event.title)
                                            .size(13.5)
                                            .strong()
                                            .color(cp.text),
                                    )
                                    .wrap(),
                                );
                            }

                            // Summary
                            if !event.summary.is_empty() {
                                ui.add_space(2.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&event.summary)
                                            .size(12.5)
                                            .color(cp.secondary),
                                    )
                                    .wrap(),
                                );
                            }
                        });
                    });
            });

            ui.add_space(6.0);
        }

        ui.add_space(10.0);
    }

    ui.data_mut(|d| {
        d.insert_temp(filter_type_id, type_filter);
        d.insert_temp(client_filter_id, client_filter);
        d.insert_temp(search_id, search);
    });
}

fn funnel(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette) {
    // 1. Auto-seed default stages if none exist
    if app.crm_columns.is_empty() {
        for col in super::kanban::default_columns() {
            app.actions.push(Action::SaveCrmColumn(col));
        }
    }

    let search_lower = app.crm_search.trim().to_lowercase();
    let mut sorted_cols = app.crm_columns.clone();
    sorted_cols.sort_by_key(|c| c.order);

    // 2. Compute metrics across all deals matching search filter
    let mut total_deals = 0;
    let mut total_value: i64 = 0;
    let mut closed_count = 0;
    let mut closed_value: i64 = 0;
    let last_col_id = sorted_cols.last().map(|c| c.id.clone()).unwrap_or_default();

    for deal in app.crm_deals.values() {
        let chat_name = app
            .chat(&deal.chat_id)
            .map(|c| app.chat_title(c))
            .unwrap_or_else(|| deal.chat_id.clone());
        let is_match = search_lower.is_empty()
            || chat_name.to_lowercase().contains(&search_lower)
            || deal.notes.to_lowercase().contains(&search_lower)
            || deal
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&search_lower));
        if is_match {
            total_deals += 1;
            total_value += deal.value_cents;
            if deal.column_id == last_col_id
                || deal.column_id == "close"
                || deal.column_id == "post"
            {
                closed_count += 1;
                closed_value += deal.value_cents;
            }
        }
    }

    let avg_ticket = if total_deals > 0 {
        total_value / total_deals as i64
    } else {
        0
    };

    // 3. Top KPI Metric Cards
    let metric_width = ((ui.available_width() - 36.0) / 4.0).max(145.0);
    ui.horizontal_wrapped(|ui| {
        metric_str(
            ui,
            cp,
            Icon::Tag,
            "Total Negócios",
            &total_deals.to_string(),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::Tag,
            "Valor no Funil",
            &format_currency(total_value),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::ListChecks,
            "Ticket Médio",
            &format_currency(avg_ticket),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::CircleCheck,
            "Fechamentos",
            &format!("{} ({})", format_currency(closed_value), closed_count),
            metric_width,
        );
    });

    ui.add_space(14.0);

    // 4. Search and Filter Bar
    ui.horizontal(|ui| {
        theme::icon(ui, Icon::Search, 16.0, cp.dim);
        ui.add(
            egui::TextEdit::singleline(&mut app.crm_search)
                .hint_text("Buscar por contato, telefone, notas internas ou etiquetas...")
                .desired_width((ui.available_width() - 60.0).max(200.0)),
        );
        if !app.crm_search.is_empty() {
            if theme::icon_button(ui, Icon::X, 13.0, cp.dim, cp.text, "Limpar busca").clicked() {
                app.crm_search.clear();
            }
        }
    });

    ui.add_space(10.0);

    // Render popovers/modals if open
    render_funnel_popovers(app, ui, cp, &sorted_cols);

    ui.add_space(10.0);

    // 5. Horizontal Columns Board
    ScrollArea::horizontal()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 14.0;
                let num_cols = sorted_cols.len();
                for (col_idx, col) in sorted_cols.iter().enumerate() {
                    render_funnel_column(
                        app,
                        ui,
                        cp,
                        col,
                        col_idx,
                        num_cols,
                        &search_lower,
                        &sorted_cols,
                    );
                }
                ui.add_space(18.0);
            });
        });
}

fn render_funnel_popovers(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette, cols: &[CrmColumn]) {
    let add_deal_id = egui::Id::new("funnel_add_deal_open");
    let add_stage_id = egui::Id::new("funnel_add_stage_open");
    let delete_stage_confirm_id = egui::Id::new("funnel_delete_stage_confirm");

    let add_deal_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(add_deal_id))
        .unwrap_or(false);
    let add_stage_open = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(add_stage_id))
        .unwrap_or(false);
    let delete_col_id: Option<String> = ui
        .ctx()
        .data(|d| d.get_temp(delete_stage_confirm_id))
        .flatten();

    // 1. Modal: Novo Negócio
    if add_deal_open {
        let mut is_open = true;
        egui::Window::new("Novo Negócio no Funil")
            .id(ui.id().with("crm-deal-modal-win"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(440.0, 420.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                render_deal_modal(app, ui, cp, cols, &add_deal_id);
            });
        if !is_open {
            ui.ctx().data_mut(|d| d.insert_temp(add_deal_id, false));
        }
    }

    // 2. Modal: Nova Etapa
    if add_stage_open {
        let mut is_open = true;
        egui::Window::new("Nova Etapa do Funil")
            .id(ui.id().with("crm-stage-modal-win"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(380.0, 240.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                render_stage_modal(app, ui, cp, &add_stage_id);
            });
        if !is_open {
            ui.ctx().data_mut(|d| d.insert_temp(add_stage_id, false));
        }
    }

    // 3. Confirm Delete Stage
    if let Some(col_id) = delete_col_id {
        if let Some(target_col) = app.crm_columns.iter().find(|c| c.id == col_id).cloned() {
            let fallback_title = app
                .crm_columns
                .iter()
                .find(|c| c.id != col_id)
                .map(|c| c.title.clone())
                .unwrap_or_else(|| "outra etapa".to_owned());
            let fallback_id = app
                .crm_columns
                .iter()
                .find(|c| c.id != col_id)
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "lead".to_owned());

            let mut is_open = true;
            egui::Window::new("Excluir Etapa")
                .id(ui.id().with("crm-del-stage-modal"))
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .fixed_size(vec2(380.0, 200.0))
                .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .frame(
                    Frame::new()
                        .fill(cp.panel)
                        .stroke(Stroke::new(1.0, cp.danger))
                        .corner_radius(CornerRadius::same(12))
                        .inner_margin(Margin::same(18)),
                )
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            theme::icon(ui, Icon::CircleAlert, 18.0, cp.danger);
                            ui.label(
                                RichText::new(format!("Excluir etapa '{}'?", target_col.title))
                                    .size(15.0)
                                    .strong()
                                    .color(cp.text),
                            );
                        });
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Os negócios desta etapa serão movidos automaticamente para '{}'.",
                                fallback_title
                            ))
                            .size(13.0)
                            .color(cp.secondary),
                        );
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            let cancel_btn = egui::Button::new(
                                RichText::new("Cancelar").size(13.0).color(cp.text),
                            )
                            .fill(cp.surface_hover)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(vec2(100.0, 32.0));
                            if ui.add(cancel_btn).clicked() {
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(delete_stage_confirm_id, None::<String>)
                                });
                            }

                            ui.add_space(8.0);

                            let del_btn = egui::Button::new(
                                RichText::new("Excluir Etapa")
                                    .size(13.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(cp.danger)
                            .corner_radius(CornerRadius::same(6))
                            .min_size(vec2(120.0, 32.0));
                            if ui.add(del_btn).clicked() {
                                let to_move: Vec<_> = app
                                    .crm_deals
                                    .values()
                                    .filter(|d| d.column_id == col_id)
                                    .cloned()
                                    .collect();
                                for mut d in to_move {
                                    d.column_id = fallback_id.clone();
                                    d.updated_at = crate::util::now();
                                    app.actions.push(Action::SaveCrmDeal(d));
                                }
                                app.actions.push(Action::DeleteCrmColumn(col_id.clone()));
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(delete_stage_confirm_id, None::<String>)
                                });
                            }
                        });
                    });
                });
            if !is_open {
                ui.ctx()
                    .data_mut(|d| d.insert_temp(delete_stage_confirm_id, None::<String>));
            }
        }
    }
}

fn render_deal_modal(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    cols: &[CrmColumn],
    add_deal_id: &egui::Id,
) {
    let sel_chat_id = egui::Id::new("modal_deal_sel_chat");
    let sel_col_id = egui::Id::new("modal_deal_sel_col");
    let val_id = egui::Id::new("modal_deal_val_str");
    let tags_id = egui::Id::new("modal_deal_tags_str");

    let mut selected_chat = ui
        .ctx()
        .data(|d| d.get_temp::<String>(sel_chat_id))
        .unwrap_or_default();
    let mut selected_col = ui
        .ctx()
        .data(|d| d.get_temp::<String>(sel_col_id))
        .unwrap_or_else(|| {
            cols.first()
                .map(|c| c.id.clone())
                .unwrap_or_else(|| "lead".into())
        });
    let mut val_str = ui
        .ctx()
        .data(|d| d.get_temp::<String>(val_id))
        .unwrap_or_default();
    let mut tags_str = ui
        .ctx()
        .data(|d| d.get_temp::<String>(tags_id))
        .unwrap_or_default();

    let search_chat_id = egui::Id::new("modal_deal_search_chat");
    let mut search_chat = ui
        .ctx()
        .data(|d| d.get_temp::<String>(search_chat_id))
        .unwrap_or_default();

    ui.vertical(|ui| {
        ui.label(
            RichText::new("Cliente / Contato:")
                .size(13.0)
                .color(cp.secondary),
        );
        ui.horizontal(|ui| {
            let w = ui.available_width() - if search_chat.is_empty() { 0.0 } else { 24.0 };
            ui.add(
                egui::TextEdit::singleline(&mut search_chat)
                    .hint_text("🔍 Buscar por nome ou número...")
                    .desired_width(w.max(60.0)),
            );
            if !search_chat.is_empty() && ui.small_button("✕").clicked() {
                search_chat.clear();
            }
        });
        ui.add_space(2.0);

        let query = search_chat.trim().to_lowercase();
        let filtered_chats: Vec<_> = app
            .chats
            .iter()
            .filter(|chat| {
                if query.is_empty() {
                    return true;
                }
                let title = app.chat_title(chat).to_lowercase();
                title.contains(&query) || chat.id.to_lowercase().contains(&query)
            })
            .collect();

        let cur_title = if selected_chat.is_empty() {
            if query.is_empty() {
                "Selecionar contato...".to_owned()
            } else {
                format!("Selecionar contato ({} encontrados)...", filtered_chats.len())
            }
        } else {
            app.chat(&selected_chat)
                .map(|c| app.chat_title(c))
                .unwrap_or_else(|| selected_chat.clone())
        };

        egui::ComboBox::from_id_salt("modal_deal_chat_cb")
            .selected_text(cur_title)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                if filtered_chats.is_empty() {
                    ui.label(RichText::new("Nenhum contato encontrado").color(cp.dim).italics());
                } else {
                    for chat in filtered_chats {
                        let title = app.chat_title(chat);
                        ui.selectable_value(&mut selected_chat, chat.id.clone(), title);
                    }
                }
            });

        ui.add_space(8.0);
        ui.label(
            RichText::new("Etapa do Funil:")
                .size(13.0)
                .color(cp.secondary),
        );
        let col_title = cols
            .iter()
            .find(|c| c.id == selected_col)
            .map(|c| c.title.clone())
            .unwrap_or_else(|| "Selecionar etapa".into());

        egui::ComboBox::from_id_salt("modal_deal_col_cb")
            .selected_text(col_title)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                for col in cols {
                    ui.selectable_value(&mut selected_col, col.id.clone(), &col.title);
                }
            });

        ui.add_space(8.0);
        ui.label(RichText::new("Valor em R$:").size(13.0).color(cp.secondary));
        ui.add(
            egui::TextEdit::singleline(&mut val_str)
                .hint_text("ex: 2.500,00")
                .desired_width(ui.available_width() - 8.0),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new("Etiquetas (separadas por vírgula):")
                .size(13.0)
                .color(cp.secondary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut tags_str)
                .hint_text("ex: VIP, Decisor, Urgente")
                .desired_width(ui.available_width() - 8.0),
        );

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let cancel_btn = egui::Button::new(RichText::new("Cancelar").size(13.0).color(cp.text))
                .fill(cp.surface_hover)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(100.0, 34.0));
            if ui.add(cancel_btn).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(*add_deal_id, false));
            }

            ui.add_space(8.0);

            let can_save = !selected_chat.is_empty() && !selected_col.is_empty();
            let save_btn = egui::Button::new(
                RichText::new("Salvar Negócio")
                    .size(13.0)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(140.0, 34.0));

            if ui.add_enabled(can_save, save_btn).clicked() {
                let cents = parse_currency_cents(&val_str).unwrap_or(0);
                let tags: Vec<String> = tags_str
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect();
                let existing_notes = app
                    .crm_deals
                    .get(&selected_chat)
                    .map(|d| d.notes.clone())
                    .unwrap_or_default();

                let deal = CrmDeal {
                    chat_id: selected_chat.clone(),
                    column_id: selected_col.clone(),
                    value_cents: cents,
                    notes: existing_notes,
                    tags,
                    updated_at: crate::util::now(),
                };
                app.actions.push(Action::SaveCrmDeal(deal));

                selected_chat.clear();
                search_chat.clear();
                val_str.clear();
                tags_str.clear();
                ui.ctx().data_mut(|d| d.insert_temp(*add_deal_id, false));
            }
        });
    });

    ui.ctx().data_mut(|d| {
        d.insert_temp(sel_chat_id, selected_chat);
        d.insert_temp(search_chat_id, search_chat);
        d.insert_temp(sel_col_id, selected_col);
        d.insert_temp(val_id, val_str);
        d.insert_temp(tags_id, tags_str);
    });
}

fn render_stage_modal(app: &mut App, ui: &mut egui::Ui, cp: &CrmPalette, add_stage_id: &egui::Id) {
    let name_id = egui::Id::new("modal_stage_name");
    let color_id = egui::Id::new("modal_stage_color");

    let mut name = ui
        .ctx()
        .data(|d| d.get_temp::<String>(name_id))
        .unwrap_or_default();
    let mut color_hex = ui
        .ctx()
        .data(|d| d.get_temp::<String>(color_id))
        .unwrap_or_else(|| "#00e699".to_owned());

    ui.vertical(|ui| {
        ui.label(
            RichText::new("Nome da Etapa:")
                .size(13.0)
                .color(cp.secondary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut name)
                .hint_text("ex: Proposta Enviada")
                .desired_width(ui.available_width() - 8.0),
        );

        ui.add_space(8.0);
        ui.label(
            RichText::new("Cor de Destaque:")
                .size(13.0)
                .color(cp.secondary),
        );
        let colors = [
            ("#00e699", "Neon"),
            ("#3b82f6", "Azul"),
            ("#eab308", "Amarelo"),
            ("#8b5cf6", "Roxo"),
            ("#ff4757", "Vermelho"),
            ("#f97316", "Laranja"),
            ("#6b7280", "Cinza"),
        ];

        ui.horizontal(|ui| {
            for (hex, _lbl) in colors {
                let is_active = color_hex == hex;
                let c = parse_hex_color(hex).unwrap_or(cp.accent);
                let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), egui::Sense::click());
                ui.painter()
                    .circle_filled(r.center(), if is_active { 9.0 } else { 7.0 }, c);
                if is_active {
                    ui.painter()
                        .circle_stroke(r.center(), 11.0, Stroke::new(1.5, cp.text));
                }
                if resp.clicked() {
                    color_hex = hex.to_owned();
                }
            }
        });

        ui.add_space(16.0);
        ui.horizontal(|ui| {
            let cancel_btn = egui::Button::new(RichText::new("Cancelar").size(13.0).color(cp.text))
                .fill(cp.surface_hover)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(90.0, 32.0));
            if ui.add(cancel_btn).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(*add_stage_id, false));
            }

            ui.add_space(8.0);

            let can_save = !name.trim().is_empty();
            let save_btn = egui::Button::new(
                RichText::new("Criar Etapa")
                    .size(13.0)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(120.0, 32.0));

            if ui.add_enabled(can_save, save_btn).clicked() {
                let next_order = app.crm_columns.iter().map(|c| c.order).max().unwrap_or(0) + 1;
                let col = CrmColumn {
                    id: format!("col_{}_{:08x}", crate::util::now(), rand::random::<u32>()),
                    title: name.trim().to_owned(),
                    color: color_hex.clone(),
                    order: next_order,
                };
                app.actions.push(Action::SaveCrmColumn(col));
                name.clear();
                ui.ctx().data_mut(|d| d.insert_temp(*add_stage_id, false));
            }
        });
    });

    ui.ctx().data_mut(|d| {
        d.insert_temp(name_id, name);
        d.insert_temp(color_id, color_hex);
    });
}

fn render_funnel_column(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    col: &CrmColumn,
    col_idx: usize,
    total_cols: usize,
    search_lower: &str,
    sorted_cols: &[CrmColumn],
) {
    let col_color = parse_hex_color(&col.color).unwrap_or(cp.accent);

    // Filter deals in this stage (ensuring orphaned/legacy stage deals are never lost)
    let is_first_col = col_idx == 0;
    let mut stage_deals: Vec<_> = app
        .crm_deals
        .values()
        .filter(|d| {
            let col_match = if d.column_id == col.id {
                true
            } else if is_first_col {
                !sorted_cols.iter().any(|c| c.id == d.column_id)
            } else {
                false
            };
            col_match && {
                if search_lower.is_empty() {
                    true
                } else {
                    let chat_title = app
                        .chat(&d.chat_id)
                        .map(|c| app.chat_title(c))
                        .unwrap_or_else(|| d.chat_id.clone());
                    chat_title.to_lowercase().contains(search_lower)
                        || d.notes.to_lowercase().contains(search_lower)
                        || d.tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(search_lower))
                }
            }
        })
        .cloned()
        .collect();
    stage_deals.sort_by_key(|d| -d.updated_at);

    let stage_value: i64 = stage_deals.iter().map(|d| d.value_cents).sum();
    let col_w = 280.0;

    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(col_w - 24.0);
            ui.vertical(|ui| {
                // Stage Title Row: display title in full without truncate or arrows overlapping
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                    ui.painter().circle_filled(r.center(), 5.0, col_color);
                    ui.add(
                        egui::Label::new(
                            RichText::new(&col.title).size(13.5).strong().color(cp.text),
                        )
                        .wrap(),
                    );
                });

                ui.add_space(6.0);

                // Stage metrics pills & controls: reorder arrows and delete are cleanly separated
                ui.horizontal(|ui| {
                    Frame::new()
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("{} negócios", stage_deals.len()))
                                    .size(11.0)
                                    .color(cp.secondary),
                            );
                        });

                    if stage_value > 0 {
                        Frame::new()
                            .fill(cp.accent.gamma_multiply(0.12))
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format_currency(stage_value))
                                        .size(11.0)
                                        .strong()
                                        .color(cp.accent),
                                );
                            });
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Delete stage button
                        if app.crm_columns.len() > 1 {
                            if theme::icon_button(
                                ui,
                                Icon::Trash,
                                12.0,
                                cp.dim,
                                cp.danger,
                                "Excluir etapa",
                            )
                            .clicked()
                            {
                                let delete_stage_confirm_id =
                                    egui::Id::new("funnel_delete_stage_confirm");
                                ui.ctx().data_mut(|d| {
                                    d.insert_temp(delete_stage_confirm_id, Some(col.id.clone()))
                                });
                            }
                        }

                        // Reorder right
                        if col_idx + 1 < total_cols {
                            if ui
                                .small_button(RichText::new("→").color(cp.secondary))
                                .on_hover_text("Mover etapa para direita")
                                .clicked()
                            {
                                let next_id = sorted_cols[col_idx + 1].id.clone();
                                let mut to_save = Vec::new();
                                for c in &app.crm_columns {
                                    if c.id == col.id {
                                        let mut updated = c.clone();
                                        updated.order += 1;
                                        to_save.push(updated);
                                    } else if c.id == next_id {
                                        let mut updated = c.clone();
                                        updated.order -= 1;
                                        to_save.push(updated);
                                    }
                                }
                                for c in to_save {
                                    app.actions.push(Action::SaveCrmColumn(c));
                                }
                            }
                        }

                        // Reorder left
                        if col_idx > 0 {
                            if ui
                                .small_button(RichText::new("←").color(cp.secondary))
                                .on_hover_text("Mover etapa para esquerda")
                                .clicked()
                            {
                                let prev_id = sorted_cols[col_idx - 1].id.clone();
                                let mut to_save = Vec::new();
                                for c in &app.crm_columns {
                                    if c.id == col.id {
                                        let mut updated = c.clone();
                                        updated.order -= 1;
                                        to_save.push(updated);
                                    } else if c.id == prev_id {
                                        let mut updated = c.clone();
                                        updated.order += 1;
                                        to_save.push(updated);
                                    }
                                }
                                for c in to_save {
                                    app.actions.push(Action::SaveCrmColumn(c));
                                }
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // Deals list
                if stage_deals.is_empty() {
                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Nenhum negócio nesta etapa")
                                .size(12.0)
                                .color(cp.dim),
                        );
                    });
                    ui.add_space(12.0);
                } else {
                    for deal in &stage_deals {
                        render_deal_card(app, ui, cp, deal, col_idx, total_cols, sorted_cols);
                        ui.add_space(8.0);
                    }
                }
            });
        });
}

fn render_deal_card(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    deal: &CrmDeal,
    col_idx: usize,
    total_cols: usize,
    sorted_cols: &[CrmColumn],
) {
    let chat_title = app
        .chat(&deal.chat_id)
        .map(|c| app.chat_title(c))
        .unwrap_or_else(|| deal.chat_id.clone());

    let confirm_deal_delete_id = ui.id().with(("del-deal-conf", &deal.chat_id));
    let mut confirm_del = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(confirm_deal_delete_id))
        .unwrap_or(false);

    Frame::new()
        .fill(cp.panel)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                // Client Name row
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::User, 13.0, cp.accent);
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&chat_title)
                            .size(13.5)
                            .strong()
                            .color(cp.text),
                    );
                });

                ui.add_space(4.0);

                // Value row
                if deal.value_cents > 0 {
                    ui.label(
                        RichText::new(format_currency(deal.value_cents))
                            .size(14.0)
                            .strong()
                            .color(cp.accent),
                    );
                    ui.add_space(2.0);
                }

                // Tags chips
                if !deal.tags.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for tag in &deal.tags {
                            Frame::new()
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(5, 1))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(tag).size(10.5).color(cp.secondary));
                                });
                        }
                    });
                    ui.add_space(4.0);
                }

                // Notes excerpt
                if !deal.notes.is_empty() {
                    let preview = if deal.notes.chars().count() > 50 {
                        format!("{}…", deal.notes.chars().take(48).collect::<String>())
                    } else {
                        deal.notes.clone()
                    };
                    ui.label(RichText::new(preview).size(11.5).color(cp.dim));
                    ui.add_space(6.0);
                }

                // Actions row
                if confirm_del {
                    ui.horizontal(|ui| {
                        let cancel_btn =
                            egui::Button::new(RichText::new("Cancelar").size(11.0).color(cp.text))
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                        if ui.add(cancel_btn).clicked() {
                            confirm_del = false;
                        }

                        let del_btn = egui::Button::new(
                            RichText::new("Excluir").size(11.0).color(Color32::WHITE),
                        )
                        .fill(cp.danger)
                        .corner_radius(CornerRadius::same(4));
                        if ui.add(del_btn).clicked() {
                            app.actions
                                .push(Action::DeleteCrmDeal(deal.chat_id.clone()));
                            confirm_del = false;
                        }
                    });
                } else {
                    ui.horizontal(|ui| {
                        // Move left button
                        if col_idx > 0 {
                            let prev_col_id = &sorted_cols[col_idx - 1].id;
                            if ui
                                .small_button(RichText::new("←").color(cp.secondary))
                                .on_hover_text("Mover para etapa anterior")
                                .clicked()
                            {
                                let mut updated = deal.clone();
                                updated.column_id = prev_col_id.clone();
                                updated.updated_at = crate::util::now();
                                app.actions.push(Action::SaveCrmDeal(updated));
                            }
                        }

                        // Move right button
                        if col_idx + 1 < total_cols {
                            let next_col_id = &sorted_cols[col_idx + 1].id;
                            if ui
                                .small_button(RichText::new("→").color(cp.secondary))
                                .on_hover_text("Avançar para próxima etapa")
                                .clicked()
                            {
                                let mut updated = deal.clone();
                                updated.column_id = next_col_id.clone();
                                updated.updated_at = crate::util::now();
                                app.actions.push(Action::SaveCrmDeal(updated));
                            }
                        }

                        // Open Chat button
                        let chat_btn =
                            egui::Button::new(RichText::new("💬 Chat").size(11.0).color(cp.accent))
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                        if ui.add(chat_btn).on_hover_text("Abrir conversa").clicked() {
                            app.actions.push(Action::OpenChat(deal.chat_id.clone()));
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::icon_button(
                                ui,
                                Icon::Trash,
                                12.0,
                                cp.dim,
                                cp.danger,
                                "Excluir negócio",
                            )
                            .clicked()
                            {
                                confirm_del = true;
                            }
                        });
                    });
                }
            });
        });

    ui.ctx()
        .data_mut(|d| d.insert_temp(confirm_deal_delete_id, confirm_del));
}

fn render_late_actions(
    app: &mut App,
    ui: &mut egui::Ui,
    task: &CrmTask,
    cp: &CrmPalette,
    now: i64,
    today: jiff::civil::Date,
    is_call: bool,
) {
    let render_done = |ui: &mut egui::Ui, app: &mut App| {
        let done_btn = egui::Button::new(
            RichText::new("✓ Realizada")
                .size(11.5)
                .strong()
                .color(Color32::WHITE),
        )
        .fill(Color32::from_rgb(0x10, 0xb9, 0x81))
        .corner_radius(CornerRadius::same(5));
        if ui
            .add(done_btn)
            .on_hover_text("Marcar como realizada")
            .clicked()
        {
            let mut updated = task.clone();
            updated.status = if is_call {
                "held".into()
            } else {
                "done".into()
            };
            if is_call && updated.call_result.is_empty() {
                updated.call_result = "Realizada".into();
            }
            updated.completed_at = Some(now);
            updated.updated_at = now;
            if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                app.crm_tasks[pos] = updated.clone();
            }
            app.actions.push(Action::SaveCrmTask(updated));
            app.toast("Compromisso marcado como realizado!");
            ui.ctx().request_repaint();
        }
    };

    let render_cancel = |ui: &mut egui::Ui, app: &mut App| {
        let cancel_btn = egui::Button::new(RichText::new("✕ Cancelada").size(11.5).color(cp.danger))
            .fill(cp.surface_hover)
            .stroke(Stroke::new(1.0, cp.danger.gamma_multiply(0.4)))
            .corner_radius(CornerRadius::same(5));
        if ui
            .add(cancel_btn)
            .on_hover_text("Marcar como cancelada")
            .clicked()
        {
            let mut updated = task.clone();
            updated.status = "cancelled".into();
            if is_call && updated.call_result.is_empty() {
                updated.call_result = "Cancelada".into();
            }
            updated.completed_at = Some(now);
            updated.updated_at = now;
            if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                app.crm_tasks[pos] = updated.clone();
            }
            app.actions.push(Action::SaveCrmTask(updated));
            app.toast("Compromisso marcado como cancelado.");
            ui.ctx().request_repaint();
        }
    };

    let render_reschedule = |ui: &mut egui::Ui, app: &mut App| {
        ui.menu_button(
            RichText::new("📅 Reagendar ▾")
                .size(11.5)
                .strong()
                .color(cp.accent),
            |ui| {
                if ui.button("Hoje (+1 hora)").clicked() {
                    let mut updated = task.clone();
                    updated.due_at = Some(now + 3600);
                    if is_call {
                        updated.status = "scheduled".into();
                    } else if updated.status == "done" || updated.status == "cancelled" {
                        updated.status = "todo".into();
                    }
                    updated.completed_at = None;
                    updated.updated_at = now;
                    if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                        app.crm_tasks[pos] = updated.clone();
                    }
                    app.actions.push(Action::SaveCrmTask(updated));
                    app.toast("Compromisso reagendado para hoje (+1h)!");
                    ui.close();
                }
                if ui.button("Hoje às 18:00").clicked() {
                    let mut updated = task.clone();
                    let target = crate::util::to_unix_seconds(today, 18, 0).unwrap_or(now + 3600);
                    updated.due_at = Some(target);
                    if is_call {
                        updated.status = "scheduled".into();
                    } else if updated.status == "done" || updated.status == "cancelled" {
                        updated.status = "todo".into();
                    }
                    updated.completed_at = None;
                    updated.updated_at = now;
                    if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                        app.crm_tasks[pos] = updated.clone();
                    }
                    app.actions.push(Action::SaveCrmTask(updated));
                    app.toast("Compromisso reagendado para hoje às 18:00!");
                    ui.close();
                }
                if ui.button("Amanhã às 09:00").clicked() {
                    let mut updated = task.clone();
                    let tomorrow = today.tomorrow().unwrap_or(today);
                    let target = crate::util::to_unix_seconds(tomorrow, 9, 0).unwrap_or(now + 86400);
                    updated.due_at = Some(target);
                    if is_call {
                        updated.status = "scheduled".into();
                    } else if updated.status == "done" || updated.status == "cancelled" {
                        updated.status = "todo".into();
                    }
                    updated.completed_at = None;
                    updated.updated_at = now;
                    if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                        app.crm_tasks[pos] = updated.clone();
                    }
                    app.actions.push(Action::SaveCrmTask(updated));
                    app.toast("Compromisso reagendado para amanhã às 09:00!");
                    ui.close();
                }
                if ui.button("Próxima semana (+7 dias)").clicked() {
                    let mut updated = task.clone();
                    updated.due_at = Some(now + 7 * 86400);
                    if is_call {
                        updated.status = "scheduled".into();
                    } else if updated.status == "done" || updated.status == "cancelled" {
                        updated.status = "todo".into();
                    }
                    updated.completed_at = None;
                    updated.updated_at = now;
                    if let Some(pos) = app.crm_tasks.iter().position(|t| t.id == updated.id) {
                        app.crm_tasks[pos] = updated.clone();
                    }
                    app.actions.push(Action::SaveCrmTask(updated));
                    app.toast("Compromisso reagendado para a próxima semana!");
                    ui.close();
                }
            },
        );
    };

    if ui.layout().prefer_right_to_left() {
        render_reschedule(ui, app);
        ui.add_space(4.0);
        render_cancel(ui, app);
        ui.add_space(4.0);
        render_done(ui, app);
    } else {
        render_done(ui, app);
        ui.add_space(4.0);
        render_cancel(ui, app);
        ui.add_space(4.0);
        render_reschedule(ui, app);
    }
}

fn overview(
    app: &mut App,
    ui: &mut egui::Ui,
    view: &mut WorkspaceView,
    selected: &mut Option<String>,
    cp: &CrmPalette,
) {
    let now = crate::util::now();
    let today = crate::util::today();

    let active_projects: Vec<_> = app
        .crm_projects
        .iter()
        .filter(|p| p.status == "active")
        .cloned()
        .collect();

    let late_tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            !matches!(
                t.status.as_str(),
                "done" | "held" | "closed" | "cancelled" | "lost"
            ) && t.due_at.is_some_and(|x| x < now)
        })
        .cloned()
        .collect();
    let late_count = late_tasks.len();

    let today_tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            t.status != "done"
                && t.kind == "task"
                && t.due_at.is_some_and(|d| {
                    crate::util::local_datetime(d)
                        .map(|(date, _, _)| date == today)
                        .unwrap_or(false)
                })
        })
        .cloned()
        .collect();

    let today_calls: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            matches!(t.kind.as_str(), "call" | "conversation")
                && t.due_at.is_some_and(|d| {
                    crate::util::local_datetime(d)
                        .map(|(date, _, _)| date == today)
                        .unwrap_or(false)
                })
        })
        .cloned()
        .collect();

    let open_followups: Vec<_> = app
        .crm_followups
        .iter()
        .filter(|f| !f.done)
        .cloned()
        .collect();

    // 1. Top Metrics Bar (Responsive wrapping)
    let metric_width = ((ui.available_width() - 36.0) / 4.0).max(145.0);
    ui.horizontal_wrapped(|ui| {
        metric_str(
            ui,
            cp,
            Icon::CircleAlert,
            "Atrasadas",
            &late_count.to_string(),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::Clock,
            "Tarefas Hoje",
            &today_tasks.len().to_string(),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::Phone,
            "Calls Hoje",
            &today_calls.len().to_string(),
            metric_width,
        );
        metric_str(
            ui,
            cp,
            Icon::Users,
            "Projetos Ativos",
            &active_projects.len().to_string(),
            metric_width,
        );
    });

    ui.add_space(16.0);

    // 2. Atalhos Operacionais (Quick Action Shortcuts)
    ui.horizontal_wrapped(|ui| {
        let btn_task = egui::Button::new(RichText::new("＋ Nova Tarefa").size(12.5).color(cp.text))
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .min_size(vec2(130.0, 32.0));
        if ui.add(btn_task).clicked() {
            *view = WorkspaceView::Tasks;
            ui.ctx().request_repaint();
        }

        let btn_call =
            egui::Button::new(RichText::new("📞 Agendar Call").size(12.5).color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(130.0, 32.0));
        if ui.add(btn_call).clicked() {
            *view = WorkspaceView::Calls;
            ui.ctx().request_repaint();
        }

        let btn_agenda =
            egui::Button::new(RichText::new("📅 Ver Agenda").size(12.5).color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(125.0, 32.0));
        if ui.add(btn_agenda).clicked() {
            *view = WorkspaceView::Agenda;
            ui.ctx().request_repaint();
        }

        let btn_funnel =
            egui::Button::new(RichText::new("⊞ Funil Comercial").size(12.5).color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(140.0, 32.0));
        if ui.add(btn_funnel).clicked() {
            *view = WorkspaceView::Funnel;
            ui.ctx().request_repaint();
        }

        let btn_proj =
            egui::Button::new(RichText::new("📁 Meus Projetos").size(12.5).color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .min_size(vec2(130.0, 32.0));
        if ui.add(btn_proj).clicked() {
            *view = WorkspaceView::Projects;
            ui.ctx().request_repaint();
        }
    });

    ui.add_space(20.0);

    let mut to_complete: Option<CrmTask> = None;

    // 3. Destaque de Atrasos (Overdue Items Alert)
    if !late_tasks.is_empty() {
        Frame::new()
            .fill(cp.danger.gamma_multiply(0.08))
            .stroke(Stroke::new(1.0, cp.danger))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleAlert, 18.0, cp.danger);
                    ui.label(
                        RichText::new(format!(
                            "Atenção: {} tarefa(s) ou compromisso(s) em atraso",
                            late_tasks.len()
                        ))
                        .size(15.0)
                        .strong()
                        .color(cp.danger),
                    );
                });

                ui.add_space(8.0);

                for task in late_tasks.iter().take(8) {
                    let is_call = matches!(task.kind.as_str(), "call" | "conversation");
                    let project_name = task
                        .project_id
                        .as_ref()
                        .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                        .map(|p| p.name.clone());
                    let client_name = task
                        .chat_id
                        .as_ref()
                        .and_then(|id| app.chat(id))
                        .map(|c| app.chat_title(c));

                    Frame::new()
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(8))
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            let item_w = ui.available_width();
                            let is_narrow = item_w < 650.0;
                            if is_narrow {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        theme::icon(
                                            ui,
                                            if is_call {
                                                Icon::Phone
                                            } else {
                                                Icon::ListChecks
                                            },
                                            15.0,
                                            cp.danger,
                                        );
                                        ui.add_space(2.0);
                                        ui.label(
                                            RichText::new(&task.title)
                                                .size(13.5)
                                                .strong()
                                                .color(cp.text),
                                        );
                                    });
                                    ui.horizontal_wrapped(|ui| {
                                        if let Some(c_name) = &client_name {
                                            ui.label(
                                                RichText::new(format!("👤 {}", c_name))
                                                    .size(11.5)
                                                    .color(cp.secondary),
                                            );
                                        } else if let Some(p_name) = &project_name {
                                            ui.label(
                                                RichText::new(format!("📁 {}", p_name))
                                                    .size(11.5)
                                                    .color(cp.secondary),
                                            );
                                        }
                                        if let Some(due) = task.due_at {
                                            if let Some((date, h, m)) =
                                                crate::util::local_datetime(due)
                                            {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Venceu em {:02}/{:02} às {:02}:{:02}",
                                                        date.day(),
                                                        date.month(),
                                                        h,
                                                        m
                                                    ))
                                                    .size(11.0)
                                                    .strong()
                                                    .color(cp.danger),
                                                );
                                            }
                                        }
                                    });
                                    ui.add_space(4.0);
                                    ui.horizontal_wrapped(|ui| {
                                        render_late_actions(app, ui, task, cp, now, today, is_call);
                                    });
                                });
                            } else {
                                ui.horizontal(|ui| {
                                    theme::icon(
                                        ui,
                                        if is_call {
                                            Icon::Phone
                                        } else {
                                            Icon::ListChecks
                                        },
                                        15.0,
                                        cp.danger,
                                    );
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(&task.title)
                                            .size(13.5)
                                            .strong()
                                            .color(cp.text),
                                    );

                                    if let Some(c_name) = &client_name {
                                        ui.label(
                                            RichText::new(format!("• 👤 {}", c_name))
                                                .size(12.0)
                                                .color(cp.secondary),
                                        );
                                    } else if let Some(p_name) = &project_name {
                                        ui.label(
                                            RichText::new(format!("• 📁 {}", p_name))
                                                .size(12.0)
                                                .color(cp.secondary),
                                        );
                                    }

                                    if let Some(due) = task.due_at {
                                        if let Some((date, h, m)) = crate::util::local_datetime(due)
                                        {
                                            ui.label(
                                                RichText::new(format!(
                                                    "• Venceu em {:02}/{:02} às {:02}:{:02}",
                                                    date.day(),
                                                    date.month(),
                                                    h,
                                                    m
                                                ))
                                                .size(11.5)
                                                .strong()
                                                .color(cp.danger),
                                            );
                                        }
                                    }

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(14.0);
                                        render_late_actions(app, ui, task, cp, now, today, is_call);
                                    });
                                });
                            }
                        });
                    ui.add_space(6.0);
                }
            });
        ui.add_space(18.0);
    }

    // 4. Agenda do Dia (Programação Completa de Hoje: Tarefas e Calls)
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Agenda do Dia")
                .size(17.0)
                .strong()
                .color(cp.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let see_all = egui::Button::new(
                RichText::new("Ver agenda completa →")
                    .size(12.5)
                    .strong()
                    .color(cp.accent),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6));
            if ui.add(see_all).clicked() {
                *view = WorkspaceView::Agenda;
                ui.ctx().request_repaint();
            }
        });
    });

    ui.add_space(8.0);

    // Combine and sort today's tasks and calls
    let mut today_items: Vec<(Option<(u8, u8)>, bool, CrmTask)> = Vec::new();
    for t in &today_tasks {
        let time = t
            .due_at
            .and_then(crate::util::local_datetime)
            .map(|(_, h, m)| (h, m));
        today_items.push((time, false, t.clone()));
    }
    for c in &today_calls {
        let time = c
            .due_at
            .and_then(crate::util::local_datetime)
            .map(|(_, h, m)| (h, m));
        today_items.push((time, true, c.clone()));
    }
    today_items.sort_by_key(|(time, _, _)| {
        time.map(|(h, m)| h as u32 * 60 + m as u32)
            .unwrap_or(u32::MAX)
    });

    if today_items.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleCheck, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Nenhum compromisso agendado para hoje. Aproveite para planejar novos negócios!")
                            .size(13.0)
                            .color(cp.secondary),
                    );
                });
            });
    } else {
        for (time, is_call, item) in &today_items {
            let project_name = item
                .project_id
                .as_ref()
                .and_then(|id| app.crm_projects.iter().find(|p| &p.id == id))
                .map(|p| p.name.clone());

            let client_name = item
                .chat_id
                .as_ref()
                .and_then(|id| app.chat(id))
                .map(|c| app.chat_title(c));

            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Time label badge
                        let time_str = if let Some((h, m)) = time {
                            format!("{:02}:{:02}", h, m)
                        } else {
                            "Hoje".to_owned()
                        };
                        Frame::new()
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(7, 2))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(time_str).size(11.5).strong().color(cp.accent),
                                );
                            });

                        ui.add_space(6.0);

                        if *is_call {
                            theme::icon(ui, Icon::Phone, 14.0, cp.accent);
                            ui.add_space(2.0);
                            Frame::new()
                                .fill(cp.accent.gamma_multiply(0.12))
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(5, 1))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new("Call").size(10.5).strong().color(cp.accent),
                                    );
                                });
                        } else {
                            let check_btn =
                                egui::Button::new(RichText::new("□").size(14.0).color(cp.accent))
                                    .fill(cp.surface_hover)
                                    .corner_radius(CornerRadius::same(4))
                                    .min_size(vec2(22.0, 22.0));
                            if ui.add(check_btn).on_hover_text("Concluir tarefa").clicked() {
                                let mut updated = item.clone();
                                updated.status = "done".into();
                                updated.completed_at = Some(now);
                                updated.updated_at = now;
                                to_complete = Some(updated);
                                ui.ctx().request_repaint();
                            }
                        }

                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&item.title)
                                .size(13.5)
                                .strong()
                                .color(cp.text),
                        );

                        if let Some(c_name) = &client_name {
                            ui.label(
                                RichText::new(format!("• 👤 {}", c_name))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        } else if let Some(p_name) = &project_name {
                            ui.label(
                                RichText::new(format!("• 📁 {}", p_name))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(16.0);
                            if let Some(chat_id) = &item.chat_id {
                                let chat_btn = egui::Button::new(
                                    RichText::new("💬 Chat").size(11.5).color(cp.accent),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(chat_btn).clicked() {
                                    app.actions.push(Action::OpenChat(chat_id.clone()));
                                    ui.ctx().request_repaint();
                                }
                            }
                            if *is_call {
                                if let Some(dur) = item.duration_minutes {
                                    ui.label(
                                        RichText::new(format!("{} min", dur))
                                            .size(11.5)
                                            .color(cp.dim),
                                    );
                                }
                            } else {
                                let (p_bg, p_fg, p_txt) = match item.priority.as_str() {
                                    "high" => (cp.danger.gamma_multiply(0.15), cp.danger, "Alta"),
                                    "low" => (cp.surface_hover, cp.dim, "Baixa"),
                                    _ => (cp.accent.gamma_multiply(0.12), cp.accent, "Normal"),
                                };
                                Frame::new()
                                    .fill(p_bg)
                                    .corner_radius(CornerRadius::same(4))
                                    .inner_margin(Margin::symmetric(5, 1))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(p_txt).size(10.5).color(p_fg));
                                    });
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }

    ui.add_space(20.0);

    // 5. Resumo de Calls de Venda
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Resumo de Calls de Venda")
                .size(17.0)
                .strong()
                .color(cp.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let btn = egui::Button::new(
                RichText::new("Ver todas as calls →")
                    .size(12.5)
                    .strong()
                    .color(cp.accent),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6));
            if ui.add(btn).clicked() {
                *view = WorkspaceView::Calls;
                ui.ctx().request_repaint();
            }
        });
    });

    ui.add_space(8.0);

    let all_calls_count = app
        .crm_tasks
        .iter()
        .filter(|t| matches!(t.kind.as_str(), "call" | "conversation"))
        .count();
    let closed_calls_count = app
        .crm_tasks
        .iter()
        .filter(|t| {
            matches!(t.kind.as_str(), "call" | "conversation")
                && (t.status == "closed" || t.revenue_cents > 0)
        })
        .count();
    let total_calls_rev: i64 = app
        .crm_tasks
        .iter()
        .filter(|t| matches!(t.kind.as_str(), "call" | "conversation"))
        .map(|t| t.revenue_cents)
        .sum();

    let call_kpi_w = ((ui.available_width() - 24.0) / 3.0).max(130.0);
    ui.horizontal_wrapped(|ui| {
        metric_str(
            ui,
            cp,
            Icon::Phone,
            "Hoje / Total",
            &format!("{}/{}", today_calls.len(), all_calls_count),
            call_kpi_w,
        );
        metric_str(
            ui,
            cp,
            Icon::CircleCheck,
            "Fechamentos",
            &closed_calls_count.to_string(),
            call_kpi_w,
        );
        metric_str(
            ui,
            cp,
            Icon::Tag,
            "Receita de Calls",
            &format_currency(total_calls_rev),
            call_kpi_w,
        );
    });

    ui.add_space(10.0);

    let display_calls: Vec<_> = if !today_calls.is_empty() {
        today_calls.clone()
    } else {
        app.crm_tasks
            .iter()
            .filter(|t| matches!(t.kind.as_str(), "call" | "conversation"))
            .rev()
            .take(4)
            .cloned()
            .collect()
    };

    if display_calls.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Phone, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Nenhuma call cadastrada ainda. Agende sua primeira reunião para alavancar conversões!")
                            .size(13.0)
                            .color(cp.secondary),
                    );
                });
            });
    } else {
        for call in &display_calls {
            let client_name = call
                .chat_id
                .as_ref()
                .and_then(|id| app.chat(id))
                .map(|c| app.chat_title(c));

            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Phone, 14.0, cp.accent);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&call.title)
                                .size(13.5)
                                .strong()
                                .color(cp.text),
                        );
                        if let Some(c_name) = &client_name {
                            ui.label(
                                RichText::new(format!("• 👤 {}", c_name))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }
                        if let Some(dur) = call.duration_minutes {
                            ui.label(
                                RichText::new(format!("• {} min", dur))
                                    .size(11.5)
                                    .color(cp.dim),
                            );
                        }
                        if !call.call_result.is_empty() {
                            Frame::new()
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&call.call_result)
                                            .size(10.5)
                                            .color(cp.secondary),
                                    );
                                });
                        }
                        if call.revenue_cents > 0 {
                            ui.label(
                                RichText::new(format_currency(call.revenue_cents))
                                    .size(12.5)
                                    .strong()
                                    .color(cp.accent),
                            );
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(16.0);
                            if let Some(chat_id) = &call.chat_id {
                                let chat_btn = egui::Button::new(
                                    RichText::new("💬 Chat").size(11.5).color(cp.accent),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(chat_btn).clicked() {
                                    app.actions.push(Action::OpenChat(chat_id.clone()));
                                    ui.ctx().request_repaint();
                                }
                            }
                            if let Some(due) = call.due_at {
                                if let Some((date, h, m)) = crate::util::local_datetime(due) {
                                    let dt_str = if date == today {
                                        format!("Hoje às {:02}:{:02}", h, m)
                                    } else {
                                        format!(
                                            "{:02}/{:02} {:02}:{:02}",
                                            date.day(),
                                            date.month(),
                                            h,
                                            m
                                        )
                                    };
                                    ui.label(RichText::new(dt_str).size(12.0).color(cp.secondary));
                                }
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }

    ui.add_space(20.0);

    // 6. Projetos Ativos
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Projetos Ativos")
                .size(17.0)
                .strong()
                .color(cp.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            let btn = egui::Button::new(
                RichText::new("Ver todos os projetos →")
                    .size(12.5)
                    .strong()
                    .color(cp.accent),
            )
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(6));
            if ui.add(btn).clicked() {
                *view = WorkspaceView::Projects;
                ui.ctx().request_repaint();
            }
        });
    });

    ui.add_space(8.0);

    if active_projects.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Users, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Nenhum projeto ativo no momento. Crie um novo projeto para organizar entregas e clientes.")
                            .size(13.0)
                            .color(cp.secondary),
                    );
                });
            });
    } else {
        let avail_w = ui.available_width();
        let num_cols = if avail_w > 900.0 {
            3
        } else if avail_w > 540.0 {
            2
        } else {
            1
        };
        let gap = 12.0;
        let card_w = ((avail_w - (num_cols as f32 - 1.0) * gap) / num_cols as f32).max(220.0);

        for row_projs in active_projects.chunks(num_cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for proj in row_projs {
                    let total_t = app
                        .crm_tasks
                        .iter()
                        .filter(|t| t.project_id.as_deref() == Some(&proj.id))
                        .count();
                    let done_t = app
                        .crm_tasks
                        .iter()
                        .filter(|t| t.project_id.as_deref() == Some(&proj.id) && t.status == "done")
                        .count();
                    let prog = if total_t == 0 {
                        0.0
                    } else {
                        done_t as f32 / total_t as f32
                    };

                    Frame::new()
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(Margin::same(14))
                        .show(ui, |ui| {
                            ui.set_width(card_w - 28.0);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    let (status_bg, status_fg, status_txt) = match proj
                                        .status
                                        .as_str()
                                    {
                                        "active" => {
                                            (cp.accent.gamma_multiply(0.15), cp.accent, "Ativo")
                                        }
                                        "paused" => {
                                            (cp.warning.gamma_multiply(0.15), cp.warning, "Pausado")
                                        }
                                        _ => (cp.surface_hover, cp.dim, "Arquivado"),
                                    };
                                    Frame::new()
                                        .fill(status_bg)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(status_txt)
                                                    .size(10.5)
                                                    .strong()
                                                    .color(status_fg),
                                            );
                                        });

                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        let open_btn = egui::Button::new(
                                            RichText::new("Abrir →")
                                                .size(11.5)
                                                .strong()
                                                .color(cp.accent),
                                        )
                                        .fill(cp.surface_hover)
                                        .corner_radius(CornerRadius::same(5));
                                        if ui.add(open_btn).clicked() {
                                            *selected = Some(proj.id.clone());
                                            *view = WorkspaceView::Projects;
                                            ui.ctx().request_repaint();
                                        }
                                    });
                                });

                                ui.add_space(6.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&proj.name)
                                            .size(14.5)
                                            .strong()
                                            .color(cp.text),
                                    )
                                    .truncate(),
                                );

                                if !proj.client_name.is_empty() {
                                    ui.add_space(2.0);
                                    ui.horizontal(|ui| {
                                        theme::icon(ui, Icon::User, 12.0, cp.secondary);
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&proj.client_name)
                                                    .size(12.0)
                                                    .color(cp.secondary),
                                            )
                                            .truncate(),
                                        );
                                        if !proj.client_phone.is_empty() {
                                            ui.label(
                                                RichText::new(format!("• {}", proj.client_phone))
                                                    .size(11.0)
                                                    .color(cp.dim),
                                            );
                                        }
                                    });
                                }

                                if let Some(due) = proj.due_at {
                                    if let Some((d, _, _)) = crate::util::local_datetime(due) {
                                        let is_late = due < now && proj.status != "completed";
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            theme::icon(
                                                ui,
                                                Icon::Clock,
                                                12.0,
                                                if is_late { cp.danger } else { cp.dim },
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "Prazo: {:02}/{:02}/{}",
                                                    d.day(),
                                                    d.month(),
                                                    d.year()
                                                ))
                                                .size(11.5)
                                                .color(if is_late {
                                                    cp.danger
                                                } else {
                                                    cp.secondary
                                                }),
                                            );
                                        });
                                    }
                                }

                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(format!("{done_t}/{total_t} tarefas"))
                                            .size(11.0)
                                            .color(cp.dim),
                                    );
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.label(
                                            RichText::new(format!("{:.0}%", prog * 100.0))
                                                .size(11.0)
                                                .strong()
                                                .color(cp.accent),
                                        );
                                    });
                                });
                                ui.add_space(2.0);
                                ui.add_sized(
                                    [ui.available_width(), 4.0],
                                    egui::ProgressBar::new(prog).fill(cp.accent).animate(false),
                                );
                            });
                        });
                }
            });
            ui.add_space(gap);
        }
    }

    ui.add_space(20.0);

    // 7. Próximos Follow-ups
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Próximos Follow-ups")
                .size(17.0)
                .strong()
                .color(cp.text),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            ui.label(
                RichText::new(format!("{} pendente(s)", open_followups.len()))
                    .size(12.5)
                    .color(cp.dim),
            );
        });
    });

    ui.add_space(8.0);

    if open_followups.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Check, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Sem lembretes de follow-up pendentes.")
                            .size(13.0)
                            .color(cp.secondary),
                    );
                });
            });
    } else {
        for followup in open_followups.iter().take(4) {
            let chat_title = app
                .chat(&followup.chat_id)
                .map(|c| app.chat_title(c))
                .unwrap_or_else(|| followup.chat_id.clone());

            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Bell, 14.0, cp.accent);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&followup.title)
                                .size(13.5)
                                .strong()
                                .color(cp.text),
                        );
                        ui.label(
                            RichText::new(format!("• {}", chat_title))
                                .size(12.0)
                                .color(cp.secondary),
                        );

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.add_space(16.0);
                            let open_btn = egui::Button::new(
                                RichText::new("Abrir Chat").size(11.5).color(cp.accent),
                            )
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4));
                            if ui.add(open_btn).clicked() {
                                app.actions.push(Action::OpenChat(followup.chat_id.clone()));
                                ui.ctx().request_repaint();
                            }

                            let complete_btn = egui::Button::new(
                                RichText::new("✓ Concluir").size(11.5).color(cp.text),
                            )
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4));
                            if ui.add(complete_btn).clicked() {
                                app.actions
                                    .push(Action::CompleteCrmFollowup(followup.id.clone()));
                                ui.ctx().request_repaint();
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }

    if let Some(t) = to_complete {
        app.actions.push(Action::SaveCrmTask(t));
        ui.ctx().request_repaint();
    }
}

fn metric_str(
    ui: &mut egui::Ui,
    palette: &CrmPalette,
    icon: Icon,
    title: &str,
    value: &str,
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
            ui.label(RichText::new(value).size(22.0).strong().color(palette.text));
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

    let avail = ui.available_width();
    if avail < 560.0 {
        ui.horizontal(|ui| {
            let search_w = (avail - 110.0).max(100.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar projeto…")
                    .desired_width(search_w),
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
        });
        ui.add_space(6.0);
        let btn = egui::Button::new(
            RichText::new("+ Novo projeto")
                .size(13.5)
                .strong()
                .color(cp.on_accent),
        )
        .fill(cp.accent)
        .corner_radius(CornerRadius::same(8))
        .min_size(vec2(140.0, 34.0));

        if ui.add(btn).clicked() {
            show_form = true;
        }
    } else {
        ui.horizontal(|ui| {
            let search_w = (avail - 290.0).clamp(160.0, 280.0);
            ui.add(
                egui::TextEdit::singleline(&mut search)
                    .hint_text("Buscar projeto…")
                    .desired_width(search_w),
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
    }

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

                // Client row
                let client_label = if !project.client_name.is_empty() {
                    project.client_name.clone()
                } else {
                    "Não informado".to_string()
                };
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Users, 13.0, cp.dim);
                    ui.label(RichText::new("Cliente:").size(11.5).color(cp.dim));
                    ui.label(RichText::new(client_label).size(11.5).color(
                        if project.client_name.is_empty() {
                            cp.dim
                        } else {
                            cp.text
                        },
                    ));
                });

                // Due Date row
                let now = crate::util::now();
                let is_overdue = project
                    .due_at
                    .is_some_and(|d| d < now && project.status != "completed");
                ui.horizontal(|ui| {
                    theme::icon(
                        ui,
                        Icon::Clock,
                        13.0,
                        if is_overdue { cp.danger } else { cp.dim },
                    );
                    ui.label(RichText::new("Prazo:").size(11.5).color(if is_overdue {
                        cp.danger
                    } else {
                        cp.dim
                    }));
                    if let Some((date, _, _)) = project.due_at.and_then(crate::util::local_datetime)
                    {
                        let date_str = crate::util::long_date(app.locale, date);
                        let text = if is_overdue {
                            format!("{} (Atrasado)", date_str)
                        } else {
                            date_str
                        };
                        ui.label(
                            RichText::new(text)
                                .size(11.5)
                                .strong()
                                .color(if is_overdue { cp.danger } else { cp.secondary }),
                        );
                    } else {
                        ui.label(RichText::new("Sem prazo").size(11.5).color(cp.dim));
                    }
                });

                // Assignee row
                let assignee_label = if !project.assignee.is_empty() {
                    project.assignee.clone()
                } else {
                    "Não atribuído".to_string()
                };
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::User, 13.0, cp.dim);
                    ui.label(RichText::new("Responsável:").size(11.5).color(cp.dim));
                    ui.label(RichText::new(assignee_label).size(11.5).color(
                        if project.assignee.is_empty() {
                            cp.dim
                        } else {
                            cp.secondary
                        },
                    ));
                });

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

        let proj_contact_search_id = ui.id().with("modal-proj-contact-search");
        let mut proj_contact_search = ui.data_mut(|d| d.get_temp::<String>(proj_contact_search_id).unwrap_or_default());

        ui.horizontal(|ui| {
            ui.label(RichText::new("Vincular cliente do ZapFast:").size(11.5).color(cp.secondary));
            let w = (ui.available_width() - 30.0).clamp(120.0, 220.0);
            ui.add(
                egui::TextEdit::singleline(&mut proj_contact_search)
                    .hint_text("🔍 Buscar contato...")
                    .desired_width(w),
            );
            if !proj_contact_search.is_empty() && ui.small_button("✕").clicked() {
                proj_contact_search.clear();
            }
        });
        let q_proj = proj_contact_search.trim().to_lowercase();
        let proj_chats: Vec<_> = app
            .chats
            .iter()
            .filter(|chat| {
                if q_proj.is_empty() {
                    return true;
                }
                let title = app.chat_title(chat).to_lowercase();
                title.contains(&q_proj) || chat.id.to_lowercase().contains(&q_proj)
            })
            .collect();

        let mut pick_c: Option<String> = None;
        let proj_cb_text = if draft.client_name.is_empty() {
            if q_proj.is_empty() { "Preencher com contato do ZapFast...".to_owned() } else { format!("Contatos encontrados ({})", proj_chats.len()) }
        } else {
            format!("Cliente selecionado: {}", draft.client_name)
        };

        egui::ComboBox::from_id_salt("modal-proj-contact-cb")
            .selected_text(proj_cb_text)
            .width(ui.available_width() - 8.0)
            .show_ui(ui, |ui| {
                if proj_chats.is_empty() {
                    ui.label(RichText::new("Nenhum contato encontrado").color(cp.dim).italics());
                } else {
                    for chat in proj_chats {
                        let title = app.chat_title(chat);
                        if ui.selectable_value(&mut pick_c, Some(chat.id.clone()), &title).clicked() {
                            draft.client_name = title;
                            let phone_part = chat.id.split('@').next().unwrap_or("").to_string();
                            if !phone_part.is_empty() && phone_part.chars().all(|c| c.is_ascii_digit()) {
                                draft.client_phone = format!("+{}", phone_part);
                            }
                        }
                    }
                }
            });
        ui.data_mut(|d| d.insert_temp(proj_contact_search_id, proj_contact_search));

        ui.add_space(8.0);

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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ProjectDetailTab {
    #[default]
    Overview,
    Tasks,
    Calls,
    Timeline,
}

fn project_detail(
    app: &mut App,
    ui: &mut egui::Ui,
    mut project: CrmProject,
    selected: &mut Option<String>,
    cp: &CrmPalette,
) {
    let confirm_id = ui.id().with(("crm-delete-project", &project.id));
    let edit_id = ui.id().with(("crm-edit-project", &project.id));
    let tab_id = ui.id().with(("crm-project-tab", &project.id));

    let mut confirm_delete = ui
        .ctx()
        .data(|d| d.get_temp::<bool>(confirm_id))
        .unwrap_or(false);
    let mut edit = ui.data_mut(|d| {
        d.get_temp::<ProjectEdit>(edit_id)
            .unwrap_or_else(|| ProjectEdit::from_project(&project))
    });
    let mut current_tab = ui
        .ctx()
        .data(|d| d.get_temp::<ProjectDetailTab>(tab_id))
        .unwrap_or_default();

    // 1. Top Header Bar (Responsive: never clips action buttons, never overlaps scrollbar)
    let header_avail = ui.available_width();
    if header_avail < 640.0 {
        ui.horizontal(|ui| {
            let back_btn =
                egui::Button::new(RichText::new("← Projetos").size(12.5).color(cp.secondary))
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(85.0, 30.0));
            if ui.add(back_btn).clicked() {
                *selected = None;
                ui.ctx().request_repaint();
            }

            ui.add_space(4.0);

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

            ui.add_space(4.0);
            let title_w = (header_avail - 190.0).max(80.0);
            ui.allocate_ui_with_layout(
                vec2(title_w, 28.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&project.name)
                                .size(16.0)
                                .strong()
                                .color(cp.text),
                        )
                        .truncate(),
                    );
                },
            );
        });
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let edit_btn = egui::Button::new(RichText::new("✎ Editar").size(12.5).color(cp.text))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(85.0, 30.0));
            if ui.add(edit_btn).clicked() {
                edit.open = true;
                ui.ctx().request_repaint();
            }

            ui.add_space(6.0);

            let del_btn = egui::Button::new(RichText::new("🗑 Excluir").size(12.5).color(cp.danger))
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(85.0, 30.0));
            if ui.add(del_btn).clicked() {
                confirm_delete = true;
                ui.ctx().request_repaint();
            }
        });
    } else {
        ui.horizontal(|ui| {
            let back_btn =
                egui::Button::new(RichText::new("← Projetos").size(13.0).color(cp.secondary))
                    .fill(cp.surface)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(95.0, 32.0));
            if ui.add(back_btn).clicked() {
                *selected = None;
                ui.ctx().request_repaint();
            }

            ui.add_space(6.0);

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
                .inner_margin(Margin::symmetric(8, 3))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(status_txt)
                            .size(12.0)
                            .strong()
                            .color(status_fg),
                    );
                });

            ui.add_space(6.0);

            // Reserve 220px for the right buttons + 190px for back_btn and status
            let title_max_w = (header_avail - 420.0).max(100.0);
            ui.allocate_ui_with_layout(
                vec2(title_max_w, 32.0),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&project.name)
                                .size(18.0)
                                .strong()
                                .color(cp.text),
                        )
                        .truncate(),
                    );
                    if project.kind == "internal" {
                        ui.add_space(4.0);
                        Frame::new()
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4))
                            .inner_margin(Margin::symmetric(6, 2))
                            .show(ui, |ui| {
                                ui.label(RichText::new("Interno").size(11.0).color(cp.secondary));
                            });
                    }
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0); // Safe right margin away from vertical scrollbar!

                // Delete project button
                let del_btn =
                    egui::Button::new(RichText::new("🗑 Excluir").size(12.5).color(cp.danger))
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(vec2(85.0, 32.0));
                if ui.add(del_btn).clicked() {
                    confirm_delete = true;
                    ui.ctx().request_repaint();
                }

                ui.add_space(6.0);

                // Edit project button
                let edit_btn =
                    egui::Button::new(RichText::new("✎ Editar").size(12.5).color(cp.text))
                        .fill(cp.surface)
                        .stroke(Stroke::new(1.0, cp.outline))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(vec2(85.0, 32.0));
                if ui.add(edit_btn).clicked() {
                    edit.open = true;
                    ui.ctx().request_repaint();
                }
            });
        });
    }

    // Fluxo Style KPI & Metadata Summary Card (Responsive: 4 cols or 2x2 grid)
    let total_tasks = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id))
        .count();
    let done_tasks = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.status == "done")
        .count();
    let proj_progress = if total_tasks == 0 {
        0.0
    } else {
        done_tasks as f32 / total_tasks as f32
    };

    ui.add_space(10.0);
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            let card_avail = ui.available_width();

            let render_col_cliente = |ui: &mut egui::Ui, w: f32, app: &mut App| {
                ui.allocate_ui_with_layout(vec2(w, 68.0), Layout::top_down(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Users, 14.0, cp.accent);
                        ui.label(RichText::new("CLIENTE").size(10.5).strong().color(cp.dim));
                    });
                    ui.add_space(4.0);
                    let c_name = if !project.client_name.is_empty() {
                        &project.client_name
                    } else {
                        "Não informado"
                    };
                    ui.add(
                        egui::Label::new(RichText::new(c_name).size(13.5).strong().color(
                            if project.client_name.is_empty() {
                                cp.dim
                            } else {
                                cp.text
                            },
                        ))
                        .truncate(),
                    );
                    if !project.client_phone.is_empty() {
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&project.client_phone)
                                        .size(11.0)
                                        .color(cp.secondary),
                                )
                                .truncate(),
                            );
                            let chat_target = project
                                .chat_ids
                                .first()
                                .cloned()
                                .unwrap_or_else(|| project.client_phone.clone());
                            let wa_btn = egui::Button::new(
                                RichText::new("💬 Chat").size(10.5).color(cp.accent),
                            )
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4));
                            if ui.add(wa_btn).clicked() {
                                app.actions.push(Action::OpenChat(chat_target));
                                ui.ctx().request_repaint();
                            }
                        });
                    } else if !project.client_email.is_empty() {
                        ui.add(
                            egui::Label::new(
                                RichText::new(&project.client_email)
                                    .size(10.5)
                                    .color(cp.dim),
                            )
                            .truncate(),
                        );
                    }
                });
            };

            let render_col_prazo = |ui: &mut egui::Ui, w: f32, app: &App| {
                ui.allocate_ui_with_layout(vec2(w, 68.0), Layout::top_down(Align::Min), |ui| {
                    let now = crate::util::now();
                    let is_overdue = project
                        .due_at
                        .is_some_and(|d| d < now && project.status != "completed");
                    ui.horizontal(|ui| {
                        theme::icon(
                            ui,
                            Icon::Clock,
                            14.0,
                            if is_overdue { cp.danger } else { cp.accent },
                        );
                        ui.label(
                            RichText::new("PRAZO")
                                .size(10.5)
                                .strong()
                                .color(if is_overdue { cp.danger } else { cp.dim }),
                        );
                    });
                    ui.add_space(4.0);
                    if let Some((date, _, _)) = project.due_at.and_then(crate::util::local_datetime)
                    {
                        let date_str = crate::util::long_date(app.locale, date);
                        let (txt, color) = if is_overdue {
                            (format!("{} (Atrasado)", date_str), cp.danger)
                        } else {
                            (date_str, cp.text)
                        };
                        ui.add(
                            egui::Label::new(RichText::new(txt).size(13.0).strong().color(color))
                                .truncate(),
                        );
                    } else {
                        ui.label(RichText::new("Sem prazo").size(13.0).color(cp.dim));
                    }
                    if let Some((s_date, _, _)) =
                        project.start_at.and_then(crate::util::local_datetime)
                    {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!(
                                    "Início: {}",
                                    crate::util::long_date(app.locale, s_date)
                                ))
                                .size(11.0)
                                .color(cp.dim),
                            )
                            .truncate(),
                        );
                    }
                });
            };

            let render_col_responsavel = |ui: &mut egui::Ui, w: f32| {
                ui.allocate_ui_with_layout(vec2(w, 68.0), Layout::top_down(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::User, 14.0, cp.accent);
                        ui.label(
                            RichText::new("RESPONSÁVEL")
                                .size(10.5)
                                .strong()
                                .color(cp.dim),
                        );
                    });
                    ui.add_space(4.0);
                    let resp = if !project.assignee.is_empty() {
                        &project.assignee
                    } else {
                        "Não atribuído"
                    };
                    ui.add(
                        egui::Label::new(RichText::new(resp).size(13.5).strong().color(
                            if project.assignee.is_empty() {
                                cp.dim
                            } else {
                                cp.text
                            },
                        ))
                        .truncate(),
                    );
                    ui.label(RichText::new("Responsável direto").size(11.0).color(cp.dim));
                });
            };

            let render_col_progresso = |ui: &mut egui::Ui, w: f32| {
                ui.allocate_ui_with_layout(vec2(w, 68.0), Layout::top_down(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::ListChecks, 14.0, cp.accent);
                        ui.label(RichText::new("PROGRESSO").size(10.5).strong().color(cp.dim));
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!(
                            "{done_tasks}/{total_tasks} ({:.0}%)",
                            proj_progress * 100.0
                        ))
                        .size(13.0)
                        .strong()
                        .color(cp.text),
                    );
                    ui.add_space(4.0);
                    ui.add_sized(
                        [ui.available_width(), 5.0],
                        egui::ProgressBar::new(proj_progress)
                            .fill(cp.accent)
                            .animate(false),
                    );
                });
            };

            if card_avail < 640.0 {
                let half_w = ((card_avail - 16.0) / 2.0).max(120.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        render_col_cliente(ui, half_w, app);
                        ui.add_space(8.0);
                        render_col_prazo(ui, half_w, app);
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        render_col_responsavel(ui, half_w);
                        ui.add_space(8.0);
                        render_col_progresso(ui, half_w);
                    });
                });
            } else {
                let col_w = ((card_avail - 40.0) / 4.0).max(110.0);
                ui.horizontal(|ui| {
                    render_col_cliente(ui, col_w, app);

                    ui.painter().vline(
                        ui.cursor().left(),
                        ui.cursor().top()..=(ui.cursor().top() + 60.0),
                        Stroke::new(1.0, cp.outline),
                    );
                    ui.add_space(8.0);

                    render_col_prazo(ui, col_w, app);

                    ui.painter().vline(
                        ui.cursor().left(),
                        ui.cursor().top()..=(ui.cursor().top() + 60.0),
                        Stroke::new(1.0, cp.outline),
                    );
                    ui.add_space(8.0);

                    render_col_responsavel(ui, col_w);

                    ui.painter().vline(
                        ui.cursor().left(),
                        ui.cursor().top()..=(ui.cursor().top() + 60.0),
                        Stroke::new(1.0, cp.outline),
                    );
                    ui.add_space(8.0);

                    render_col_progresso(ui, col_w);
                });
            }
        });

    // 2. Standardized Deletion Confirmation Banner
    if confirm_delete {
        ui.add_space(10.0);
        Frame::new()
            .fill(cp.danger.gamma_multiply(0.08))
            .stroke(Stroke::new(1.0, cp.danger))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(12))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::CircleAlert, 18.0, cp.danger);
                    ui.label(
                        RichText::new(format!(
                            "Deseja realmente excluir o projeto '{}'? Todas as tarefas associadas perderão o vínculo.",
                            project.name
                        ))
                        .size(13.5)
                        .strong()
                        .color(cp.text),
                    );
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let cancel_btn = egui::Button::new(
                        RichText::new("Cancelar").size(12.5).color(cp.text),
                    )
                    .fill(cp.surface_hover)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(90.0, 30.0));
                    if ui.add(cancel_btn).clicked() {
                        confirm_delete = false;
                    }

                    ui.add_space(8.0);

                    let del_confirm = egui::Button::new(
                        RichText::new("Excluir Projeto")
                            .size(12.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(cp.danger)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(120.0, 30.0));
                    if ui.add(del_confirm).clicked() {
                        app.actions.push(Action::DeleteCrmProject(project.id.clone()));
                        *selected = None;
                        confirm_delete = false;
                    }
                });
            });
    }

    ui.ctx()
        .data_mut(|d| d.insert_temp(confirm_id, confirm_delete));

    // 3. Edit Project Floating Modal Window
    if edit.open {
        let mut is_open = true;
        egui::Window::new("Editar Projeto")
            .id(ui.id().with(("edit-proj-modal", &project.id)))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .fixed_size(vec2(480.0, 480.0))
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .frame(
                Frame::new()
                    .fill(cp.panel)
                    .stroke(Stroke::new(1.0, cp.outline))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(18)),
            )
            .show(ui.ctx(), |ui| {
                render_edit_project_modal(app, ui, cp, &mut project, &mut edit);
            });
        if !is_open {
            edit.open = false;
        }
    }
    ui.data_mut(|d| d.insert_temp(edit_id, edit));

    ui.add_space(14.0);

    // 4. Sub-Navigation Tabs Bar
    let project_tasks_count = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.kind == "task")
        .count();
    let project_calls_count = app
        .crm_tasks
        .iter()
        .filter(|t| {
            t.project_id.as_deref() == Some(&project.id)
                && matches!(t.kind.as_str(), "call" | "conversation")
        })
        .count();

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.spacing_mut().item_spacing.y = 6.0;

        let tabs = [
            (ProjectDetailTab::Overview, "📋 Resumo".to_owned()),
            (
                ProjectDetailTab::Tasks,
                format!("⊞ Tarefas ({project_tasks_count})"),
            ),
            (
                ProjectDetailTab::Calls,
                format!("📞 Calls ({project_calls_count})"),
            ),
            (ProjectDetailTab::Timeline, "🕒 Timeline".to_owned()),
        ];

        for (tab_type, tab_label) in tabs {
            let is_active = current_tab == tab_type;
            let tab_btn = egui::Button::new(RichText::new(&tab_label).size(13.0).strong().color(
                if is_active {
                    cp.on_accent
                } else {
                    cp.secondary
                },
            ))
            .fill(if is_active { cp.accent } else { cp.surface })
            .stroke(Stroke::new(
                1.0,
                if is_active { cp.accent } else { cp.outline },
            ))
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(120.0, 32.0));

            if ui.add(tab_btn).clicked() {
                current_tab = tab_type;
                ui.ctx().request_repaint();
            }
        }
    });

    ui.ctx().data_mut(|d| d.insert_temp(tab_id, current_tab));

    ui.add_space(14.0);

    // 5. Active Tab Body Content
    match current_tab {
        ProjectDetailTab::Overview => {
            render_project_tab_overview(app, ui, cp, &mut project, tab_id);
        }
        ProjectDetailTab::Tasks => {
            render_project_tab_tasks(app, ui, cp, &project);
        }
        ProjectDetailTab::Calls => {
            render_project_tab_calls(app, ui, cp, &project);
        }
        ProjectDetailTab::Timeline => {
            render_project_tab_timeline(app, ui, cp, &project);
        }
    }
}

fn render_edit_project_modal(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &mut CrmProject,
    edit: &mut ProjectEdit,
) {
    ui.vertical(|ui| {
        ui.label(
            RichText::new("Nome do Projeto:")
                .size(13.0)
                .color(cp.secondary),
        );
        ui.add(
            egui::TextEdit::singleline(&mut edit.name)
                .hint_text("ex: Implantação CRM")
                .desired_width(ui.available_width() - 8.0),
        );

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tipo:").size(13.0).color(cp.secondary));
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

            ui.add_space(10.0);
            ui.checkbox(&mut edit.has_due, "Definir prazo");
        });

        ui.add_space(6.0);
        ui.label(RichText::new("Descrição:").size(13.0).color(cp.secondary));
        ui.add(
            egui::TextEdit::multiline(&mut edit.description)
                .hint_text("Objetivos e escopo do projeto...")
                .desired_width(ui.available_width() - 8.0)
                .desired_rows(2),
        );

        ui.add_space(6.0);
        ui.columns(2, |uis| {
            uis[0].label(RichText::new("Responsável:").size(12.5).color(cp.secondary));
            uis[0].add(
                egui::TextEdit::singleline(&mut edit.assignee)
                    .hint_text("Nome do consultor")
                    .desired_width(uis[0].available_width() - 4.0),
            );

            uis[1].label(RichText::new("Cliente:").size(12.5).color(cp.secondary));
            uis[1].add(
                egui::TextEdit::singleline(&mut edit.client_name)
                    .hint_text("Nome da empresa/cliente")
                    .desired_width(uis[1].available_width() - 4.0),
            );
        });

        ui.add_space(6.0);
        ui.columns(2, |uis| {
            uis[0].label(
                RichText::new("Telefone / WhatsApp:")
                    .size(12.5)
                    .color(cp.secondary),
            );
            uis[0].add(
                egui::TextEdit::singleline(&mut edit.client_phone)
                    .hint_text("+55...")
                    .desired_width(uis[0].available_width() - 4.0),
            );

            uis[1].label(RichText::new("E-mail:").size(12.5).color(cp.secondary));
            uis[1].add(
                egui::TextEdit::singleline(&mut edit.client_email)
                    .hint_text("contato@empresa.com")
                    .desired_width(uis[1].available_width() - 4.0),
            );
        });

        ui.add_space(6.0);

        // Data de Início: Calendar only opens on button click!
        let start_cal_id = ui.id().with(("proj-edit-start-cal", &project.id));
        let mut start_cal_open = ui
            .ctx()
            .data(|d| d.get_temp::<bool>(start_cal_id))
            .unwrap_or(false);

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Data de início:")
                    .size(13.0)
                    .color(cp.secondary),
            );
            let start_str = format!(
                "📅 {:02}/{:02}/{}",
                edit.start_date.day(),
                edit.start_date.month(),
                edit.start_date.year()
            );
            if ui
                .button(RichText::new(start_str).color(cp.accent))
                .clicked()
            {
                start_cal_open = !start_cal_open;
            }
        });

        if start_cal_open {
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
                        "project-edit-start-date",
                        &mut edit.start_date,
                        &mut edit.start_hour,
                        &mut edit.start_minute,
                    );
                });
        }
        ui.ctx()
            .data_mut(|d| d.insert_temp(start_cal_id, start_cal_open));

        // Prazo: Calendar only opens on button click!
        if edit.has_due {
            ui.add_space(4.0);
            let due_cal_id = ui.id().with(("proj-edit-due-cal", &project.id));
            let mut due_cal_open = ui
                .ctx()
                .data(|d| d.get_temp::<bool>(due_cal_id))
                .unwrap_or(false);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Prazo de entrega:")
                        .size(13.0)
                        .color(cp.secondary),
                );
                let due_str = format!(
                    "📅 {:02}/{:02}/{}",
                    edit.due_date.day(),
                    edit.due_date.month(),
                    edit.due_date.year()
                );
                if ui.button(RichText::new(due_str).color(cp.accent)).clicked() {
                    due_cal_open = !due_cal_open;
                }
            });

            if due_cal_open {
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
                            "project-edit-due-date",
                            &mut edit.due_date,
                            &mut edit.due_hour,
                            &mut edit.due_minute,
                        );
                    });
            }
            ui.ctx()
                .data_mut(|d| d.insert_temp(due_cal_id, due_cal_open));
        }

        ui.add_space(14.0);

        ui.horizontal(|ui| {
            let cancel_btn = egui::Button::new(RichText::new("Cancelar").size(13.0).color(cp.text))
                .fill(cp.surface_hover)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(100.0, 32.0));
            if ui.add(cancel_btn).clicked() {
                edit.open = false;
            }

            ui.add_space(8.0);

            let can_save = !edit.name.trim().is_empty();
            let save_btn = egui::Button::new(
                RichText::new("Salvar Alterações")
                    .size(13.0)
                    .strong()
                    .color(cp.on_accent),
            )
            .fill(cp.accent)
            .corner_radius(CornerRadius::same(6))
            .min_size(vec2(140.0, 32.0));

            if ui.add_enabled(can_save, save_btn).clicked() {
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
    });
}

fn render_project_tab_overview(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &mut CrmProject,
    tab_id: egui::Id,
) {
    let total_tasks = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.kind == "task")
        .count();
    let done_tasks = app
        .crm_tasks
        .iter()
        .filter(|t| {
            t.project_id.as_deref() == Some(&project.id) && t.kind == "task" && t.status == "done"
        })
        .count();
    let progress_pct = if total_tasks == 0 {
        0.0
    } else {
        done_tasks as f32 / total_tasks as f32
    };

    // 1. Progress Bar Card
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                theme::icon(ui, Icon::ListChecks, 16.0, cp.accent);
                ui.label(
                    RichText::new("Progresso Geral do Projeto")
                        .size(14.0)
                        .strong()
                        .color(cp.text),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{done_tasks}/{total_tasks} tarefas ({}%)",
                            (progress_pct * 100.0) as i32
                        ))
                        .size(12.5)
                        .strong()
                        .color(cp.accent),
                    );
                });
            });

            ui.add_space(8.0);

            let (bar_rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 8.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(bar_rect, CornerRadius::same(4), cp.surface_hover);
            if progress_pct > 0.0 {
                let mut filled_rect = bar_rect;
                filled_rect.set_width(bar_rect.width() * progress_pct);
                ui.painter()
                    .rect_filled(filled_rect, CornerRadius::same(4), cp.accent);
            }
        });

    ui.add_space(14.0);

    // 2. Project Details Grid Card
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Informações do Projeto")
                        .size(14.5)
                        .strong()
                        .color(cp.text),
                );
                ui.add_space(10.0);

                ui.columns(2, |uis| {
                    uis[0].vertical(|ui| {
                        if !project.client_name.is_empty() {
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::User, 14.0, cp.accent);
                                ui.label(
                                    RichText::new(format!("Cliente: {}", project.client_name))
                                        .size(13.5)
                                        .strong()
                                        .color(cp.text),
                                );
                            });
                            ui.add_space(4.0);
                        }
                        if !project.assignee.is_empty() {
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::User, 14.0, cp.secondary);
                                ui.label(
                                    RichText::new(format!("Responsável: {}", project.assignee))
                                        .size(13.0)
                                        .color(cp.secondary),
                                );
                            });
                            ui.add_space(4.0);
                        }
                        if !project.client_phone.is_empty() {
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::Phone, 14.0, cp.accent);
                                ui.label(
                                    RichText::new(&project.client_phone)
                                        .size(13.0)
                                        .strong()
                                        .color(cp.text),
                                );
                                let copy_btn = egui::Button::new(
                                    RichText::new("📋 Copiar").size(11.0).color(cp.secondary),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(copy_btn).clicked() {
                                    ui.ctx().copy_text(project.client_phone.clone());
                                    ui.ctx().request_repaint();
                                }
                                if let Some(first_chat) = project.chat_ids.first() {
                                    let chat_btn = egui::Button::new(
                                        RichText::new("💬 WhatsApp").size(11.0).color(cp.accent),
                                    )
                                    .fill(cp.surface_hover)
                                    .corner_radius(CornerRadius::same(4));
                                    if ui.add(chat_btn).clicked() {
                                        app.actions.push(Action::OpenChat(first_chat.clone()));
                                        ui.ctx().request_repaint();
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }
                        if !project.client_email.is_empty() {
                            ui.horizontal(|ui| {
                                theme::icon(ui, Icon::Send, 14.0, cp.accent);
                                ui.label(
                                    RichText::new(&project.client_email)
                                        .size(13.0)
                                        .color(cp.secondary),
                                );
                                let copy_btn = egui::Button::new(
                                    RichText::new("📋 Copiar").size(11.0).color(cp.secondary),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(copy_btn).clicked() {
                                    ui.ctx().copy_text(project.client_email.clone());
                                    ui.ctx().request_repaint();
                                }
                            });
                            ui.add_space(4.0);
                        }
                        if project.client_phone.is_empty() && project.client_email.is_empty() {
                            ui.label(
                                RichText::new("Nenhum contato do cliente cadastrado")
                                    .size(12.0)
                                    .italics()
                                    .color(cp.dim),
                            );
                        }
                    });

                    uis[1].vertical(|ui| {
                        if let Some((d, _, _)) =
                            project.start_at.and_then(crate::util::local_datetime)
                        {
                            ui.label(
                                RichText::new(format!(
                                    "Início: {:02}/{:02}/{}",
                                    d.day(),
                                    d.month(),
                                    d.year()
                                ))
                                .size(13.0)
                                .color(cp.secondary),
                            );
                            ui.add_space(4.0);
                        }

                        if let Some(due) = project.due_at {
                            if let Some((d, _, _)) = crate::util::local_datetime(due) {
                                let is_late =
                                    due < crate::util::now() && project.status != "completed";
                                ui.label(
                                    RichText::new(format!(
                                        "Prazo: {:02}/{:02}/{}",
                                        d.day(),
                                        d.month(),
                                        d.year()
                                    ))
                                    .size(13.0)
                                    .strong()
                                    .color(if is_late {
                                        cp.danger
                                    } else {
                                        cp.accent
                                    }),
                                );
                                ui.add_space(4.0);
                            }
                        }

                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Situação:").size(13.0).color(cp.secondary));
                            let before = project.status.clone();
                            egui::ComboBox::from_id_salt(("proj-status-quick", &project.id))
                                .selected_text(project_status(&project.status))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut project.status,
                                        "active".into(),
                                        "Ativo",
                                    );
                                    ui.selectable_value(
                                        &mut project.status,
                                        "paused".into(),
                                        "Pausado",
                                    );
                                    ui.selectable_value(
                                        &mut project.status,
                                        "completed".into(),
                                        "Concluído",
                                    );
                                    ui.selectable_value(
                                        &mut project.status,
                                        "archived".into(),
                                        "Arquivado",
                                    );
                                });
                            if project.status != before {
                                project.updated_at = crate::util::now();
                                app.actions.push(Action::SaveCrmProject(project.clone()));
                            }
                        });
                    });
                });

                if !project.description.is_empty() {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Descrição do Projeto:")
                            .size(12.5)
                            .color(cp.secondary),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(&project.description)
                            .size(13.0)
                            .color(cp.text),
                    );
                }
            });
        });

    ui.add_space(14.0);

    // 3. Tarefas do Cliente / Projeto
    let project_tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.kind == "task")
        .cloned()
        .collect();
    let (done_tasks, pending_tasks): (Vec<_>, Vec<_>) =
        project_tasks.into_iter().partition(|t| t.status == "done");

    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::ListChecks, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Tarefas do Cliente / Projeto")
                            .size(14.5)
                            .strong()
                            .color(cp.text),
                    );
                    ui.label(
                        RichText::new(format!(
                            "({} pendentes • {} concluídas)",
                            pending_tasks.len(),
                            done_tasks.len()
                        ))
                        .size(12.5)
                        .color(cp.secondary),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let view_tasks_btn = egui::Button::new(
                            RichText::new("Ver todas as tarefas →")
                                .size(12.0)
                                .color(cp.accent),
                        )
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(4));
                        if ui.add(view_tasks_btn).clicked() {
                            ui.ctx()
                                .data_mut(|d| d.insert_temp(tab_id, ProjectDetailTab::Tasks));
                            ui.ctx().request_repaint();
                        }
                    });
                });

                ui.add_space(10.0);

                // Quick Add inline
                let quick_task_id = ui.id().with(("overview-quick-task", &project.id));
                let mut quick_task = ui
                    .ctx()
                    .data(|d| d.get_temp::<String>(quick_task_id))
                    .unwrap_or_default();
                ui.horizontal(|ui| {
                    let task_input_w = (ui.available_width() - 130.0).max(120.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut quick_task)
                            .hint_text("Adicionar nova tarefa...")
                            .desired_width(task_input_w),
                    );
                    let add_btn = egui::Button::new(
                        RichText::new("＋ Adicionar")
                            .size(12.0)
                            .strong()
                            .color(cp.on_accent),
                    )
                    .fill(cp.accent)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(110.0, 28.0));
                    if ui
                        .add_enabled(!quick_task.trim().is_empty(), add_btn)
                        .clicked()
                    {
                        let nonce = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_nanos();
                        let now = crate::util::now();
                        app.actions.push(Action::SaveCrmTask(CrmTask {
                            id: format!("task-{nonce}"),
                            chat_id: project.chat_ids.first().cloned(),
                            project_id: Some(project.id.clone()),
                            title: quick_task.trim().into(),
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
                        quick_task.clear();
                        ui.ctx().request_repaint();
                    }
                });
                ui.ctx()
                    .data_mut(|d| d.insert_temp(quick_task_id, quick_task));

                ui.add_space(8.0);

                if pending_tasks.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma tarefa pendente neste projeto.")
                            .size(13.0)
                            .color(cp.dim),
                    );
                } else {
                    for task in pending_tasks.iter().take(5) {
                        render_project_task_row(app, ui, cp, task, true);
                        ui.add_space(4.0);
                    }
                    if pending_tasks.len() > 5 {
                        ui.label(
                            RichText::new(format!(
                                "+ {} outras tarefas pendentes",
                                pending_tasks.len() - 5
                            ))
                            .size(12.0)
                            .color(cp.secondary),
                        );
                    }
                }
            });
        });

    ui.add_space(14.0);

    // 4. Calls & Reuniões do Cliente
    let project_calls: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            t.project_id.as_deref() == Some(&project.id)
                && matches!(t.kind.as_str(), "call" | "conversation")
        })
        .cloned()
        .collect();

    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Phone, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Calls & Reuniões do Cliente")
                            .size(14.5)
                            .strong()
                            .color(cp.text),
                    );
                    ui.label(
                        RichText::new(format!("({} registradas)", project_calls.len()))
                            .size(12.5)
                            .color(cp.secondary),
                    );

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let view_calls_btn = egui::Button::new(
                            RichText::new("Ver todas as calls →")
                                .size(12.0)
                                .color(cp.accent),
                        )
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(4));
                        if ui.add(view_calls_btn).clicked() {
                            ui.ctx()
                                .data_mut(|d| d.insert_temp(tab_id, ProjectDetailTab::Calls));
                            ui.ctx().request_repaint();
                        }
                    });
                });

                ui.add_space(10.0);

                if project_calls.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Nenhuma call ou reunião registrada para este projeto.")
                                .size(13.0)
                                .color(cp.dim),
                        );
                        let schedule_btn = egui::Button::new(
                            RichText::new("＋ Agendar Call").size(12.0).color(cp.accent),
                        )
                        .fill(cp.surface_hover)
                        .corner_radius(CornerRadius::same(4));
                        if ui.add(schedule_btn).clicked() {
                            ui.ctx()
                                .data_mut(|d| d.insert_temp(tab_id, ProjectDetailTab::Calls));
                            ui.ctx().request_repaint();
                        }
                    });
                } else {
                    for call in project_calls.iter().take(4) {
                        Frame::new()
                            .fill(cp.panel)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(6))
                            .inner_margin(Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    theme::icon(ui, Icon::Phone, 14.0, cp.accent);
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new(&call.title)
                                            .size(13.0)
                                            .strong()
                                            .color(cp.text),
                                    );
                                    if let Some(dur) = call.duration_minutes {
                                        ui.label(
                                            RichText::new(format!("• {dur} min"))
                                                .size(12.0)
                                                .color(cp.secondary),
                                        );
                                    }
                                    if !call.call_result.is_empty() {
                                        Frame::new()
                                            .fill(cp.surface_hover)
                                            .corner_radius(CornerRadius::same(4))
                                            .inner_margin(Margin::symmetric(6, 2))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(&call.call_result)
                                                        .size(11.0)
                                                        .color(cp.secondary),
                                                );
                                            });
                                    }
                                    if call.revenue_cents > 0 {
                                        ui.label(
                                            RichText::new(format_currency(call.revenue_cents))
                                                .size(12.0)
                                                .strong()
                                                .color(cp.accent),
                                        );
                                    }
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        if let Some(chat_id) = &call.chat_id {
                                            let chat_btn = egui::Button::new(
                                                RichText::new("💬 Chat")
                                                    .size(11.0)
                                                    .color(cp.accent),
                                            )
                                            .fill(cp.surface_hover)
                                            .corner_radius(CornerRadius::same(4));
                                            if ui.add(chat_btn).clicked() {
                                                app.actions.push(Action::OpenChat(chat_id.clone()));
                                                ui.ctx().request_repaint();
                                            }
                                        }
                                        if let Some(due) = call.due_at {
                                            if let Some((d, h, m)) =
                                                crate::util::local_datetime(due)
                                            {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:02}/{:02} {:02}:{:02}",
                                                        d.day(),
                                                        d.month(),
                                                        h,
                                                        m
                                                    ))
                                                    .size(11.5)
                                                    .color(cp.secondary),
                                                );
                                            }
                                        }
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }
                    if project_calls.len() > 4 {
                        ui.label(
                            RichText::new(format!("+ {} outras calls", project_calls.len() - 4))
                                .size(12.0)
                                .color(cp.secondary),
                        );
                    }
                }
            });
        });

    ui.add_space(14.0);

    // 5. Linked Chats Card
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::MessageCircle, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Conversas Vinculadas")
                            .size(14.5)
                            .strong()
                            .color(cp.text),
                    );
                });

                ui.add_space(8.0);

                if project.chat_ids.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma conversa do WhatsApp vinculada a este projeto.")
                            .size(13.0)
                            .color(cp.dim),
                    );
                } else {
                    let mut unbind_id: Option<String> = None;
                    for chat_id in &project.chat_ids {
                        let title = app
                            .chat(chat_id)
                            .map(|chat| app.chat_title(chat))
                            .unwrap_or_else(|| chat_id.clone());

                        ui.horizontal(|ui| {
                            theme::icon(ui, Icon::User, 14.0, cp.secondary);
                            ui.label(RichText::new(&title).size(13.5).strong().color(cp.text));

                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let unbind_btn = egui::Button::new(
                                    RichText::new("✕ Desvincular").size(11.5).color(cp.dim),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(unbind_btn).clicked() {
                                    unbind_id = Some(chat_id.clone());
                                }

                                let chat_btn = egui::Button::new(
                                    RichText::new("💬 Abrir Conversa")
                                        .size(11.5)
                                        .color(cp.accent),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(chat_btn).clicked() {
                                    app.actions.push(Action::OpenChat(chat_id.clone()));
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }

                    if let Some(target) = unbind_id {
                        project.chat_ids.retain(|id| id != &target);
                        project.updated_at = crate::util::now();
                        app.actions.push(Action::SaveCrmProject(project.clone()));
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(8.0);

                // Quick link controls
                ui.horizontal(|ui| {
                    if let Some(current_chat) = app.current_chat().cloned() {
                        let is_already_linked = project.chat_ids.contains(&current_chat.id);
                        if !is_already_linked {
                            let link_open_btn = egui::Button::new(
                                RichText::new("＋ Vincular Conversa Atual")
                                    .size(12.0)
                                    .color(cp.text),
                            )
                            .fill(cp.surface_hover)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(6));
                            if ui.add(link_open_btn).clicked() {
                                project.chat_ids.push(current_chat.id.clone());
                                project.updated_at = crate::util::now();
                                app.actions.push(Action::SaveCrmProject(project.clone()));
                            }
                        }
                    }

                    let pick_chat_id = ui.id().with(("proj-pick-chat-add", &project.id));
                    let search_chat_id = ui.id().with(("proj-pick-chat-search", &project.id));
                    let mut pick_chat = ui
                        .ctx()
                        .data(|d| d.get_temp::<String>(pick_chat_id))
                        .unwrap_or_default();
                    let mut search_chat = ui
                        .ctx()
                        .data(|d| d.get_temp::<String>(search_chat_id))
                        .unwrap_or_default();

                    ui.horizontal(|ui| {
                        let w: f32 = 150.0 - if search_chat.is_empty() { 0.0 } else { 22.0 };
                        ui.add(
                            egui::TextEdit::singleline(&mut search_chat)
                                .hint_text("🔍 Buscar...")
                                .desired_width(w.max(60.0)),
                        );
                        if !search_chat.is_empty() && ui.small_button("✕").clicked() {
                            search_chat.clear();
                        }
                    });

                    let q = search_chat.trim().to_lowercase();
                    let available_chats: Vec<_> = app
                        .chats
                        .iter()
                        .filter(|c| !project.chat_ids.contains(&c.id))
                        .filter(|c| {
                            if q.is_empty() {
                                return true;
                            }
                            let title = app.chat_title(c).to_lowercase();
                            title.contains(&q) || c.id.to_lowercase().contains(&q)
                        })
                        .collect();

                    let pick_title = if pick_chat.is_empty() {
                        if q.is_empty() {
                            "Vincular outro contato...".to_owned()
                        } else {
                            format!("Contatos ({})...", available_chats.len())
                        }
                    } else {
                        app.chat(&pick_chat)
                            .map(|c| app.chat_title(c))
                            .unwrap_or_else(|| pick_chat.clone())
                    };

                    egui::ComboBox::from_id_salt(("proj-link-chat-cb", &project.id))
                        .selected_text(pick_title)
                        .width(200.0)
                        .show_ui(ui, |ui| {
                            if available_chats.is_empty() {
                                ui.label(RichText::new("Nenhum contato encontrado").color(cp.dim).italics());
                            } else {
                                for chat in available_chats {
                                    let title = app.chat_title(chat);
                                    ui.selectable_value(&mut pick_chat, chat.id.clone(), title);
                                }
                            }
                        });

                    if !pick_chat.is_empty() {
                        let add_btn = egui::Button::new(
                            RichText::new("Vincular")
                                .size(12.0)
                                .strong()
                                .color(cp.on_accent),
                        )
                        .fill(cp.accent)
                        .corner_radius(CornerRadius::same(6));
                        if ui.add(add_btn).clicked() {
                            project.chat_ids.push(pick_chat.clone());
                            project.updated_at = crate::util::now();
                            app.actions.push(Action::SaveCrmProject(project.clone()));
                            pick_chat.clear();
                            search_chat.clear();
                        }
                    }
                    ui.ctx().data_mut(|d| {
                        d.insert_temp(pick_chat_id, pick_chat);
                        d.insert_temp(search_chat_id, search_chat);
                    });
                });
            });
        });
}

fn render_project_tab_tasks(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &CrmProject,
) {
    let now = crate::util::now();
    let new_task_id = ui.id().with(("new-project-task", &project.id));
    let mut new_task = ui
        .ctx()
        .data(|d| d.get_temp::<String>(new_task_id))
        .unwrap_or_default();

    // 1. Quick Add Task
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(12))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut new_task)
                        .hint_text("Adicionar nova tarefa para este projeto...")
                        .desired_width(ui.available_width() - 150.0),
                );

                let add_btn = egui::Button::new(
                    RichText::new("＋ Adicionar")
                        .size(12.5)
                        .strong()
                        .color(cp.on_accent),
                )
                .fill(cp.accent)
                .corner_radius(CornerRadius::same(6))
                .min_size(vec2(130.0, 30.0));

                if ui
                    .add_enabled(!new_task.trim().is_empty(), add_btn)
                    .clicked()
                {
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
        });
    ui.ctx().data_mut(|d| d.insert_temp(new_task_id, new_task));

    ui.add_space(14.0);

    // 2. Project Tasks List
    let project_tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.kind == "task")
        .cloned()
        .collect();

    let (done_tasks, pending_tasks): (Vec<_>, Vec<_>) =
        project_tasks.into_iter().partition(|t| t.status == "done");

    if pending_tasks.is_empty() && done_tasks.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Nenhuma tarefa cadastrada neste projeto.")
                        .size(13.0)
                        .color(cp.dim),
                );
            });
    } else {
        // Pending tasks
        for task in &pending_tasks {
            render_project_task_row(app, ui, cp, task, true);
            ui.add_space(6.0);
        }

        // Completed tasks
        if !done_tasks.is_empty() {
            ui.add_space(10.0);
            ui.label(
                RichText::new(format!("Concluídas ({})", done_tasks.len()))
                    .size(13.0)
                    .color(cp.secondary),
            );
            ui.add_space(4.0);
            for task in &done_tasks {
                render_project_task_row(app, ui, cp, task, false);
                ui.add_space(6.0);
            }
        }
    }

    ui.add_space(16.0);

    // 3. Unlinked Tasks Section
    let unassigned_tasks: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.is_none() && t.kind == "task")
        .cloned()
        .collect();

    if !unassigned_tasks.is_empty() {
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new("Vincular tarefa avulsa a este projeto:")
                .size(13.5)
                .strong()
                .color(cp.secondary),
        );
        ui.add_space(6.0);

        for mut task in unassigned_tasks.into_iter().take(6) {
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&task.title).size(13.0).color(cp.text));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let bind_btn = egui::Button::new(
                                RichText::new("＋ Vincular").size(11.5).color(cp.accent),
                            )
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4));
                            if ui.add(bind_btn).clicked() {
                                task.project_id = Some(project.id.clone());
                                task.updated_at = now;
                                app.actions.push(Action::SaveCrmTask(task));
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
    }
}

fn render_project_task_row(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    task: &CrmTask,
    is_pending: bool,
) {
    let now = crate::util::now();
    let is_late = is_pending && task.due_at.is_some_and(|d| d < now);

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
                    RichText::new(if is_pending { "□" } else { "✓" })
                        .size(14.0)
                        .color(if is_pending { cp.accent } else { cp.dim }),
                )
                .fill(cp.surface_hover)
                .corner_radius(CornerRadius::same(4))
                .min_size(vec2(24.0, 24.0));

                if ui.add(check_btn).clicked() {
                    let mut updated = task.clone();
                    if is_pending {
                        updated.status = "done".into();
                        updated.completed_at = Some(now);
                    } else {
                        updated.status = "todo".into();
                        updated.completed_at = None;
                    }
                    updated.updated_at = now;
                    app.actions.push(Action::SaveCrmTask(updated));
                }

                ui.add_space(6.0);

                // Priority Badge
                let (p_bg, p_fg, p_txt) = match task.priority.as_str() {
                    "high" => (cp.danger.gamma_multiply(0.15), cp.danger, "Alta"),
                    "low" => (cp.surface_hover, cp.dim, "Baixa"),
                    _ => (cp.accent.gamma_multiply(0.12), cp.accent, "Normal"),
                };
                Frame::new()
                    .fill(p_bg)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(5, 1))
                    .show(ui, |ui| {
                        ui.label(RichText::new(p_txt).size(10.5).color(p_fg));
                    });

                ui.add_space(4.0);

                ui.label(
                    RichText::new(&task.title)
                        .size(13.5)
                        .strong()
                        .color(if is_pending { cp.text } else { cp.dim }),
                );

                if !task.assignee.is_empty() {
                    ui.label(
                        RichText::new(format!("• {}", task.assignee))
                            .size(12.0)
                            .color(cp.secondary),
                    );
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let unbind_btn =
                        egui::Button::new(RichText::new("✕ Desvincular").size(11.0).color(cp.dim))
                            .fill(cp.surface_hover)
                            .corner_radius(CornerRadius::same(4));
                    if ui.add(unbind_btn).clicked() {
                        let mut updated = task.clone();
                        updated.project_id = None;
                        updated.updated_at = now;
                        app.actions.push(Action::SaveCrmTask(updated));
                    }

                    if let Some(due) = task.due_at {
                        if let Some((d, _, _)) = crate::util::local_datetime(due) {
                            ui.label(
                                RichText::new(format!("{:02}/{:02}", d.day(), d.month()))
                                    .size(12.0)
                                    .color(if is_late { cp.danger } else { cp.dim }),
                            );
                        }
                    }
                });
            });
        });
}

fn render_project_tab_calls(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &CrmProject,
) {
    let now = crate::util::now();
    let conversation_id = ui.id().with(("project-conversation", &project.id));
    let mut conversation = ui.data_mut(|d| {
        d.get_temp::<ConversationDraft>(conversation_id)
            .unwrap_or_default()
    });

    // 1. Schedule Call Card
    Frame::new()
        .fill(cp.surface)
        .stroke(Stroke::new(1.0, cp.outline))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    theme::icon(ui, Icon::Phone, 16.0, cp.accent);
                    ui.label(
                        RichText::new("Agendar Call ou Reunião")
                            .size(14.0)
                            .strong()
                            .color(cp.text),
                    );
                });

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut conversation.title)
                            .hint_text("Assunto da reunião...")
                            .desired_width(260.0),
                    );

                    egui::ComboBox::from_id_salt(("proj-call-duration", &project.id))
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

                    // Date & Time button: calendar strictly opens on-demand!
                    let cal_open_id = ui.id().with(("proj-call-cal-open", &project.id));
                    let mut cal_open = ui
                        .ctx()
                        .data(|d| d.get_temp::<bool>(cal_open_id))
                        .unwrap_or(false);

                    let date_str = format!(
                        "📅 {:02}/{:02}/{} às {:02}:{:02}",
                        conversation.date.day(),
                        conversation.date.month(),
                        conversation.date.year(),
                        conversation.hour,
                        conversation.minute
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
                            .fill(cp.panel)
                            .stroke(Stroke::new(1.0, cp.outline))
                            .corner_radius(CornerRadius::same(8))
                            .inner_margin(Margin::same(8))
                            .show(ui, |ui| {
                                super::dialogs::date_time_picker(
                                    ui,
                                    app.locale,
                                    &app.palette,
                                    "project-conversation-date",
                                    &mut conversation.date,
                                    &mut conversation.hour,
                                    &mut conversation.minute,
                                );
                            });
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(cal_open_id, cal_open));

                    let add_call_btn = egui::Button::new(
                        RichText::new("Agendar Call")
                            .size(12.5)
                            .strong()
                            .color(cp.on_accent),
                    )
                    .fill(cp.accent)
                    .corner_radius(CornerRadius::same(6))
                    .min_size(vec2(120.0, 30.0));

                    if ui
                        .add_enabled(!conversation.title.trim().is_empty(), add_call_btn)
                        .clicked()
                    {
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
                            call_result: "Agendada".into(),
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
            });
        });
    ui.data_mut(|d| d.insert_temp(conversation_id, conversation));

    ui.add_space(14.0);

    // 2. Project Calls List
    let calls: Vec<_> = app
        .crm_tasks
        .iter()
        .filter(|t| {
            t.project_id.as_deref() == Some(&project.id)
                && matches!(t.kind.as_str(), "call" | "conversation")
        })
        .cloned()
        .collect();

    if calls.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("Nenhuma call ou reunião registrada para este projeto.")
                        .size(13.0)
                        .color(cp.dim),
                );
            });
    } else {
        for call in &calls {
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Phone, 15.0, cp.accent);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&call.title)
                                .size(13.5)
                                .strong()
                                .color(cp.text),
                        );

                        if let Some(dur) = call.duration_minutes {
                            ui.label(
                                RichText::new(format!("• {dur} min"))
                                    .size(12.0)
                                    .color(cp.secondary),
                            );
                        }

                        if !call.call_result.is_empty() {
                            Frame::new()
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(6, 2))
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(&call.call_result)
                                            .size(11.0)
                                            .color(cp.secondary),
                                    );
                                });
                        }

                        if call.revenue_cents > 0 {
                            ui.label(
                                RichText::new(format_currency(call.revenue_cents))
                                    .size(12.5)
                                    .strong()
                                    .color(cp.accent),
                            );
                        }

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if let Some(chat_id) = &call.chat_id {
                                let chat_btn = egui::Button::new(
                                    RichText::new("💬 Chat").size(11.5).color(cp.accent),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(chat_btn).clicked() {
                                    app.actions.push(Action::OpenChat(chat_id.clone()));
                                }
                            }

                            if let Some(due) = call.due_at {
                                if let Some((d, h, m)) = crate::util::local_datetime(due) {
                                    ui.label(
                                        RichText::new(format!(
                                            "{:02}/{:02} {:02}:{:02}",
                                            d.day(),
                                            d.month(),
                                            h,
                                            m
                                        ))
                                        .size(12.0)
                                        .color(cp.secondary),
                                    );
                                }
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }
    }
}

fn message_preview(content: &crate::model::Content) -> String {
    match content {
        crate::model::Content::Text { text, .. } => text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(140)
            .collect(),
        crate::model::Content::Interactive { text, .. } => text
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(140)
            .collect(),
        crate::model::Content::Image { caption, .. } => caption
            .as_deref()
            .unwrap_or("📷 Foto")
            .chars()
            .take(140)
            .collect(),
        crate::model::Content::Video { caption, .. } => caption
            .as_deref()
            .unwrap_or("🎥 Vídeo")
            .chars()
            .take(140)
            .collect(),
        crate::model::Content::Audio { voice_note, .. } => {
            if *voice_note {
                "🎤 Mensagem de voz".into()
            } else {
                "🎵 Áudio".into()
            }
        }
        crate::model::Content::Document { file_name, .. } => format!("📄 {file_name}"),
        crate::model::Content::Sticker { .. } => "🏷️ Figurinha".into(),
        crate::model::Content::Location { name, .. } => {
            format!("📍 {}", name.as_deref().unwrap_or("Localização"))
        }
        crate::model::Content::Contact { display_name, .. } => {
            format!("👤 Contato: {display_name}")
        }
        crate::model::Content::Poll { question, .. } => format!("📊 Enquete: {question}"),
        _ => "Mensagem".into(),
    }
}

fn render_project_tab_timeline(
    app: &mut App,
    ui: &mut egui::Ui,
    cp: &CrmPalette,
    project: &CrmProject,
) {
    let filter_id = ui.id().with(("proj-timeline-filter", &project.id));
    let search_id = ui.id().with(("proj-timeline-search", &project.id));
    let mut current_filter = ui
        .ctx()
        .data(|d| d.get_temp::<String>(filter_id))
        .unwrap_or_else(|| "all".to_string());
    let mut search_query = ui
        .ctx()
        .data(|d| d.get_temp::<String>(search_id))
        .unwrap_or_default();

    struct TimelineItem {
        timestamp: i64,
        category: &'static str,
        icon: Icon,
        icon_bg: Color32,
        icon_fg: Color32,
        title: String,
        subtitle: Option<String>,
        description: Option<String>,
        tag: Option<(String, Color32, Color32)>,
        chat_id: Option<String>,
    }

    let mut events: Vec<TimelineItem> = Vec::new();

    // 1. Milestones
    if let Some(start) = project.start_at {
        events.push(TimelineItem {
            timestamp: start,
            category: "milestones",
            icon: Icon::Calendar,
            icon_bg: cp.accent.gamma_multiply(0.15),
            icon_fg: cp.accent,
            title: "Início Oficial do Projeto".into(),
            subtitle: Some(format!(
                "Cliente: {}",
                if project.client_name.is_empty() {
                    "Não informado"
                } else {
                    &project.client_name
                }
            )),
            description: None,
            tag: Some(("Marco".into(), cp.surface_hover, cp.accent)),
            chat_id: None,
        });
    }

    if let Some(due) = project.due_at {
        let is_overdue = due < crate::util::now() && project.status != "completed";
        events.push(TimelineItem {
            timestamp: due,
            category: "milestones",
            icon: Icon::Clock,
            icon_bg: if is_overdue {
                cp.danger.gamma_multiply(0.15)
            } else {
                cp.accent.gamma_multiply(0.15)
            },
            icon_fg: if is_overdue { cp.danger } else { cp.accent },
            title: "Prazo de Entrega do Projeto".into(),
            subtitle: Some(format!("Status atual: {}", project_status(&project.status))),
            description: None,
            tag: Some((
                if is_overdue {
                    "Atrasado".into()
                } else {
                    "Prazo".into()
                },
                if is_overdue {
                    cp.danger.gamma_multiply(0.15)
                } else {
                    cp.surface_hover
                },
                if is_overdue { cp.danger } else { cp.accent },
            )),
            chat_id: None,
        });
    }

    if project.created_at > 0 {
        events.push(TimelineItem {
            timestamp: project.created_at,
            category: "milestones",
            icon: Icon::Calendar,
            icon_bg: cp.surface_hover,
            icon_fg: cp.secondary,
            title: "Projeto Criado".into(),
            subtitle: Some(format!(
                "Responsável: {}",
                if project.assignee.is_empty() {
                    "Equipe"
                } else {
                    &project.assignee
                }
            )),
            description: if !project.description.is_empty() {
                Some(project.description.clone())
            } else {
                None
            },
            tag: Some(("Criação".into(), cp.surface_hover, cp.dim)),
            chat_id: None,
        });
    }

    // 2. Tasks
    for t in app
        .crm_tasks
        .iter()
        .filter(|t| t.project_id.as_deref() == Some(&project.id) && t.kind == "task")
    {
        if let Some(c_at) = t.completed_at {
            events.push(TimelineItem {
                timestamp: c_at,
                category: "tasks",
                icon: Icon::CircleCheck,
                icon_bg: Color32::from_rgb(0x13, 0x2e, 0x1d),
                icon_fg: Color32::from_rgb(0x4a, 0xc7, 0x6e),
                title: format!("Tarefa Concluída: {}", t.title),
                subtitle: Some(format!(
                    "Prioridade: {} • Resp: {}",
                    t.priority,
                    if t.assignee.is_empty() {
                        "Equipe"
                    } else {
                        &t.assignee
                    }
                )),
                description: if !t.description.is_empty() {
                    Some(t.description.clone())
                } else {
                    None
                },
                tag: Some((
                    "Concluída".into(),
                    Color32::from_rgb(0x13, 0x2e, 0x1d),
                    Color32::from_rgb(0x4a, 0xc7, 0x6e),
                )),
                chat_id: t.chat_id.clone(),
            });
        } else if let Some(due) = t.due_at {
            let is_overdue = due < crate::util::now();
            events.push(TimelineItem {
                timestamp: due,
                category: "tasks",
                icon: Icon::ListChecks,
                icon_bg: if is_overdue {
                    cp.danger.gamma_multiply(0.15)
                } else {
                    cp.warning.gamma_multiply(0.15)
                },
                icon_fg: if is_overdue { cp.danger } else { cp.warning },
                title: format!("Prazo de Tarefa: {}", t.title),
                subtitle: Some(format!("Status: {} • Prioridade: {}", t.status, t.priority)),
                description: if !t.description.is_empty() {
                    Some(t.description.clone())
                } else {
                    None
                },
                tag: Some((
                    if is_overdue {
                        "Atrasada".into()
                    } else {
                        "Pendente".into()
                    },
                    cp.surface_hover,
                    if is_overdue { cp.danger } else { cp.warning },
                )),
                chat_id: t.chat_id.clone(),
            });
        } else if t.created_at > 0 {
            events.push(TimelineItem {
                timestamp: t.created_at,
                category: "tasks",
                icon: Icon::ListChecks,
                icon_bg: cp.surface_hover,
                icon_fg: cp.secondary,
                title: format!("Tarefa Criada: {}", t.title),
                subtitle: Some(format!("Prioridade: {}", t.priority)),
                description: if !t.description.is_empty() {
                    Some(t.description.clone())
                } else {
                    None
                },
                tag: Some(("Aberta".into(), cp.surface_hover, cp.dim)),
                chat_id: t.chat_id.clone(),
            });
        }
    }

    // 3. Calls
    for c in app.crm_tasks.iter().filter(|t| {
        t.project_id.as_deref() == Some(&project.id)
            && matches!(t.kind.as_str(), "call" | "conversation")
    }) {
        let ts = c.due_at.unwrap_or(c.created_at);
        let res_label = if !c.call_result.is_empty() {
            c.call_result.clone()
        } else {
            "Agendada".into()
        };
        let mut sub = format!("{} min", c.duration_minutes.unwrap_or(30));
        if !c.assignee.is_empty() {
            sub.push_str(&format!(" • Resp: {}", c.assignee));
        }
        if c.revenue_cents > 0 {
            sub.push_str(&format!(" • {}", format_currency(c.revenue_cents)));
        }
        events.push(TimelineItem {
            timestamp: ts,
            category: "calls",
            icon: Icon::Phone,
            icon_bg: Color32::from_rgb(0x1a, 0x2e, 0x3d),
            icon_fg: Color32::from_rgb(0x5c, 0xa7, 0xdb),
            title: format!("Call / Reunião: {}", c.title),
            subtitle: Some(sub),
            description: if !c.description.is_empty() {
                Some(c.description.clone())
            } else if !c.next_step.is_empty() {
                Some(format!("Próximo passo: {}", c.next_step))
            } else {
                None
            },
            tag: Some((res_label, cp.surface_hover, cp.accent)),
            chat_id: c
                .chat_id
                .clone()
                .or_else(|| project.chat_ids.first().cloned()),
        });
    }

    // 4. Follow-ups
    for f in &app.crm_followups {
        if project.chat_ids.contains(&f.chat_id) {
            events.push(TimelineItem {
                timestamp: f.remind_at,
                category: "milestones",
                icon: Icon::Bell,
                icon_bg: cp.warning.gamma_multiply(0.15),
                icon_fg: cp.warning,
                title: format!("Follow-up: {}", f.title),
                subtitle: Some(if f.done {
                    "Lembrete concluído".into()
                } else {
                    "Lembrete programado".into()
                }),
                description: None,
                tag: Some(("Follow-up".into(), cp.surface_hover, cp.secondary)),
                chat_id: Some(f.chat_id.clone()),
            });
        }
    }

    // 5. WhatsApp Messages from linked chats
    for chat_id in &project.chat_ids {
        if let Some(conv) = app.conversations.get(chat_id) {
            for msg in conv.messages.iter().rev().take(30) {
                let preview = message_preview(&msg.content);
                if preview.is_empty() {
                    continue;
                }
                events.push(TimelineItem {
                    timestamp: msg.timestamp,
                    category: "messages",
                    icon: if msg.from_me {
                        Icon::Send
                    } else {
                        Icon::MessageCircle
                    },
                    icon_bg: if msg.from_me {
                        cp.accent.gamma_multiply(0.15)
                    } else {
                        cp.surface_hover
                    },
                    icon_fg: if msg.from_me { cp.accent } else { cp.text },
                    title: if msg.from_me {
                        "Mensagem Enviada (Você)".into()
                    } else {
                        format!(
                            "Mensagem Recebida ({})",
                            msg.sender_name.as_deref().unwrap_or("Cliente")
                        )
                    },
                    subtitle: None,
                    description: Some(preview),
                    tag: Some((
                        if msg.from_me { "Enviada" } else { "Recebida" }.into(),
                        cp.surface_hover,
                        cp.dim,
                    )),
                    chat_id: Some(chat_id.clone()),
                });
            }
        }
    }

    // Sort newest first
    events.sort_by_key(|e| -e.timestamp);

    let all_count = events.len();
    let calls_count = events.iter().filter(|e| e.category == "calls").count();
    let tasks_count = events.iter().filter(|e| e.category == "tasks").count();
    let messages_count = events.iter().filter(|e| e.category == "messages").count();
    let milestones_count = events.iter().filter(|e| e.category == "milestones").count();

    // Filter toolbar (responsive: stacks search below chips when narrow)
    let avail_w = ui.available_width();
    if avail_w < 680.0 {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            let filter_btns = [
                ("all", format!("Todos ({all_count})")),
                ("calls", format!("📞 Calls ({calls_count})")),
                ("tasks", format!("✓ Tarefas ({tasks_count})")),
                ("messages", format!("💬 Mensagens ({messages_count})")),
                ("milestones", format!("🚩 Marcos ({milestones_count})")),
            ];
            for (key, label) in filter_btns {
                let active = current_filter == key;
                let btn = egui::Button::new(RichText::new(&label).size(12.0).color(if active {
                    cp.on_accent
                } else {
                    cp.secondary
                }))
                .fill(if active { cp.accent } else { cp.surface })
                .stroke(Stroke::new(
                    1.0,
                    if active { cp.accent } else { cp.outline },
                ))
                .corner_radius(CornerRadius::same(6));
                if ui.add(btn).clicked() {
                    current_filter = key.into();
                    ui.ctx().request_repaint();
                }
            }
        });

        ui.add_space(6.0);

        ui.add(
            egui::TextEdit::singleline(&mut search_query)
                .hint_text("🔍 Filtrar eventos...")
                .desired_width(ui.available_width()),
        );
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let filter_btns = [
                ("all", format!("Todos ({all_count})")),
                ("calls", format!("📞 Calls ({calls_count})")),
                ("tasks", format!("✓ Tarefas ({tasks_count})")),
                ("messages", format!("💬 Mensagens ({messages_count})")),
                ("milestones", format!("🚩 Marcos ({milestones_count})")),
            ];
            for (key, label) in filter_btns {
                let active = current_filter == key;
                let btn = egui::Button::new(RichText::new(&label).size(12.0).color(if active {
                    cp.on_accent
                } else {
                    cp.secondary
                }))
                .fill(if active { cp.accent } else { cp.surface })
                .stroke(Stroke::new(
                    1.0,
                    if active { cp.accent } else { cp.outline },
                ))
                .corner_radius(CornerRadius::same(6));
                if ui.add(btn).clicked() {
                    current_filter = key.into();
                    ui.ctx().request_repaint();
                }
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                ui.add(
                    egui::TextEdit::singleline(&mut search_query)
                        .hint_text("🔍 Buscar na timeline...")
                        .desired_width(200.0),
                );
            });
        });
    }

    ui.ctx()
        .data_mut(|d| d.insert_temp(filter_id, current_filter.clone()));
    ui.ctx()
        .data_mut(|d| d.insert_temp(search_id, search_query.clone()));

    ui.add_space(12.0);

    // Apply filtering
    let filtered_events: Vec<_> = events
        .into_iter()
        .filter(|e| {
            if current_filter != "all" && e.category != current_filter {
                return false;
            }
            if !search_query.trim().is_empty() {
                let q = search_query.trim().to_lowercase();
                let in_title = e.title.to_lowercase().contains(&q);
                let in_sub = e
                    .subtitle
                    .as_deref()
                    .is_some_and(|s| s.to_lowercase().contains(&q));
                let in_desc = e
                    .description
                    .as_deref()
                    .is_some_and(|d| d.to_lowercase().contains(&q));
                if !in_title && !in_sub && !in_desc {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered_events.is_empty() {
        Frame::new()
            .fill(cp.surface)
            .stroke(Stroke::new(1.0, cp.outline))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin::same(20))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    theme::icon(ui, Icon::Clock, 24.0, cp.dim);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Nenhum evento encontrado para os filtros selecionados.")
                            .size(13.5)
                            .color(cp.dim),
                    );
                });
            });
    } else {
        let today = crate::util::today();
        let mut last_day: Option<jiff::civil::Date> = None;

        for event in filtered_events {
            let (date_opt, hour, min) = match crate::util::local_datetime(event.timestamp) {
                Some((d, h, m)) => (Some(d), h, m),
                None => (None, 0, 0),
            };

            // Day separator
            if let Some(date) = date_opt {
                if last_day != Some(date) {
                    last_day = Some(date);
                    ui.add_space(10.0);

                    let day_diff = today
                        .since(date)
                        .map(|span| span.get_days())
                        .unwrap_or(i32::MAX);

                    let day_str = match day_diff {
                        0 => "Hoje".to_string(),
                        1 => "Ontem".to_string(),
                        -1 => "Amanhã".to_string(),
                        _ => format!("{:02}/{:02}/{}", date.day(), date.month(), date.year()),
                    };

                    ui.horizontal(|ui| {
                        theme::icon(ui, Icon::Calendar, 14.0, cp.accent);
                        ui.label(RichText::new(&day_str).size(13.0).strong().color(cp.text));
                        ui.add_space(6.0);
                        let sep_w = (ui.available_width() - 8.0).max(20.0);
                        let (sep_rect, _) =
                            ui.allocate_exact_size(vec2(sep_w, 1.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(sep_rect, CornerRadius::ZERO, cp.outline);
                    });
                    ui.add_space(6.0);
                }
            }

            // Event Card
            Frame::new()
                .fill(cp.surface)
                .stroke(Stroke::new(1.0, cp.outline))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        let header_w = ui.available_width();
                        let is_narrow = header_w < 520.0;

                        if is_narrow {
                            ui.horizontal(|ui| {
                                Frame::new()
                                    .fill(event.icon_bg)
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::same(6))
                                    .show(ui, |ui| {
                                        theme::icon(ui, event.icon, 14.0, event.icon_fg);
                                    });
                                ui.label(
                                    RichText::new(format!("{:02}:{:02}", hour, min))
                                        .size(11.5)
                                        .color(cp.secondary),
                                );
                                if let Some((tag_txt, tag_bg, tag_fg)) = &event.tag {
                                    Frame::new()
                                        .fill(*tag_bg)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(tag_txt).size(11.0).color(*tag_fg),
                                            );
                                        });
                                }
                            });
                            ui.add_space(4.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&event.title)
                                        .size(13.5)
                                        .strong()
                                        .color(cp.text),
                                )
                                .wrap(),
                            );
                            if let Some(chat_id) = &event.chat_id {
                                ui.add_space(4.0);
                                let chat_btn = egui::Button::new(
                                    RichText::new("💬 Abrir Conversa")
                                        .size(11.5)
                                        .color(cp.accent),
                                )
                                .fill(cp.surface_hover)
                                .corner_radius(CornerRadius::same(4));
                                if ui.add(chat_btn).clicked() {
                                    app.actions.push(Action::OpenChat(chat_id.clone()));
                                    ui.ctx().request_repaint();
                                }
                            }
                        } else {
                            ui.horizontal(|ui| {
                                Frame::new()
                                    .fill(event.icon_bg)
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(Margin::same(6))
                                    .show(ui, |ui| {
                                        theme::icon(ui, event.icon, 14.0, event.icon_fg);
                                    });
                                ui.label(
                                    RichText::new(format!("{:02}:{:02}", hour, min))
                                        .size(12.0)
                                        .color(cp.secondary),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(&event.title)
                                            .size(13.5)
                                            .strong()
                                            .color(cp.text),
                                    )
                                    .wrap(),
                                );
                                if let Some((tag_txt, tag_bg, tag_fg)) = &event.tag {
                                    Frame::new()
                                        .fill(*tag_bg)
                                        .corner_radius(CornerRadius::same(4))
                                        .inner_margin(Margin::symmetric(6, 2))
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(tag_txt).size(11.0).color(*tag_fg),
                                            );
                                        });
                                }

                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if let Some(chat_id) = &event.chat_id {
                                        let chat_btn = egui::Button::new(
                                            RichText::new("💬 Abrir Conversa")
                                                .size(11.5)
                                                .color(cp.accent),
                                        )
                                        .fill(cp.surface_hover)
                                        .corner_radius(CornerRadius::same(4));
                                        if ui.add(chat_btn).clicked() {
                                            app.actions.push(Action::OpenChat(chat_id.clone()));
                                            ui.ctx().request_repaint();
                                        }
                                    }
                                });
                            });
                        }

                        if let Some(sub) = &event.subtitle {
                            ui.add_space(4.0);
                            ui.label(RichText::new(sub).size(12.0).color(cp.secondary));
                        }

                        if let Some(desc) = &event.description {
                            ui.add_space(6.0);
                            Frame::new()
                                .fill(cp.panel)
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(desc).size(12.5).color(cp.text),
                                        )
                                        .wrap(),
                                    );
                                });
                        }
                    });
                });
            ui.add_space(6.0);
        }
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
