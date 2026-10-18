use pumpkin_weight::{AddPlayerError, Game, Unit};

use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::Select;

#[derive(PartialEq)]
enum Confirmation {
    Ask,
    Skip,
}

fn get_input(prompt: &str, units: Unit) -> String {
    let choices = match units {
        Unit::Kg => "[kg]/lb",
        Unit::Lb => "kg/[lb]",
    };

    let prompt2 = format!("{} (or select {})", prompt, choices);
    Input::<String>::new()
        .with_prompt(prompt2)
        .interact_text()
        .unwrap()
        .trim()
        .to_string()
}

fn request_weight(prompt: &str, confirm: Confirmation) -> (f32, Unit) {
    let mut units = Unit::Kg;

    loop {
        let input = get_input(prompt, units);

        match input.as_str() {
            "kg" => {
                units = Unit::Kg;
                continue;
            }
            "lb" => {
                units = Unit::Lb;
                continue;
            }
            _ => {}
        }

        let weight = match units.parse_to_kg(&input) {
            Ok(v) => v,
            Err(e) => {
                println!("'{}': {}", input, e);
                continue;
            }
        };

        if confirm == Confirmation::Ask {
            let prompt = format!(
                "You entered {} {}, are you sure?",
                units.from_kg(weight),
                units.label()
            );
            let sure = Confirm::new()
                .with_prompt(prompt)
                .default(true)
                .interact()
                .unwrap();

            if !sure {
                continue;
            }
        }

        break (weight, units);
    }
}

fn new_guess(game: &mut Game) -> Result<(), AddPlayerError> {
    let name = loop {
        let name: String = Input::new()
            .with_prompt("Your name?")
            .interact_text()
            .unwrap();

        match game.check_name(&name) {
            Ok(_) => break name,
            Err(e) => {
                println!("'{}': {}", name, e)
            }
        }
    };

    let (guess_weight, _) = request_weight("How heavy is the pumpkin?", Confirmation::Skip);

    game.add_player(&name, guess_weight)
}

fn main() {
    let (real_weight, unit) =
        request_weight("Enter the real weight of the pumpkin", Confirmation::Ask);
    println!(
        "Starting game with {} {}",
        unit.from_kg(real_weight),
        unit.label()
    );

    // Starting the game
    let mut game =
        Game::new(real_weight, unit).expect("request_weight only returns positive weights");

    const MENU: [&str; 2] = ["Next Player", "End game"];
    loop {
        let selection = Select::new().items(MENU).default(0).interact().unwrap();

        if MENU[selection] == "End game" {
            break;
        }

        if let Err(e) = new_guess(&mut game) {
            println!("Could not add player: {}", e);
        }
    }

    let ranking = game.ranking();
    let winners = game.winner();

    match winners.as_slice() {
        [] => println!("Nobody played :("),
        [winner] => println!("And the winner is {}!", winner.name),
        tied => {
            let names: Vec<&str> = tied.iter().map(|p| p.name.as_str()).collect();
            println!("It's a tie between {}!", names.join(" and "))
        }
    }

    println!("Leader board:");

    for p in &ranking {
        println!(
            "{} guessed {} {} - off by {:.2}",
            p.name,
            game.unit().from_kg(p.guess),
            game.unit().label(),
            game.unit().from_kg(p.error(game.real_weight()))
        );
    }
}
