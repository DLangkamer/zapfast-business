//! The navigation rail on the left of the chat list: Chats, Calls and CRM
//! at the top; Starred, Archived and Locked chats (while there are any),
//! Settings and our own avatar (the account switcher) at the bottom.
//! While the chat list is hidden, the chats' avatars fill the rail between
//! the two, so every chat is still one click away.

use egui::{Frame, Margin, Rect, Sense, Vec2, vec2};

use crate::app::App;
use crate::model::{Action, Page};
use crate::theme::{self, Icon, Palette};

use super::focus::{Stop, TabStop};
use super::widgets;

/// Width of the rail, on every platform.
pub const WIDTH: f32 = 52.0;
/// Height of one destination at the top.
const CELL: f32 = 48.0;
/// The lit square behind a destination's icon.
const PLATE: f32 = 36.0;
/// A destination's icon.
const ICON: f32 = 20.0;
/// Our avatar at the foot of the rail.
const AVATAR: f32 = 32.0;
/// Space above the first destination and between bottom group plates.
const GAP: f32 = 8.0;
/// The least height the chats' avatars take.
const MIN_AVATARS: f32 = 64.0;
/// Width of the line between destinations and chat avatars.
const DIVIDER: f32 = 24.0;
/// Clearance for macOS traffic lights.
const LIGHTS_CLEARANCE: f32 = theme::TOP_BAR_HEIGHT / 2.0 + 12.0;

/// Where the rail was laid out, for layout tests.
pub(crate) fn rect_id() -> egui::Id {
    egui::Id::new("nav-rail")
}

/// Stable id of a destination's button.
pub fn destination_id(destination: Destination) -> egui::Id {
    egui::Id::new(("nav-rail-destination", destination as u8))
}

/// How far the macOS traffic lights reach past the rail.
pub fn lights_overhang(ctx: &egui::Context) -> f32 {
    if !theme::macos_chrome(ctx) {
        return 0.0;
    }
    (theme::traffic_light_inset(ctx) - WIDTH).max(0.0)
}

/// A place the rail leads to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    Chats,
    Calls,
    Crm,
    Starred,
    Archived,
    Locked,
    Settings,
}

impl Destination {
    /// Whether the window shows this destination now: its icon is lit.
    pub fn lit(self, app: &App) -> bool {
        let chats = app.page == Page::Chats && !app.show_starred;
        match self {
            Self::Chats => chats && !app.show_archived && !app.locked_folder_open(),
            Self::Calls => app.page == Page::Calls,
            Self::Crm => app.page == Page::Crm,
            Self::Starred => app.page == Page::Chats && app.show_starred,
            Self::Archived => chats && app.show_archived,
            Self::Locked => chats && app.locked_folder_open(),
            Self::Settings => matches!(app.page, Page::Settings | Page::Wallpaper),
        }
    }

    /// Whether the rail offers this destination: Archived while chats are
    /// archived and Locked while chats are locked.
    pub fn offered(self, app: &App) -> bool {
        match self {
            Self::Archived => app.archived_count() > 0 || app.show_archived,
            Self::Locked => app.locked_count() > 0 || app.locked_folder_open(),
            _ => true,
        }
    }

    /// The actions that bring the window here.
    pub fn actions(self, app: &App) -> Vec<Action> {
        let mut actions = Vec::new();
        let to_chats = |actions: &mut Vec<Action>| {
            if app.page != Page::Chats {
                actions.push(Action::Open(Page::Chats));
            }
            if app.show_starred {
                actions.push(Action::ToggleStarred);
            }
        };
        match self {
            Self::Chats => {
                to_chats(&mut actions);
                if app.show_archived {
                    actions.push(Action::ShowArchived(false));
                }
                if app.locked_folder {
                    actions.push(Action::CloseLockedFolder);
                }
            }
            Self::Calls => {
                if app.page != Page::Calls {
                    actions.push(Action::Open(Page::Calls));
                }
            }
            Self::Crm => {
                if app.page != Page::Crm {
                    actions.push(Action::Open(Page::Crm));
                }
            }
            Self::Starred => {
                if app.page != Page::Chats {
                    actions.push(Action::Open(Page::Chats));
                }
                if !app.show_starred {
                    actions.push(Action::ToggleStarred);
                }
            }
            Self::Archived => {
                if !self.lit(app) {
                    to_chats(&mut actions);
                    if !app.sidebar_visible {
                        actions.push(Action::ToggleSidebar);
                    }
                    actions.push(Action::ShowArchived(true));
                }
            }
            Self::Locked => {
                if !self.lit(app) {
                    to_chats(&mut actions);
                    actions.push(Action::OpenLockedFolder);
                }
            }
            Self::Settings => actions.push(Action::ToggleSettings),
        }
        actions
    }
}

