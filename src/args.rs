use std::path::PathBuf;
#[derive(StructOpt, Debug)]
#[structopt(name = "wyvern")]
pub enum Wyvern {
    #[structopt(name = "ls", about = "List all games you own")]
    List {
        #[structopt(short = "i", long = "id", help = "search with id")]
        id: Option<i64>,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "down", about = "Download specific game")]
    Download {
        #[structopt(short = "i", long = "id", help = "download id")]
        id: Option<i64>,
        #[structopt(short = "s", long = "search", help = "search manually")]
        search: Option<String>,
        #[structopt(parse(from_os_str))]
        #[structopt(
            short = "n",
            long = "install",
            help = "install downloaded game to path"
        )]
        install_after: Option<PathBuf>,
        #[structopt(
            short = "w",
            long = "windows-auto",
            help = "Download windows version if no linux is available"
        )]
        windows_auto: bool,
        #[structopt(long = "force-windows", help = "Force downloading windows version")]
        windows_force: bool,
        #[structopt(
            short = "f",
            long = "first",
            help = "When searching, use first result without waiting for selection"
        )]
        first: bool,
        #[structopt(short = "a", long = "all", help = "Download all games in your library")]
        all: bool,
        #[structopt(
            short = "d",
            long = "desktop",
            help = "Add a desktop shortcut for the installed game"
        )]
        desktop: bool,
        #[structopt(
            short = "m",
            long = "menu",
            help = "Add an application menu shortcut for the installed game"
        )]
        menu: bool,
        #[structopt(
            short = "c",
            long = "shortcuts",
            help = "Add both kinds of shortcuts for the installed game"
        )]
        shortcuts: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[cfg(feature = "eidolonint")]
    #[structopt(
        name = "update-eidolon",
        about = "Update all eidolon-registered GOG games"
    )]
    UpdateEidolon {
        force: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "connect", about = "Operations associated with GOG Connect")]
    Connect(Connect),
    #[structopt(name = "install", about = "Install a GOG game from an installer")]
    Install {
        installer_name: String,
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        #[structopt(
            short = "d",
            long = "desktop",
            help = "Add a desktop shortcut for the installed game"
        )]
        desktop: bool,
        #[structopt(
            short = "m",
            long = "menu",
            help = "Add an application menu shortcut for the installed game"
        )]
        menu: bool,
        #[structopt(
            short = "c",
            long = "shortcuts",
            help = "Add both kinds of shortcuts for the installed game"
        )]
        shortcuts: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(
        name = "update",
        about = "Update a game if there is an update available"
    )]
    Update {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,
        #[structopt(short = "f", long = "force", help = "Force updating even if unneeded")]
        force: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
        #[structopt(short = "d", long = "delta", help = "Update only changed files")]
        delta: bool,
    },
    #[structopt(
        name = "sync",
        about = "Sync a game's saves to a specific location for backup"
    )]
    Sync(Sync),
}
#[derive(StructOpt, Debug)]
pub enum Sync {
    #[structopt(name = "saves", about = "Configure where a game's saves are located")]
    Saves {
        #[structopt(parse(from_os_str))]
        game_dir: PathBuf,
        #[structopt(parse(from_os_str))]
        saves: PathBuf,
        #[structopt(short = "d", long = "db", help = "Db to save config to")]
        #[structopt(parse(from_os_str))]
        db: Option<PathBuf>,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "push", about = "Push save files to sync location")]
    Push {
        #[structopt(parse(from_os_str))]
        game_dir: PathBuf,
        #[structopt(parse(from_os_str))]
        sync_to: Option<PathBuf>,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "pull", about = "Pull synced save files")]
    Pull {
        #[structopt(parse(from_os_str))]
        game_dir: PathBuf,
        #[structopt(parse(from_os_str))]
        sync_from: Option<PathBuf>,
        #[structopt(short = "f", long = "force", help = "Force syncing even if unneeded")]
        force: bool,
        #[structopt(
            short = "i",
            long = "ignore",
            help = "Automatically refuse syncing save files that are older than the current"
        )]
        ignore_older: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "db-pull", about = "Pull all save files from a database")]
    DbPull {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,
        #[structopt(short = "f", long = "force", help = "Force syncing even if unneeded")]
        force: bool,
        #[structopt(
            short = "i",
            long = "ignore",
            help = "Automatically refuse syncing save files that are older than the current"
        )]
        ignore_older: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "db-push", about = "Push all save files in a database")]
    DbPush {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,
        #[structopt(short = "f", long = "force", help = "Force syncing even if unneeded")]
        force: bool,
        #[structopt(
            short = "i",
            long = "ignore",
            help = "Automatically refuse pushing save files that are older than the current"
        )]
        ignore_older: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
}
#[derive(StructOpt, Debug)]
pub enum Connect {
    #[structopt(name = "ls", about = "List available GOG Connect games")]
    ListConnect {
        #[structopt(
            short = "c",
            long = "claimable",
            help = "only show games that are currently claimable"
        )]
        claim: bool,
        #[structopt(short = "q", long = "quiet", help = "only print game names")]
        quiet: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
    #[structopt(name = "claim", about = "Claim all available GOG Connect games")]
    ClaimAll {
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
    },
}
