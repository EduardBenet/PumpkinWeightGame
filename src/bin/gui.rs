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

/// Global UI scale. Tuned for the official 7" DSI panel (800x480), which at
/// this factor gives roughly 667x400 points of layout space — enough for the
/// tallest screen without scrolling. Raise it for a larger or denser display.
const ZOOM: f32 = 1.2;

/// On-screen key size. The panel has no physical keyboard, so these have to
/// be hittable with a fingertip, not a stylus.
const KEY: [f32; 2] = [38.0, 34.0];
const KEY_GAP: f32 = 4.0;
const NUMKEY: [f32; 2] = [60.0, 42.0];

const BACKSPACE: &str = "\u{232b}";

/// Longest a player name may get. Stops one child filling the leaderboard.
const NAME_MAX: usize = 12;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 117, 24);
const ERROR_RED: egui::Color32 = egui::Color32::from_rgb(255, 105, 105);

const LETTER_ROWS: [&str; 3] = ["qwertyuiop", "asdfghjkl", "zxcvbnm"];

enum Screen {
    SetRealWeight,
    Menu,
    EnterName,
    EnterGuess,
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
        }
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
        ui.heading("Pumpkin Weight");
        ui.label("Enter the real weight to start.");
        ui.add_space(12.0);

        self.unit_toggle(ui);
        ui.add_sized(
            FIELD,
            egui::Label::new(reading(&self.weight_input, "weight")),
        );
        self.error_banner(ui);

        number_pad(ui, &mut self.weight_input);
        ui.add_space(10.0);

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
        ui.add_space(26.0);

        if ui
            .add_sized(BTN, egui::Button::new("Next player"))
            .clicked()
        {
            self.error = None;
            self.screen = Screen::EnterName;
        }

        if ui.add_sized(BTN, egui::Button::new("End game")).clicked() {
            self.error = None;
            self.screen = Screen::Results;
        }
    }

    fn enter_name(&mut self, ui: &mut egui::Ui) {
        ui.heading("What's your name?");
        ui.add_space(8.0);

        ui.add_sized(FIELD, egui::Label::new(reading(&self.name_input, "name")));
        self.error_banner(ui);

        letter_pad(ui, &mut self.name_input);
        ui.add_space(10.0);

        if ui.add_sized(BTN, egui::Button::new("Next")).clicked() {
            // Check the name before asking for a guess, so a duplicate is
            // caught on the screen that can still fix it.
            let verdict = self
                .game
                .as_ref()
                .map(|game| game.check_name(&self.name_input));

            match verdict {
                Some(Ok(())) => {
                    self.error = None;
                    self.screen = Screen::EnterGuess;
                }
                Some(Err(e)) => self.error = Some(e.to_string()),
                None => self.error = Some("no game in progress".to_string()),
            }
        }

        if ui
            .add_sized(BTN_SMALL, egui::Button::new("Cancel"))
            .clicked()
        {
            self.cancel_entry();
        }
    }

    fn enter_guess(&mut self, ui: &mut egui::Ui) {
        ui.heading(format!("How heavy, {}?", self.name_input));
        ui.add_space(8.0);

        self.unit_toggle(ui);
        ui.add_sized(FIELD, egui::Label::new(reading(&self.guess_input, "guess")));
        self.error_banner(ui);

        number_pad(ui, &mut self.guess_input);
        ui.add_space(10.0);

        if ui.add_sized(BTN, egui::Button::new("Add player")).clicked() {
            match self.unit.parse_to_kg(&self.guess_input) {
                Ok(kg) => {
                    let added = self
                        .game
                        .as_mut()
                        .map(|game| game.add_player(&self.name_input, kg));

                    match added {
                        Some(Ok(())) => {
                            self.name_input.clear();
                            self.guess_input.clear();
                            self.error = None;
                            self.screen = Screen::Menu;
                        }
                        Some(Err(e)) => self.error = Some(e.to_string()),
                        None => self.error = Some("no game in progress".to_string()),
                    }
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }

        if ui.add_sized(BTN_SMALL, egui::Button::new("Back")).clicked() {
            self.guess_input.clear();
            self.error = None;
            self.screen = Screen::EnterName;
        }
    }

    fn cancel_entry(&mut self) {
        self.name_input.clear();
        self.guess_input.clear();
        self.error = None;
        self.screen = Screen::Menu;
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
            self.screen = Screen::SetRealWeight;
        }
    }
}