/// The rail as a fixed panel on the left.
pub fn show(app: &mut App, ui: &mut egui::Ui, hidden: bool) {
    let palette = app.palette;
    let panel = egui::Panel::left("chat-rail")
        .resizable(false)
        .exact_size(WIDTH)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.rail()).inner_margin(Margin::ZERO));

    let response = panel.show(ui, |ui| {
        let row_gap = ui.spacing().item_spacing.y;
        ui.spacing_mut().item_spacing.y = 0.0;
        let width = ui.available_width();
        if theme::traffic_light_inset(ui.ctx()) > 0.0 {
            let (strip, _) = ui.allocate_exact_size(vec2(width, LIGHTS_CLEARANCE), Sense::hover());
            super::titlebar_drag(ui, strip);
        } else {
            ui.add_space(GAP);
        }

        let bottom: Vec<Destination> = [
            Destination::Starred,
            Destination::Archived,
            Destination::Locked,
            Destination::Settings,
        ]
        .into_iter()
        .filter(|destination| destination.offered(app))
        .collect();

        let foot = bottom.len() as f32 * (PLATE + GAP) + GAP + AVATAR + GAP;
        let top_count = 3.0; // Chats, Calls, Crm
        let minimum = top_count * CELL
            + foot
            + if hidden {
                CELL + GAP + MIN_AVATARS
            } else {
                0.0
            };
        let height = ui.available_height().max(minimum);

        egui::ScrollArea::vertical()
            .id_salt("navigation-rail")
            .auto_shrink([false, false])
            .min_scrolled_height(0.0)
            .show(ui, |ui| {
                ui.set_height(height);
                for destination in [Destination::Chats, Destination::Calls, Destination::Crm] {
                    destination_button(app, ui, &palette, destination, CELL);
                }
                if hidden {
                    show_list_button(app, ui, &palette);
                    ui.add_space(GAP / 2.0);
                    let line = Rect::from_center_size(
                        ui.cursor().min + vec2(width / 2.0, 0.0),
                        vec2(DIVIDER, 1.0),
                    );
                    ui.painter().rect_filled(line, 0.0, palette.outline);
                    ui.add_space(GAP / 2.0);
                    let middle = (ui.available_height() - foot).max(MIN_AVATARS);
                    ui.allocate_ui_with_layout(
                        vec2(width, middle),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = row_gap;
                            super::chats::compact_list(app, ui);
                        },
                    );
                }
                let rest = (ui.available_height() - foot).max(0.0);
                ui.add_space(rest);
                for destination in bottom {
                    ui.add_space(GAP);
                    destination_button(app, ui, &palette, destination, PLATE);
                }
                ui.add_space(GAP);
                ui.vertical_centered(|ui| {
                    super::accounts::avatar_button(app, ui, AVATAR).tab_stop(Stop::Profile);
                });
            });
    });
    let rect = response.response.rect;
    ui.ctx().data_mut(|data| data.insert_temp(rect_id(), rect));
    if hidden {
        widgets::paint_edge_beside(ui, &palette, rect);
    }
}

/// The button that brings the hidden chat list back.
fn show_list_button(app: &mut App, ui: &mut egui::Ui, palette: &Palette) {
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), CELL),
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
        |ui| {
            if theme::icon_button(
                ui,
                Icon::PanelLeft,
                18.0,
                palette.secondary,
                palette.text,
                "Exibir lista de conversas (Ctrl+B)",
            )
            .tab_stop(Stop::Sidebar)
            .clicked()
            {
                app.actions.push(Action::ToggleSidebar);
            }
        },
    );
}

/// One destination button on the rail.
fn destination_button(
    app: &mut App,
    ui: &mut egui::Ui,
    palette: &Palette,
    destination: Destination,
    height: f32,
) {
    let lit = destination.lit(app);
    let mut badge = 0;
    let (icon, stop, label) = match destination {
        Destination::Chats => (
            Icon::MessageCircle,
            Stop::Chats,
            "Conversas (Ctrl+Shift+1)".to_string(),
        ),
        Destination::Calls => (
            Icon::Phone,
            Stop::Calls,
            "Ligações (Ctrl+Shift+2)".to_string(),
        ),
        Destination::Crm => {
            // Count pending followups due today or overdue
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            badge = app
                .crm_followups
                .iter()
                .filter(|f| !f.done && f.remind_at <= now + 86400)
                .count() as u32;
            (
                Icon::ListChecks,
                Stop::Crm,
                "Central CRM & Funil (Ctrl+Shift+3)".to_string(),
            )
        }
        Destination::Starred => (
            Icon::Star,
            Stop::Starred,
            "Mensagens favoritas".to_string(),
        ),
        Destination::Archived => {
            badge = app.archived_unread() as u32;
            (
                Icon::Archive,
                Stop::Archived,
                "Arquivadas".to_string(),
            )
        }
        Destination::Locked => (
            Icon::Lock,
            Stop::Locked,
            "Bloqueadas".to_string(),
        ),
        Destination::Settings => (
            Icon::Settings,
            Stop::Settings,
            if lit {
                "Fechar configurações (Ctrl+,)".to_string()
            } else {
                "Configurações (Ctrl+,)".to_string()
            },
        ),
    };

    let (cell, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let plate = Rect::from_center_size(cell.center(), Vec2::splat(PLATE));
    let response = ui
        .interact(plate, destination_id(destination), Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    theme::reveal_focus(&response);

    let spoken = if badge > 0 {
        format!("{label}, {badge}")
    } else {
        label.clone()
    };
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            lit,
            spoken.as_str(),
        )
    });

    if ui.is_rect_visible(plate) {
        let hovered = response.hovered() || response.has_focus();
        if lit {
            widgets::highlight_card(ui, palette, plate, palette.surface_active);
        } else if hovered {
            widgets::highlight_card(ui, palette, plate, palette.surface_hover);
        }
        let tint = if lit || hovered {
            palette.text
        } else {
            palette.secondary
        };
        let scale = if response.is_pointer_button_down_on() {
            0.92
        } else {
            1.0
        };
        theme::paint_icon(ui, icon, plate, ICON * scale, tint);
        if badge > 0 {
            widgets::unread_indicator(
                ui,
                palette,
                plate.right_top() + vec2(-3.0, 5.0),
                badge,
                false,
                false,
            );
        }
    }
    let response = response.on_hover_text(label).tab_stop(stop);
    if response.clicked() {
        let actions = destination.actions(app);
        app.actions.extend(actions);
    }
}
