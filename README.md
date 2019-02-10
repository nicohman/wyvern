# wyvern ![](https://img.shields.io/crates/v/wyvern.svg?style=flat-square) [![builds.sr.ht status](https://builds.sr.ht/~nicohman/wyvern.svg)](https://builds.sr.ht/~nicohman/wyvern?)

Wyvern is a command-line tool written in rust that is meant to make downloading GOG games and associated activities easier and faster on linux. It features: 

- Downloading games

- Installing games without need for the graphical installers

- One-command updating of games to their latest versions, while only updating files that have changed between versions.

- GOG Connect functionality so you can scan for and claim games without leaving the terminal

- Syncing save files to a filesystem backup(with integration with cloud services being worked on).

- Optional(compile with the 'eidolonint' feature) integration with [eidolon](https://git.sr.ht/~nicohman/eidolon), so that it automatically registers installed games to eidolon.

The GitHub repo is a mirror of the main [sr.ht](https://git.sr.ht/~nicohman/wyvern) repository.

## See it working

[![asciicast](https://asciinema.org/a/226434.svg)](https://asciinema.org/a/226434)

## Installation

Wyvern is available on [crates.io](https://crates.io/crates/wyvern), installable via cargo:

`cargo install wyvern`

You can download a binary that's automatically built from the latest git commit [on my website](https://demenses.net/downloads). If you want to, you can also build it from source if you have cargo installed easily:

```

git clone https://git.sr.ht/~nicohman/wyvern && cd wyvern

cargo install --path . --force

```

Plus, it's available on the AUR as [wyvern](https://aur.archlinux.org/packages/wyvern), helpfully maintained by [@PinkCathodeCat@cathoderay.tube](https://cathoderay.tube/users/PinkCathodeCat).

## Usage

Run `wyvern help` for a list of commands:

```
wyvern 1.0.0
nicohman <nicohman@demenses.net>
A simple CLI tool for installing and maintaining linux GOG games

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
    sync       Sync a game's saves to a specific location for backup
    update     Update a game if there is an update available
```

## Contributing/Reporting bugs

Please file isues at the [sr.ht issue tracker](https://todo.sr.ht/~nicohman/wyvern) and patches/pull requests should be sent to [the mailing list](https://lists.sr.ht/~nicohman/wyvern). However, I will still accept both on GitHub if need be.


## Todo

- Very happy to take feature requests!