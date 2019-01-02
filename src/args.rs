use std::path::PathBuf;
#[derive(StructOpt, Debug)]
#[structopt(name = "wyvern")]
pub enum Wyvern {
    #[structopt(name = "ls", about = "List all games you own")]
    List {
        #[structopt(short = "i", long = "id", help = "search with id")]
        id: Option<i64>,
    },
    #[structopt(name = "down", about = "Download specific game")]
    Download {
        #[structopt(short = "i", long = "id", help = "download id")]
        id: Option<i64>,
        #[structopt(short = "s", long = "search", help = "search manually")]
        search: Option<String>,
        #[structopt(parse(from_os_str))]
        #[structopt(short = "n", long = "install", help = "install downloaded game to path")]
        install_after: Option<PathBuf>,
        #[structopt(short = "w", long = "windows-auto", help = "Download windows version if no linux is available")]
        windows_auto: bool,
        #[structopt(long = "force-windows", help = "Force downloading windows version")]
        windows_force:bool,
        #[structopt(short = "f", long = "first", help = "When searching, use first result without waiting for selection")]
        first: bool
    },
    #[structopt(name = "connect", about = "Operations associated with GOG Connect")]
    Connect(Connect),
    #[structopt(name = "install", about = "Install a GOG game from an installer")]
    Install {
        installer_name: String,
        #[structopt(parse(from_os_str))]
        path: PathBuf
    },
    #[structopt(name = "update", about = "Update a game if there is an update available")]
    Update {
        #[structopt(parse(from_os_str))]
        path: Option<PathBuf>,
        #[structopt(short = "f", long = "force", help = "Force updating even if unneeded")]
        force: bool
    }
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
    },
    #[structopt(name = "claim", about = "Claim all available GOG Connect games")]
    ClaimAll {},
}
