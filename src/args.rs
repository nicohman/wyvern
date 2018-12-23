#[derive(StructOpt, Debug)]
#[structopt(name = "wyvern")]
pub enum Wyvern {
    #[structopt(name = "ls", about = "List all games you own")]
    List {
         #[structopt(short = "i", long = "id", help = "search with id")]
         id: Option<i64>

    },
    #[structopt(name = "down", about = "Download specific game")]
    Download {
        #[structopt(short = "i", long = "id", help ="download id")]
        id: Option<i64>,
        #[structopt(short = "s", long = "search", help ="search manually")]
        search: Option<String>
    },
    #[structopt(name = "connect", about = "Operations associated with GOG Connect")]
    Connect(Connect)
}
#[derive(StructOpt, Debug)]
pub enum Connect {
    #[structopt(name = "ls", about = "List available GOG Connect games")]
    ListConnect {
        #[structopt(short = "c", long = "claimable", help = "only show games that are currently claimable")]
        claim: bool
    }
}
