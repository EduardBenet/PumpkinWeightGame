use eframe::egui;
use pumpkin_weight::{Game, Unit};

const BTN: [f32; 2] = [220.0, 56.0];
const BTN_SMALL: [f32; 2] = [220.0, 40.0];
const FIELD: [f32; 2] = [220.0, 36.0];
const TOGGLE: [f32; 2] = [42.0, 24.0];
const TOGGLE_GAP: f32 = 6.0;
const LOGO_PANEL_WIDTH: f32 = 150.0;

const ACCENT: egui::Color32 = egui::Color32::from_rgb(255, 117, 24);
const ERROR_RED: egui::Color32 = egui::Color32::from_rgb(255, 105, 105);

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
            self.screen = Screen::SetRealWeight;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Fixed logo column, drawn before the content so it claims its width
        // first and every screen gets the same remaining area.
        egui::Panel::left("logo_panel")
            .resizable(false)
            .exact_size(LOGO_PANEL_WIDTH)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(14.0);
                    ui.add(
                        egui::Image::new(egui::include_image!("../../assets/pumpkin_logo.png"))
                            .max_width(LOGO_PANEL_WIDTH - 28.0)
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

            ctx.set_zoom_factor(2.0);
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
