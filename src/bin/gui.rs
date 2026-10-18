use eframe::egui::{self, Key::P};
use pumpkin_weight::{Game, Unit};

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
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        match self.screen {
            Screen::SetRealWeight => {
                ui.heading("Enter the real weight");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.unit, Unit::Kg, "kg");
                    ui.selectable_value(&mut self.unit, Unit::Lb, "lb");
                });
                ui.horizontal(|ui| {
                    ui.label("Real Weight: ");
                    ui.text_edit_singleline(&mut self.weight_input);
                });
                if ui.button("Start").clicked() {
                    match self.unit.parse_to_kg(&self.weight_input) {
                        Ok(kg) => {
                            self.game = Some(
                                Game::new(kg, self.unit).expect("Already parsed by parse to kg"),
                            );
                            self.error = None;
                            self.screen = Screen::Menu;
                        }
                        Err(e) => {
                            self.error = Some(e.to_string());
                        }
                    }
                }
                if let Some(msg) = &self.error {
                    ui.colored_label(egui::Color32::RED, msg);
                }
            }
            Screen::Menu => {
                if ui.button("Next Player").clicked() {
                    self.screen = Screen::EnterPlayer;
                }
                if ui.button("End Game").clicked() {
                    self.screen = Screen::Results;
                }
            }

            Screen::EnterPlayer => {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.unit, Unit::Kg, "kg");
                    ui.selectable_value(&mut self.unit, Unit::Lb, "lb");
                });
                ui.horizontal(|ui| {
                    ui.label("your name");
                    ui.text_edit_singleline(&mut self.name_input);
                });
                ui.horizontal(|ui| {
                    ui.label("your guess");
                    ui.text_edit_singleline(&mut self.guess_input);
                });
                if ui.button("Add Player").clicked() {
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
                        Err(e) => {
                            self.error = Some(e.to_string());
                        }
                    }
                }
                if let Some(msg) = &self.error {
                    ui.colored_label(egui::Color32::RED, msg);
                }
            }
            Screen::Results => {
                if let Some(game) = &self.game {
                    let winners = game.winner();

                    match winners.as_slice() {
                        [] => {
                            ui.label("Noone Played :(");
                        }
                        [w] => {
                            ui.label(format!("And the winner is {}!", w.name));
                        }
                        tied => {
                            let names: Vec<&str> = tied.iter().map(|p| p.name.as_str()).collect();
                            ui.label(format!("It's a tie between {}", names.join(" and ")));
                        }
                    }

                    ui.separator();

                    egui::Grid::new("ranking")
                        .num_columns(3)
                        .striped(true)
                        .show(ui, |ui| {
                            ui.label("Player");
                            ui.label("Guess");
                            ui.label("Off by");
                            ui.end_row();

                            for p in game.ranking() {
                                ui.label(&p.name);
                                ui.label(format!("{:.2} kg", p.guess));
                                ui.label(format!("{:.2} kg", p.error(game.real_weight())));
                                ui.end_row();
                            }
                        });
                }

                if ui.button("New Game").clicked() {
                    self.game = None;
                    self.weight_input.clear();
                    self.name_input.clear();
                    self.guess_input.clear();
                    self.error = None;
                    self.screen = Screen::SetRealWeight;
                }
            }
        }
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "Pumpkin Weight",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            cc.egui_ctx.set_zoom_factor(2.0);
            Ok(Box::new(App::new()))
        }),
    )
}
