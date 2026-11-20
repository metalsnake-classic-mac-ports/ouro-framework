# ouro-framework

What the [Ouro ports](https://github.com/metalsnake-classic-mac-ports)
(Ouro-Quagmire, Ouro-S.C.Out, Ouro-Harry-the-Handsome-Executive) share
at run time: the screens every port has by the ports' rules (the game
library, adding packs, a game's menu, the level selection, the keys
page, the options page, the timeline).

The split (decided 2026-11-19): the framework holds what a screen does
and where things go - state, keys, mouse, layout. How it looks is the
game's: each port draws the screens through a `Theme` (colours, font,
boxes, its title picture behind), so a screen works the same in every
port and looks like the game it belongs to. Where a game needs more
room, the layout gets a setting; a game does not rebuild a screen.

## Crates

- `ouro-ui`: the screens. No SDL; the port maps its keyboard and gamepad
  onto `ouro_ui::Key`, gives the mouse in screen pixels and shows what a
  screen draws (a picture of the port's own type and lines of text).
  - `library`: the games the program plays, each with its fan datasets;
    the selection starts on the game played last (Return: Continue);
    "Add from a file..." and "Get packs online...".
  - `browser`: choosing a file inside the game (full screen, gamepad),
    for where dragging a file onto the window does not work.
  - `menu`: a game's main menu, a column of entries in a box the port
    places (Sammy: its title's box; Quagmire: under its title's rail,
    the rows lower to fit).
  - `levels`: the level selection, the same in every port: the levels
    with number, name and a tick when done (other entries, a story
    sequence, with a play mark, maybe locked); the chosen one's start view
    and the port's lines about it (best, first, latest run).
  - `options`: the options page: the game's settings with their values
    (Left / Right change one), rows the port adds (the keys page,
    Defaults), Back; the port applies and saves a change at once.
  - `keys`: the keys page: the actions with their key and pad button,
    Defaults, Back; the port takes the new key and swaps one in use.
  - `timeline`: rewind's snapshots as a tree (a lane per branch), the
    chosen one's picture and the port's line about it; the port keeps
    the snapshots and gives their tree.

- `ouro-play`: what the screens show, kept with its files (serde, PNG):
  - `records`: the level selection's records, first / best / latest
    done run with its deaths, in a JSON file the port names;
  - `rewind`: rewind's snapshots as a tree (any state the port can
    clone and serialise, a PNG and a line each), with ouro-ui's
    timeline tree, saved and loaded incrementally;
  - `saved`: Continue's state in a file that names its format;
  - `picture`: RGB pictures as PNG, halved for the snapshots.

Releasing a port that uses it (the Homebrew formula brings the framework
as a resource): [RELEASING.md](RELEASING.md).

## Using it

The ports find this repository next to theirs:

    [dependencies]
    ouro-ui = { path = "../ouro-framework/crates/ouro-ui" }
    ouro-play = { path = "../ouro-framework/crates/ouro-play" }

    cargo test
