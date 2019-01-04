# wyvern

Wyvern is a command-line tool written in rust that is meant to make downloading GOG games and associated activities easier and faster on linux. It features: 

- Downloading games

- Installing games without need for the graphical installers

- One-command updating of games to their latest versions.

- GOG Connect functionality so you can scan for and claim games without leaving the terminal

- Optional(compile with the 'eidolonint' feature) integration with [eidolon](https://git.sr.ht/~nicohman/eidolon), so that it automatically registers installed games to eidolon.

The GitHub repo is a mirror of the main [sr.ht](https://git.sr.ht/~nicohman/wyvern) repository.

## Installation

Right now, wyvern is still in alpha, so it's not on crates.io yet. If you want to use it, you can build it from source if you have cargo installed easily:

```

git clone https://git.sr.ht/~nicohman/wyvern && cd wyvern

cargo install --path . --force

```

## Usage

Run `wyvern help` for a list of commands:

```
wyvern 0.1.0
nicohman <nicohman@demenses.net>

USAGE:
    wyvern <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    connect    Operations associated with GOG Connect
    down       Download specific game
    help       Prints this message or the help of the given subcommand(s)
    install    Install a GOG game from an installer
    ls         List all games you own
    update     Update a game if there is an update available

```

## Contributing/Reporting bugs

Please file isues at the [sr.ht issue tracker](https://todo.sr.ht/~nicohman/wyvern) and patches/pull requests should be sent to [the mailing list](https://lists.sr.ht/~nicohman/wyvern). However, I will still accept both on GitHub if need be.


## Todo

- Add in some form of cloud save support(Users will have to specify where their saves are for this to work)

- Very happy to take feature requests!