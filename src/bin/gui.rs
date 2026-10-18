use eframe::egui;
use pumpkin_weight::{Game, Unit};

const BTN: [f32; 2] = [220.0, 56.0];
const BTN_SMALL: [f32; 2] = [220.0, 40.0];
const FIELD: [f32; 2] = [220.0, 36.0];
const TOGGLE: [f32; 2] = [42.0, 24.0];
const TOGGLE_GAP: f32 = 6.0;
const LOGO_PANEL_FRACTION: f32 = 0.34;
const LOGO_PANEL_MIN: f32 = 150.0;
const LOGO_PANEL_MAX: f32 = 340.0;
const LOGO_MARGIN: f32 = 14.0;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 117, 24);
const ERROR_RED: egui::Color32 = egui::Color32::from_rgb(255, 105, 105);

/// Embedded at compile time rather than read at runtime: a kiosk that boots
/// straight into the game has no sensible way to recover from a missing file.
const CHEERS: &str = include_str!("../../assets/cheers.txt");

/// At least this many players before anyone gets cheered — with one player
/// there is no contest to lead.
const CHEER_FROM: usize = 2;

fn cheer_lines() -> Vec<&'static str> {
    CHEERS
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
}

enum Screen {
    SetRealWeight,
    Menu,
    EnterPlayer,
    Results,
}

struct App {
    screen: Screen,
    weight_input: String,
    unit: Unit,
    error: Option<String>,
    game: Option<Game>,
    name_input: String,
    guess_input: String,
    cheer: Option<String>,
    last_cheer: Option<usize>,
    rng: u64,
}

