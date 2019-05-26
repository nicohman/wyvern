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

There's also a few other ways to get wyvern:

- AUR: [wyvern](https://aur.archlinux.org/packages/wyvern), maintained by
  [@PinkCathodeCat@cathoderay.tube](https://cathoderay.tube/users/PinkCathodeCat)
  
- snap: [wyvern](https://snapcraft.io) on snapcraft.io

- Download a binary, built from the latest git commit from [my website](https://demenses.net/downloads)

- OpenSuse package
  [here](https://software.opensuse.org//download.html?project=home:stryan&package=wyvern),
  maintained by Steve Ryan

- Build from source:

```

git clone https://git.sr.ht/~nicohman/wyvern && cd wyvern

cargo install --path . --force

```

### Dependencies

Wyvern has a few extra dependencies, but few are required:
- rsync for save file syncing
- innoextract for windows game installation
- unzip for faster game installation

## Usage

Run `wyvern help` for a list of commands:

```
wyvern 1.4.0
nicohman <nicohman@demenses.net>
A simple CLI tool for installing and maintaining linux GOG games

USAGE:
    wyvern [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help         Prints help information
    -V, --version      Prints version information
    -v, --verbosity    Pass many times for more log output

SUBCOMMANDS:
    connect    Operations associated with GOG Connect
    down       Download specific game
    extras     Download a game's extras
    help       Prints this message or the help of the given subcommand(s)
    install    Install a GOG game from an installer
    int        Enter interactive mode
    login      Force a login to GOG
    ls         List all games you own
    sync       Sync a game's saves to a specific location for backup
    update     Update a game if there is an update available
```

## Contributing/Reporting bugs

Please file isues at the [sr.ht issue tracker](https://todo.sr.ht/~nicohman/wyvern) and patches/pull requests should be sent to [the mailing list](https://lists.sr.ht/~nicohman/wyvern). However, I will still accept both on GitHub if need be.


