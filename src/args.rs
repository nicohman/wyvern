use std::path::PathBuf;
#[derive(StructOpt, Debug)]
#[structopt(name = "wyvern")]
pub struct Wyvern {
    #[structopt(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity,
    #[structopt(subcommand)]
    pub command: Command,
}
#[derive(StructOpt, Debug)]
pub enum Command {
    #[structopt(name = "ls", alias = "list", about = "List all games you own")]
    List {
        #[structopt(short = "i", long = "id", help = "search with id")]
        id: Option<i64>,
        #[structopt(short = "j", long = "json", help = "Display games in JSON format")]
        json: bool,
    },
    #[structopt(name = "down", alias = "download", about = "Download specific game")]
    Download {
        #[structopt(flatten)]
        options: DownloadOptions,
        #[structopt(flatten)]
        shortcuts: ShortcutOptions,
    },
    #[structopt(name = "extras", about = "Download a game's extras")]
    Extras {
        #[structopt(short = "a", long = "all", help = "Download all available extras")]
        all: bool,
        #[structopt(short = "f", long = "first", help = "Download the first search result")]
        first: bool,
        #[structopt(short = "i", long = "id", help = "Download a game's extras by id")]
        id: Option<i64>,
        #[structopt(parse(from_os_str))]
        #[structopt(short = "o", long = "output-folder", help = "Name of folder to output extras to")]
        output: Option<PathBuf>,
        game: Option<String>,
        #[structopt(short = "s", long = "slug", help =  "Download a single extra by slug name")]
        slug: Option<String>
    },
    #[derive(Default)]
    #[cfg(feature = "eidolonint")]
    #[structopt(
        name = "update-eidolon",
        about = "Update all eidolon-registered GOG games"
    )]
    UpdateEidolon {
        force: bool,
        #[structopt(flatten)]
        verbose: clap_verbosity_flag::Verbosity,
        #[structopt(short = "d", long = "delta", help = "Update only changed files")]
        delta: bool,
    },
    #[structopt(name = "connect", about = "Operations associated with GOG Connect")]
    Connect(Connect),
    #[structopt(name = "install", about = "Install a GOG game from an installer")]
    Install {
        installer_name: String,
        #[structopt(parse(from_os_str))]
        path: PathBuf,
        #[structopt(flatten)]
        shortcuts: ShortcutOptions,
        #[structopt(short = "w", long = "windows", help = "Install a windows game")]
        windows: bool,
        #[structopt(
            short = "e",
            long = "external-zip",
            help = "Use the zip CLI tool to unzip the installer. Faster."
        )]
        external_zip: bool,
    },
    #[structopt(
        name = "update",
        about = "Update a game if there is an update available"
    )]
    Update {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,
        #[structopt(short = "d", long = "dlc", help = "Update with all DLCs")]
        dlc: bool,
    },
    #[structopt(
        name = "sync",
        about = "Sync a game's saves to a specific location for backup"
    )]
    Sync(Sync),
    #[structopt(name = "int", alias = "interactive", about = "Enter interactive mode")]
    Interactive,
    #[structopt(name = "login", about = "Force a login to GOG")]
    Login {
        #[structopt(short = "u", long = "username", help = "Username to log in with")]
        username: Option<String>,
        #[structopt(short = "p", long = "password", help = "Password to log in with")]
        password: Option<String>,
        #[structopt(short = "c", long = "code", help = "Use a login code to log in")]
        code: Option<String>
    }
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
    },
    #[structopt(name = "push", about = "Push save files to sync location")]
    Push {
        #[structopt(parse(from_os_str))]
        game_dir: PathBuf,
        #[structopt(parse(from_os_str))]
        sync_to: Option<PathBuf>,
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
    },
}
#[derive(StructOpt, Debug)]
pub enum Connect {
    #[structopt(
        name = "ls",
        alias = "list",
        about = "List available GOG Connect games"
    )]
    ListConnect {
        #[structopt(
            short = "c",
            long = "claimable",
            help = "only show games that are currently claimable"
        )]
        claim: bool,
        #[structopt(short = "q", long = "quiet", help = "Only print game names")]
        quiet: bool,
        #[structopt(short = "j", long = "json", help = "Print results in JSON format")]
        json: bool,
    },
    #[structopt(name = "claim", about = "Claim all available GOG Connect games")]
    ClaimAll,
}
#[derive(StructOpt, Debug)]
pub struct ShortcutOptions {
    #[structopt(
        short = "d",
        long = "desktop",
        help = "Add a desktop shortcut for the installed game"
    )]
    pub desktop: bool,
    #[structopt(
        short = "m",
        long = "menu",
        help = "Add an application menu shortcut for the installed game"
    )]
    pub menu: bool,
    #[structopt(
        short = "c",
        long = "shortcuts",
        help = "Add both kinds of shortcuts for the installed game"
    )]
    pub shortcuts: bool,
}
#[derive(StructOpt, Debug, Default)]
pub struct DownloadOptions {
    #[structopt(short = "i", long = "id", help = "download id")]
    pub id: Option<i64>,
    #[structopt(help = "search manually")]
    pub search: Option<String>,
    #[structopt(parse(from_os_str))]
    #[structopt(
        short = "n",
        long = "install",
        help = "install downloaded game to path"
    )]
    pub install_after: Option<PathBuf>,
    #[structopt(
        short = "w",
        long = "windows-auto",
        help = "Download windows version if no linux is available"
    )]
    pub windows_auto: bool,
    #[structopt(long = "force-windows", help = "Force downloading windows version")]
    pub windows_force: bool,
    #[structopt(
        short = "f",
        long = "first",
        help = "When searching, use first result without waiting for selection"
    )]
    pub first: bool,
    #[structopt(short = "a", long = "all", help = "Download all games in your library")]
    pub all: bool,
    #[structopt(short = "D", long = "dlc", help = "Download DLCs as well")]
    pub dlc: bool,
    #[structopt(short = "r", long = "resume", help = "Resume downloading games")]
    pub resume: bool,
    #[structopt(
        short = "O",
        long = "no-original-name",
        help = "Don't preserve the original game name"
    )]
    pub original: bool,
    #[structopt(parse(from_os_str))]
    #[structopt(
        short = "o",
        long = "output",
        help = "Write downloaded file to target location. Note: if the file already exists/multiple files are downloaded, appends a count to the end"
    )]
    pub output: Option<PathBuf>,
    #[structopt(
        long = "preserve-extension",
        help = "When used with -o, preserves the original file extension"
    )]
    pub preserve_extension: bool,
    #[structopt(
        short = "e",
        long = "external-zip",
        help = "Use the zip CLI tool to unzip the installer. Faster."
    )]
    pub external_zip: bool,
}
