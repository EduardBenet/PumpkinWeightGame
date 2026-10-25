# Pumpkin Weight

<img src="assets/pumpkin_logo.png" alt="" width="120" align="right">

A guess-the-weight game for a Raspberry Pi with a touchscreen. The host
weighs a pumpkin and enters the real weight; players take turns entering a
name and a guess; whoever is closest wins.

Built for a stall: everything is driven by on-screen buttons, there is no
keyboard, and the game survives the power being pulled.

---

## How it works

Five screens, driven by a state machine — no keyboard, no mouse, no caret.

| screen | what happens |
|---|---|
| **Set real weight** | host enters the true weight, in kg or lb |
| **Menu** | player count, and the way in and out of a round |
| **Enter name** | on-screen qwerty, max 12 characters |
| **Enter guess** | on-screen keypad, in kg or lb |
| **Results** | winner (or winners, on a tie), then the full leaderboard |

Guesses are converted to kilograms the moment they are entered, so players
may each use whichever unit they think in and the comparison stays correct.
The leaderboard always reads in kilograms.

Ties are real: two guesses an equal distance either side of the truth both
win.

## Project layout

```
src/lib.rs       the rules: Game, Player, Unit, errors, save format
src/bin/gui.rs   the touchscreen app (eframe/egui)
src/main.rs      a terminal version of the same game, using the same library
assets/          the logo, embedded into the binary at compile time
```

`lib.rs` performs no input or output. Every rule — valid weights, unique
names, unit conversion, ranking, ties, the save format — lives there and is
covered by tests. The two front ends only collect text and draw results.

```bash
cargo test                      # 16 tests, no display needed
cargo run --bin gui             # the touchscreen app
cargo run --bin pumpkin-weight  # the terminal version
```

---

## Building on a Raspberry Pi

Assumes a Pi 4 with the official 7" DSI touchscreen (800x480), running
**Raspberry Pi OS Lite** (no desktop).

### 1. System packages

```bash
sudo apt update
sudo apt install -y \
    build-essential pkg-config git \
    cage seatd \
    libwayland-egl1 libegl1 libgles2 \
    libxkbcommon-dev libgl1-mesa-dev \
    libx11-dev libxcursor-dev libxrandr-dev libxi-dev
```

`cage` is a kiosk compositor that runs exactly one fullscreen application.
Lite ships no compositor at all, and eframe cannot draw without one.

### 2. Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 3. Build

```bash
git clone https://github.com/EduardBenet/PumpkinWeightGame.git
cd PumpkinWeightGame
cargo build --release --bin gui
```

The first build takes 20–40 minutes; later ones are incremental and quick.

### 4. Run it

From the Pi's own console, not over SSH:

```bash
cage -- ~/PumpkinWeightGame/target/release/gui
```

---

## Starting automatically at boot

### 1. Log in without a prompt


```bash
sudo raspi-config nonint do_boot_behaviour B2
```

### 2. Launch the game from that login

Append to your login profile — bash reads the *first* of `~/.bash_profile`,
`~/.bash_login`, `~/.profile` that exists and ignores the rest, so put it in
the one you already have:

```sh
# Kiosk: start the game on the console only, never over SSH.
if [ "$(tty)" = "/dev/tty1" ] && [ -z "$WAYLAND_DISPLAY" ]; then
    cage -- "$HOME/PumpkinWeightGame/target/release/gui" >>"$HOME/pumpkin.log" 2>&1
fi
```

## Using it

**Exit:** press `Esc`, or tap the small `exit` link under the logo. It is
deliberately dim and small — reachable when you want it, unappealing to a
four-year-old.

**Crash recovery:** the game is written to
`~/.local/share/pumpkin-weight/resume.txt` when the weight is set and after
every player is added, and read back at startup. Pull the plug mid-session
and it resumes where it left off.

The file is deleted by **End game**, so one existing means the last session
ended unexpectedly. A half-finished entry is not saved — only committed
players are.

Saving is best effort: a failure is reported to stderr and never interrupts
play. A damaged file is reported, deleted and ignored, so a corrupt save
cannot wedge the kiosk into failing at every boot.

---

## Tuning for a different display

All in `src/bin/gui.rs`:

| constant | what it does |
|---|---|
| `ZOOM` | global UI scale, `1.2` for the 800x480 panel |
| `LOGO_PANEL_FRACTION` | width of the logo column, as a fraction |
| `KEY` / `NUMKEY` | on-screen key sizes |
| `BTN` / `ACTION` / `FIELD` | button and field sizes |

`ZOOM` first — it scales everything at once, and the rest are proportions
within whatever it gives you.

---

## Licence

[PolyForm Noncommercial 1.0.0](LICENSE.txt) — free to use, copy and modify
for any noncommercial purpose, including hobby, education and charity.
Commercial use needs a separate arrangement.

Note this is a *source available* licence rather than an open source one:
the Open Source Definition does not permit restrictions on the field of
use, so a noncommercial clause disqualifies a licence by definition. If you
would rather the project be genuinely open source, MIT or Apache-2.0 are
the usual choices — both allow commercial use.