impl App {
    fn new() -> Self {
        Self {
            screen: Screen::SetRealWeight,
            weight_input: String::new(),
            unit: Unit::Kg,
            error: None,
            game: None,
            name_input: String::new(),
            guess_input: String::new(),
            cheer: None,
            last_cheer: None,
            rng: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_nanos() as u64)
                // xorshift is stuck at zero, so make sure the seed never is.
                | 1,
        }
    }

    /// xorshift64*. A dependency-free generator is plenty for picking one of
    /// fifty strings, and saves pulling `rand` onto the Pi's build.
    fn next_random(&mut self, len: usize) -> usize {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % len
    }

    /// Pick a new cheer for whoever is currently closest.
    ///
    /// Called once when a player is added, never per frame — rolling the dice
    /// inside `ui()` would reshuffle the text sixty times a second.
    fn refresh_cheer(&mut self) {
        // Take a `char` out of the game so the borrow ends before the
        // `&mut self` call to `next_random` below.
        let leader = self.game.as_ref().and_then(|game| {
            if game.players().len() < CHEER_FROM {
                return None;
            }
            game.ranking().first().and_then(|p| p.name.chars().next())
        });

        let lines = cheer_lines();
        let (Some(initial), false) = (leader, lines.is_empty()) else {
            self.cheer = None;
            return;
        };

        let mut index = self.next_random(lines.len());
        if Some(index) == self.last_cheer && lines.len() > 1 {
            index = (index + 1) % lines.len();
        }
        self.last_cheer = Some(index);

        let name = format!("{}.", initial.to_uppercase());
        self.cheer = Some(lines[index].replace("{}", &name));
    }

    /// kg / lb toggle. Allocating a fixed-width row is what lets the
    /// surrounding `vertical_centered` centre it — a plain `ui.horizontal`
    /// claims the full width and so stays left-aligned.
    ///
    /// The buttons use `min_size` rather than `add_sized`: `add_sized` treats
    /// its size as a maximum and overflows centred, which misaligns the pair
    /// whenever the global button padding makes them taller than the box.
    fn unit_toggle(&mut self, ui: &mut egui::Ui) {
        let size = egui::Vec2::from(TOGGLE);
        let row = egui::vec2(TOGGLE[0] * 2.0 + TOGGLE_GAP, TOGGLE[1]);

        ui.allocate_ui_with_layout(
            row,
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = TOGGLE_GAP;
                ui.spacing_mut().button_padding = egui::vec2(4.0, 2.0);
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Button,
                    egui::FontId::new(14.0, egui::FontFamily::Proportional),
                );

                for (unit, label) in [(Unit::Kg, "kg"), (Unit::Lb, "lb")] {
                    let button = egui::Button::selectable(self.unit == unit, label).min_size(size);
                    if ui.add(button).clicked() {
                        self.unit = unit;
                    }
                }
            },
        );
    }

    /// Drawn in the same slot on every screen, and the slot is reserved even
    /// when there is no message, so nothing below it jumps when one appears.
    fn error_banner(&self, ui: &mut egui::Ui) {
        let text = self.error.as_deref().unwrap_or(" ");
        ui.add_sized(
            [FIELD[0], 26.0],
            egui::Label::new(egui::RichText::new(text).color(ERROR_RED)),
        );
    }

    fn set_real_weight(&mut self, ui: &mut egui::Ui) {
        ui.heading("Guess the Pumpkin Weight");
        ui.label("Enter the real weight to start.");
        ui.add_space(18.0);

        self.unit_toggle(ui);

        ui.add_sized(
            FIELD,
            egui::TextEdit::singleline(&mut self.weight_input).hint_text("weight"),
        );

        self.error_banner(ui);

        if ui.add_sized(BTN, egui::Button::new("Start")).clicked() {
            match self.unit.parse_to_kg(&self.weight_input) {
                Ok(kg) => {
                    self.game =
                        Some(Game::new(kg, self.unit).expect("parse_to_kg guarantees positive"));
                    self.error = None;
                    self.screen = Screen::Menu;
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let players = self.game.as_ref().map_or(0, |g| g.players().len());

        ui.heading("Who's next?");
        ui.label(match players {
            1 => "1 player so far".to_string(),
            n => format!("{} players so far", n),
        });

        if let Some(cheer) = &self.cheer {
            ui.add_space(8.0);
            ui.add(egui::Label::new(
                egui::RichText::new(cheer).italics().color(ACCENT),
            ));
        }

        ui.add_space(26.0);

        if ui
            .add_sized(BTN, egui::Button::new("Next player"))
            .clicked()
        {
            self.error = None;
            self.screen = Screen::EnterPlayer;
        }

        if ui.add_sized(BTN, egui::Button::new("End game")).clicked() {
            self.error = None;
            self.screen = Screen::Results;
        }
    }

    fn enter_player(&mut self, ui: &mut egui::Ui) {
        ui.heading("New player");
        ui.add_space(18.0);

        ui.add_sized(
            FIELD,
            egui::TextEdit::singleline(&mut self.name_input).hint_text("your name"),
        );

        self.unit_toggle(ui);

        ui.add_sized(
            FIELD,
            egui::TextEdit::singleline(&mut self.guess_input).hint_text("your guess"),
        );

        self.error_banner(ui);

        if ui.add_sized(BTN, egui::Button::new("Add player")).clicked() {
            match self.unit.parse_to_kg(&self.guess_input) {
                Ok(kg) => {
                    if let Some(game) = &mut self.game {
                        match game.add_player(&self.name_input, kg) {
                            Ok(()) => {
                                self.error = None;
                                self.name_input.clear();
                                self.guess_input.clear();
                                self.screen = Screen::Menu;
                                self.refresh_cheer();
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }

        if ui
            .add_sized(BTN_SMALL, egui::Button::new("Cancel"))
            .clicked()
        {
            self.name_input.clear();
            self.guess_input.clear();
            self.error = None;
            self.screen = Screen::Menu;
        }
    }

    fn results(&mut self, ui: &mut egui::Ui) {
        if let Some(game) = &self.game {
            match game.winner().as_slice() {
                [] => {
                    ui.heading("Nobody played");
                }
                [w] => {
                    ui.label("The winner is");
                    ui.add(egui::Label::new(
                        egui::RichText::new(w.name.as_str())
                            .size(42.0)
                            .strong()
                            .color(ACCENT),
                    ));
                }
                tied => {
                    let names: Vec<&str> = tied.iter().map(|p| p.name.as_str()).collect();
                    ui.label("It's a tie between");
                    ui.add(egui::Label::new(
                        egui::RichText::new(names.join(" and "))
                            .size(30.0)
                            .strong()
                            .color(ACCENT),
                    ));
                }
            }

            ui.add_space(10.0);
            ui.label(format!("The pumpkin weighed {:.2} kg", game.real_weight()));
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);

            egui::Grid::new("ranking")
                .num_columns(3)
                .striped(true)
                .spacing([28.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Player").strong());
                    ui.label(egui::RichText::new("Guess").strong());
                    ui.label(egui::RichText::new("Off by").strong());
                    ui.end_row();

                    for p in game.ranking() {
                        ui.label(&p.name);
                        ui.label(format!("{:.2} kg", p.guess));
                        ui.label(format!("{:.2} kg", p.error(game.real_weight())));
                        ui.end_row();
                    }
                });
        }

        ui.add_space(26.0);

        if ui.add_sized(BTN, egui::Button::new("New game")).clicked() {
            self.game = None;
            self.weight_input.clear();
            self.name_input.clear();
            self.guess_input.clear();
            self.error = None;
            self.cheer = None;
            self.screen = Screen::SetRealWeight;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Logo column, drawn before the content so it claims its width first
        // and every screen gets the same remaining area. Sized as a fraction
        // of the window rather than in fixed pixels, so it adapts to whatever
        // resolution and zoom factor the display ends up using.
        let panel_width =
            (ui.available_width() * LOGO_PANEL_FRACTION).clamp(LOGO_PANEL_MIN, LOGO_PANEL_MAX);

        egui::Panel::left("logo_panel")
            .resizable(false)
            .exact_size(panel_width)
            .show(ui, |ui| {
                // The logo is square, so one side length satisfies both caps,
                // and knowing its height lets us centre it vertically.
                let side = (panel_width - LOGO_MARGIN * 2.0)
                    .min(ui.available_height() - LOGO_MARGIN * 2.0)
                    .max(0.0);
                let top = ((ui.available_height() - side) / 2.0).max(0.0);

                ui.vertical_centered(|ui| {
                    ui.add_space(top);
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/pumpkin_logo.png"))
                            .fit_to_exact_size(egui::vec2(side, side))
                            .corner_radius(12.0),
                    );
                });
            });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(26.0);

                match self.screen {
                    Screen::SetRealWeight => self.set_real_weight(ui),
                    Screen::Menu => self.menu(ui),
                    Screen::EnterPlayer => self.enter_player(ui),
                    Screen::Results => self.results(ui),
                }

                ui.add_space(26.0);
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        // OpenGL ES rather than eframe's default wgpu: the Pi 4's V3D driver
        // can't meet wgpu's default device limits and fails to start.
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 780.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Pumpkin Weight",
        options,
        Box::new(|cc| {
            let ctx = &cc.egui_ctx;

            // Required before `include_image!` will render anything.
            egui_extras::install_image_loaders(ctx);

            ctx.set_zoom_factor(ZOOM);
            ctx.set_theme(egui::ThemePreference::Dark);

            ctx.all_styles_mut(|style| {
                use egui::{FontFamily::Proportional, FontId, TextStyle};

                style
                    .text_styles
                    .insert(TextStyle::Heading, FontId::new(30.0, Proportional));
                style
                    .text_styles
                    .insert(TextStyle::Body, FontId::new(18.0, Proportional));
                style
                    .text_styles
                    .insert(TextStyle::Button, FontId::new(20.0, Proportional));

                style.spacing.item_spacing = egui::vec2(12.0, 12.0);
                style.spacing.button_padding = egui::vec2(18.0, 12.0);

                style.visuals.selection.bg_fill = ACCENT;
            });

            Ok(Box::new(App::new()))
        }),
    )
}
