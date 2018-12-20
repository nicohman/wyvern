#[derive(StructOpt, Debug)]
#[structopt(name = "wyvern")]
pub enum Wyvern {
    #[structopt(name = "ls", about = "List all games you own")]
    List {

    },
    #[structopt(name = "down", about = "Download specific game")]
    Download {
        //TODO: Add support for multiple ways of specifying game to download
        id: i64
    }
}
