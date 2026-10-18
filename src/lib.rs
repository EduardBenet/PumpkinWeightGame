use std::fmt;

/// Two guesses within this many kilograms of each other count as a tie.
///
/// Guesses are normalised to kg on the way in, and that conversion rounds:
/// 19 lb and 21 lb are both exactly 1 lb from 20 lb, but in `f32` their
/// errors differ by about 1e-6 kg. Comparing with `==` would hand the win
/// to whichever one happened to round low. A gram is far below any
/// difference a person could mean, and a thousand times the rounding noise.
const TIE_TOLERANCE_KG: f32 = 0.001;

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
    /// Inverse of [`Unit::label`], for reading a saved game back.
    pub fn from_label(label: &str) -> Option<Self> {
        match label.trim() {
            "kg" => Some(Unit::Kg),
            "lb" => Some(Unit::Lb),
            _ => None,
        }
    }

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

/// First line of a saved game. Bumping the number makes older files load as
/// [`LoadError::UnknownVersion`] rather than being misread.
const SAVE_HEADER: &str = "pumpkin-weight 1";

/// Separates a guess from a name in a saved line. A tab, because the
/// on-screen keyboard cannot produce one and so a name can never contain it.
const SAVE_SEPARATOR: char = '\t';

#[derive(Debug, PartialEq)]
pub enum LoadError {
    /// Missing or unrecognised header line.
    UnknownVersion,
    /// A line could not be understood, or the weight was absent.
    Malformed,
}

impl std::error::Error for LoadError {}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let msg = match self {
            LoadError::UnknownVersion => "saved game is from a different version",
            LoadError::Malformed => "saved game is damaged",
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

    pub fn players(&self) -> &[Player] {
        &self.players
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

    /// Render the whole game as text, for a caller to persist however it
    /// likes. Weights are kg, as they are everywhere inside `Game`.
    pub fn to_save_string(&self) -> String {
        let mut out = format!(
            "{}\nweight {}\nunit {}\n",
            SAVE_HEADER,
            self.real_weight,
            self.unit.label()
        );

        for player in &self.players {
            out.push_str(&format!(
                "player {}{}{}\n",
                player.guess, SAVE_SEPARATOR, player.name
            ));
        }

        out
    }

    /// Rebuild a game from [`Game::to_save_string`].
    ///
    /// Players go back in through `add_player`, so a file that has been
    /// hand-edited into an invalid state (duplicate names, say) is rejected
    /// rather than loaded into a `Game` that breaks its own guarantees.
    pub fn from_save_string(text: &str) -> Result<Self, LoadError> {
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty());

        if lines.next() != Some(SAVE_HEADER) {
            return Err(LoadError::UnknownVersion);
        }

        let mut weight: Option<f32> = None;
        let mut unit = Unit::Kg;
        let mut players: Vec<(String, f32)> = Vec::new();

        for line in lines {
            let (key, value) = line.split_once(' ').ok_or(LoadError::Malformed)?;

            match key {
                "weight" => {
                    weight = Some(value.parse().map_err(|_| LoadError::Malformed)?);
                }
                "unit" => {
                    unit = Unit::from_label(value).ok_or(LoadError::Malformed)?;
                }
                "player" => {
                    let (guess, name) = value
                        .split_once(SAVE_SEPARATOR)
                        .ok_or(LoadError::Malformed)?;
                    let guess: f32 = guess.parse().map_err(|_| LoadError::Malformed)?;
                    players.push((name.to_string(), guess));
                }
                _ => return Err(LoadError::Malformed),
            }
        }

        let weight = weight.ok_or(LoadError::Malformed)?;
        let mut game = Game::new(weight, unit).map_err(|_| LoadError::Malformed)?;

        for (name, guess) in players {
            game.add_player(&name, guess)
                .map_err(|_| LoadError::Malformed)?;
        }

        Ok(game)
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
            .filter(|p| (p.error(real) - best).abs() <= TIE_TOLERANCE_KG)
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

    /// Regression: 19 lb and 21 lb are both exactly 1 lb from 20 lb, but the
    /// conversion to kg rounds, so their errors are ~1e-6 apart in `f32`.
    /// Exact equality picked a single winner; the tolerance restores the tie.
    #[test]
    fn ties_survive_unit_conversion() {
        let real = Unit::Lb.parse_to_kg("20").unwrap();
        let low = Unit::Lb.parse_to_kg("19").unwrap();
        let high = Unit::Lb.parse_to_kg("21").unwrap();

        let mut game = Game::new(real, Unit::Lb).unwrap();
        game.add_player("Low", low).unwrap();
        game.add_player("High", high).unwrap();

        assert_eq!(game.winner().len(), 2);
    }

    /// The tolerance must not swallow differences people actually mean.
    #[test]
    fn near_misses_are_not_ties() {
        let mut game = Game::new(10.0, Unit::Kg).unwrap();
        game.add_player("Close", 10.05).unwrap();
        game.add_player("Closer", 10.0).unwrap();

        let winners = game.winner();
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].name, "Closer");
    }

    #[test]
    fn save_and_load_round_trip() {
        let mut game = Game::new(9.5, Unit::Lb).unwrap();
        game.add_player("Bob", 9.0).unwrap();
        game.add_player("Ada Lovelace", 10.25).unwrap();

        let restored = Game::from_save_string(&game.to_save_string()).unwrap();

        assert_eq!(restored.real_weight(), 9.5);
        assert_eq!(restored.unit(), Unit::Lb);
        assert_eq!(restored.players().len(), 2);
        assert_eq!(restored.players()[1].name, "Ada Lovelace");
        assert_eq!(restored.players()[1].guess, 10.25);
    }

    #[test]
    fn load_rejects_a_foreign_file() {
        assert_eq!(
            Game::from_save_string("hello\n").unwrap_err(),
            LoadError::UnknownVersion
        );
        assert_eq!(
            Game::from_save_string("").unwrap_err(),
            LoadError::UnknownVersion
        );
    }

    #[test]
    fn load_rejects_damage() {
        let good = {
            let mut g = Game::new(9.5, Unit::Kg).unwrap();
            g.add_player("Bob", 9.0).unwrap();
            g.to_save_string()
        };

        // A truncated weight line.
        let bad = good.replace("weight 9.5", "weight ");
        assert_eq!(
            Game::from_save_string(&bad).unwrap_err(),
            LoadError::Malformed
        );

        // A duplicate name that `add_player` would never have allowed.
        let bad = format!("{good}player 1\tBob\n");
        assert_eq!(
            Game::from_save_string(&bad).unwrap_err(),
            LoadError::Malformed
        );
    }

    #[test]
    fn load_needs_a_weight() {
        let text = format!("{}\nunit kg\n", super::SAVE_HEADER);
        assert_eq!(
            Game::from_save_string(&text).unwrap_err(),
            LoadError::Malformed
        );
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
