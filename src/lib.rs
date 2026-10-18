use std::fmt;

#[derive(Debug)]
pub struct Player {
    pub name: String,
    pub guess: f32,
}

impl Player {
    pub fn error(&self, real_weight: f32) -> f32 {
        (self.guess - real_weight).abs()
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Unit {
    Kg,
    Lb,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::Kg => "kg",
            Unit::Lb => "lb",
        }
    }

    pub fn to_kg(self, weight: f32) -> f32 {
        match self {
            Unit::Kg => weight,
            Unit::Lb => weight * 0.453592,
        }
    }

    pub fn from_kg(self, weight: f32) -> f32 {
        match self {
            Unit::Kg => weight,
            Unit::Lb => weight / 0.453592,
        }
    }

    pub fn parse_to_kg(self, input: &str) -> Result<f32, WeightError> {
        let weight = match input.parse::<f32>() {
            Ok(v) => v,
            Err(_) => {
                return Err(WeightError::NotANumber);
            }
        };

        if weight <= 0.0 {
            return Err(WeightError::NotPositive);
        }

        Ok(self.to_kg(weight))
    }
}

#[derive(Debug, PartialEq)]
pub enum WeightError {
    NotANumber,
    NotPositive,
}

impl std::error::Error for WeightError {}

impl fmt::Display for WeightError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            WeightError::NotANumber => "that is not a number",
            WeightError::NotPositive => "a weights must be positive",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug, PartialEq)]
pub enum AddPlayerError {
    DuplicateName,
    EmptyName,
}

impl std::error::Error for AddPlayerError {}

impl fmt::Display for AddPlayerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            AddPlayerError::DuplicateName => "this name already exists",
            AddPlayerError::EmptyName => "a name can't be empty",
        };
        write!(f, "{}", msg)
    }
}

#[derive(Debug)]
pub struct Game {
    real_weight: f32, // always kg
    unit: Unit,       // display preference
    players: Vec<Player>,
}

impl Game {
    pub fn new(real_weight: f32, unit: Unit) -> Result<Self, WeightError> {
        if real_weight <= 0.0 {
            return Err(WeightError::NotPositive);
        }
        Ok(Self {
            real_weight: real_weight,
            unit: unit,
            players: Vec::new(),
        })
    }

    pub fn real_weight(&self) -> f32 {
        self.real_weight
    }

    pub fn unit(&self) -> Unit {
        self.unit
    }

    pub fn check_name(&self, name: &str) -> Result<(), AddPlayerError> {
        let name = name.trim();

        if name.is_empty() {
            return Err(AddPlayerError::EmptyName);
        }

        if self
            .players
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(name))
        {
            return Err(AddPlayerError::DuplicateName);
        }

        Ok(())
    }

    pub fn add_player(&mut self, name: &str, guess: f32) -> Result<(), AddPlayerError> {
        self.check_name(name)?;

        self.players.push(Player {
            name: name.trim().to_string(),
            guess,
        });

        Ok(())
    }

    pub fn ranking(&self) -> Vec<&Player> {
        let mut ranked: Vec<&Player> = self.players.iter().collect();

        ranked.sort_by(|a, b| {
            let da = (a.guess - self.real_weight).abs();
            let db = (b.guess - self.real_weight).abs();
            da.total_cmp(&db)
        });
        ranked
    }

    pub fn winner(&self) -> Vec<&Player> {
        let real = self.real_weight;

        let Some(best) = self
            .players
            .iter()
            .map(|p| p.error(real))
            .min_by(f32::total_cmp)
        else {
            return Vec::new();
        };

        self.players
            .iter()
            .filter(|p| p.error(real) == best)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_number() {
        assert_eq!(Unit::Kg.parse_to_kg("10"), Ok(10.0))
    }

    #[test]
    fn reject_text() {
        assert_eq!(Unit::Kg.parse_to_kg("abc"), Err(WeightError::NotANumber));
    }

    #[test]
    fn reject_negative() {
        assert_eq!(Unit::Lb.parse_to_kg("-1"), Err(WeightError::NotPositive))
    }

    #[test]
    fn reject_zero() {
        assert_eq!(
            Game::new(0.0, Unit::Kg).unwrap_err(),
            WeightError::NotPositive
        )
    }

    #[test]
    fn reject_duplicates() {
        let mut game = Game::new(3.0, Unit::Kg).unwrap();
        game.add_player("Bob", 3.0).unwrap();
        assert_eq!(
            game.add_player("Bob", 2.0),
            Err(AddPlayerError::DuplicateName)
        )
    }

    #[test]
    fn ties_return_both() {
        let mut game = Game::new(3.0, Unit::Kg).unwrap();
        game.add_player("Bob", 3.0).unwrap();
        game.add_player("Bob2", 3.0).unwrap();
        let winner = game.winner();
        assert_eq!(winner.len(), 2)
    }

    #[test]
    fn empty_game() {
        let game = Game::new(3.0, Unit::Kg).unwrap();
        assert_eq!(game.winner().len(), 0)
    }

    #[test]
    fn convert_pounds() {
        let kg = Unit::Lb.parse_to_kg("1").unwrap();
        assert!((kg / 0.453592 - 1.0).abs() < 1e-5, "got {}", kg)
    }

    #[test]
    fn rejects_whitespace_name() {
        let mut game = Game::new(3.0, Unit::Kg).unwrap();
        assert_eq!(game.add_player("  ", 4.4), Err(AddPlayerError::EmptyName))
    }

    #[test]
    fn duplicate_ignores_case_and_padding() {
        let mut game = Game::new(3.9, Unit::Lb).unwrap();
        game.add_player("Bob", 3.0).unwrap();
        assert_eq!(
            game.add_player("bob ", 3.2),
            Err(AddPlayerError::DuplicateName)
        )
    }
}