/// What a text field would show: the typed value, or greyed placeholder text
/// while it is empty. These are `Label`s rather than `TextEdit`s because the
/// on-screen pads are the only way in — a focusable field on a kiosk just
/// invites a caret nobody can move.
fn reading(value: &str, placeholder: &str) -> egui::RichText {
    if value.is_empty() {
        egui::RichText::new(placeholder).weak()
    } else {
        egui::RichText::new(value).size(24.0).strong()
    }
}

/// Digits, a decimal point and backspace.
fn number_pad(ui: &mut egui::Ui, target: &mut String) {
    let rows = [
        ["1", "2", "3"],
        ["4", "5", "6"],
        ["7", "8", "9"],
        [".", "0", BACKSPACE],
    ];
    let width = NUMKEY[0] * 3.0 + KEY_GAP * 2.0;

    pad(ui, width, |ui| {
        for row in rows {
            ui.horizontal(|ui| {
                for label in row {
                    if ui
                        .add(egui::Button::new(label).min_size(NUMKEY.into()))
                        .clicked()
                    {
                        press(target, label, usize::MAX);
                    }
                }
            });
        }
    });
}

/// A-Z plus backspace, laid out as a compact qwerty.
fn letter_pad(ui: &mut egui::Ui, target: &mut String) {
    let longest = LETTER_ROWS
        .iter()
        .map(|r| r.chars().count())
        .max()
        .unwrap_or(1);
    let width = KEY[0] * longest as f32 + KEY_GAP * (longest as f32 - 1.0);

    pad(ui, width, |ui| {
        for (index, row) in LETTER_ROWS.iter().enumerate() {
            ui.horizontal(|ui| {
                for letter in row.chars() {
                    let label = letter.to_uppercase().to_string();
                    if ui
                        .add(egui::Button::new(label).min_size(KEY.into()))
                        .clicked()
                    {
                        press(target, &letter.to_string(), NAME_MAX);
                    }
                }

                // Backspace rides on the shortest row so the block stays tidy.
                if index == LETTER_ROWS.len() - 1
                    && ui
                        .add(egui::Button::new(BACKSPACE).min_size(KEY.into()))
                        .clicked()
                {
                    target.pop();
                }
            });
        }
    });
}

/// Shared chrome for both pads: a fixed-width block so the enclosing
/// `vertical_centered` can centre it, with keys tight together and smaller
/// text than the primary buttons.
fn pad(ui: &mut egui::Ui, width: f32, contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(KEY_GAP, KEY_GAP);
            ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );
            contents(ui);
        },
    );
}

/// Apply one keypress. Letters are capitalised at the start of a name and
/// lowercased after it, so "BOB" and "bob" both come out as "Bob".
fn press(target: &mut String, label: &str, max: usize) {
    if label == BACKSPACE {
        target.pop();
        return;
    }
    if target.chars().count() >= max {
        return;
    }
    match label.chars().next() {
        Some(c) if c.is_alphabetic() && !target.is_empty() => {
            target.extend(c.to_lowercase());
        }
        Some(c) => target.extend(c.to_uppercase()),
        None => {}
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Escape quits. The panel has a USB keyboard attached, and without
        // this the only way out of a fullscreen kiosk is killing the process.
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

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
                let usable = ui.available_height() - LOGO_MARGIN * 2.0;

                // The logo is square, so one side length satisfies both caps,
                // and knowing its height lets us centre it vertically.
                let side = (panel_width - LOGO_MARGIN * 2.0).min(usable).max(0.0);
                let top = ((ui.available_height() - side) / 2.0 - LOGO_MARGIN).max(0.0);

                ui.vertical_centered(|ui| {
                    ui.add_space(top);
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/pumpkin_logo.png"))
                            .fit_to_exact_size(egui::vec2(side, side))
                            .corner_radius(12.0),
                    );

                    // Deliberately small and dim: reachable when you want it,
                    // not an obvious target for a four-year-old.
                    ui.add_space(8.0);
                    let exit = egui::Button::new(egui::RichText::new("exit").size(12.0).weak())
                        .frame(false);
                    if ui.add(exit).clicked() {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);

                match self.screen {
                    Screen::SetRealWeight => self.set_real_weight(ui),
                    Screen::Menu => self.menu(ui),
                    Screen::EnterName => self.enter_name(ui),
                    Screen::EnterGuess => self.enter_guess(ui),
                    Screen::Results => self.results(ui),
                }

                ui.add_space(20.0);
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
