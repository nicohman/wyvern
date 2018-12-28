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
        install_after: Option<PathBuf>
    },
    #[structopt(name = "connect", about = "Operations associated with GOG Connect")]
    Connect(Connect),
    #[structopt(name = "install", about = "Install a GOG game from an installer")]
    Install {
        installer_name: String,
        #[structopt(parse(from_os_str))]
        path: PathBuf
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
