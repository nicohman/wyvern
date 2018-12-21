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
        //TODO: Add support for multiple ways of specifying game to download
        #[structopt(short = "i", long = "id", help ="download id")]
        id: Option<i64>,
        #[structopt(short = "s", long = "search", help ="search manually")]
        search: Option<String>
    }
}
