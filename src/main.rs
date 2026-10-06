use dialoguer::Confirm;
use dialoguer::Input;
use dialoguer::Select;
use std::io;

#[derive(PartialEq)]
enum Confirmation {
    Ask,
    Skip,
}

struct Player {
    name: String,
    guess: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Unit {
    Kg,
    Lb,
}

impl Unit {
    fn label(self) -> &'static str {
        match self {
            Unit::Kg => "kg",
            Unit::Lb => "lb",
        }
    }

    fn to_kg(self, weight: f32) -> f32 {
        match self {
            Unit::Kg => weight,
            Unit::Lb => weight * 0.453592,
        }
    }
}

fn get_input(prompt: &str, units: Unit) -> String {
    let choices = match units {
        Unit::Kg => "[kg]/lb",
        Unit::Lb => "kg/[lb]",
    };
    println!("{} (or select {}):", prompt, choices);

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn request_weight(prompt: &str, confirm: Confirmation) -> f32 {
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

        let weight = match input.parse::<f32>() {
            Ok(v) => v,
            Err(_) => {
                println!("'{}' is not a number", input);
                continue;
            }
        };

        if weight <= 0.0 {
            println!("This was invalid, a weight is positive");
            continue;
        }

        if confirm == Confirmation::Ask {
            let prompt = format!("You entered {} {}, are you sure?", weight, units.label());
            let sure = Confirm::new()
                .with_prompt(prompt)
                .default(true)
                .interact()
                .unwrap();

            if !sure {
                continue;
            }
        }

        break units.to_kg(weight);
    }
}

fn new_guess(results: &Vec<Player>) -> Player {
    let name = loop {
        let name: String = Input::new()
            .with_prompt("Your name?")
            .interact_text()
            .unwrap();

        if results.iter().any(|p| p.name == name) {
            println!("Name already exists..");
            continue;
        }

        break name;
    };

    let weight = request_weight("How heavy is the pumpkin?", Confirmation::Skip);

    Player {
        name: name,
        guess: weight,
    }
}

fn main() {
    let real_weight = request_weight("Enter the real weight of the pumpkin", Confirmation::Ask);
    println!("Starting game with {} kg", real_weight);

    // Starting game
    let mut results: Vec<Player> = Vec::new();
    const MENU: [&str; 2] = ["Next Player", "End game"];
    loop {
        let selection = Select::new().items(&MENU).default(0).interact().unwrap();

        if MENU[selection] == "End game" {
            break;
        }

        let entry = new_guess(&results);

        results.push(entry);
    }

    if results.is_empty() {
        println!("Nobody played :(");
        return;
    }

    results.sort_by(|a, b| {
        let da = (a.guess - real_weight).abs();
        let db = (b.guess - real_weight).abs();
        da.total_cmp(&db)
    });

    for p in &results {
        println!(
            "{} guessed {} kg - off by {:.2}",
            p.name,
            p.guess,
            (p.guess - real_weight).abs()
        );
    }
    println!("And the winner is {}", results[0].name);
}
