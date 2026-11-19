# ouro-framework

What the [Ouro ports](https://github.com/metalsnake-classic-mac-ports)
(Ouro-Quagmire, Ouro-S.C.Out, Ouro-Harry-the-Handsome-Executive) share
at run time: the screens every port has by the ports' rules (the game
library, adding packs; later the keys page, the level selection, Continue
and the timeline).

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

## Using it

The ports find this repository next to theirs:

    [dependencies]
    ouro-ui = { path = "../ouro-framework/crates/ouro-ui" }

    cargo test
