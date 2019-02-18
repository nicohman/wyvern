use curl::easy::{Handler, WriteError};
use fs::File;
use fs::OpenOptions;
use gog::token::Token;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json;
use std::collections::HashMap;
use std::default::Default;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Hash)]
pub enum SaveType {
    GOG(i64),
    Other(String),
}
type SaveMap = HashMap<String, SaveInfo>;
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    pub sync_saves: Option<String>,
    pub token: Option<Token>,
}
impl Default for Config {
    fn default() -> Config {
        Config {
            version: 1,
            sync_saves: None,
            token: None,
        }
    }
}
#[derive(Serialize, Deserialize)]
pub struct SaveInfo {
    pub identifier: SaveType,
    pub path: String,
}
#[derive(Serialize, Deserialize)]
pub struct SaveDB {
    pub saves: SaveMap,
}
impl Default for SaveDB {
    fn default() -> SaveDB {
        SaveDB {
            saves: HashMap::new(),
        }
    }
}
impl SaveDB {
    pub fn load<N>(path: N) -> Result<SaveDB, std::io::Error>
    where
        N: Into<PathBuf>,
    {
        let path = path.into();
        let mut unparsed = String::new();
        let file = fs::File::open(path.clone());
        if file.is_ok() {
            file.unwrap().read_to_string(&mut unparsed)?;
            Ok(serde_json::from_str(&unparsed).unwrap())
        } else {
            let default = SaveDB::default();
            default.store(path)?;
            Ok(default)
        }
    }
    pub fn store<N>(&self, path: N) -> Result<&SaveDB, std::io::Error>
    where
        N: Into<PathBuf>,
    {
        let to_write = serde_json::to_string(&self).unwrap();
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path.into())?
            .write_all(to_write.as_bytes())?;
        Ok(self)
    }
}
pub struct GameInfo {
    pub version: String,
    pub name: String,
}
impl GameInfo {
    pub fn parse(ginfo: impl Into<String>) -> Result<GameInfo, gog::Error> {
        let ginfo = ginfo.into();
        let mut lines = ginfo.trim().lines();
        info!("Getting name from gameinfo");
        if let Some(name) = lines.next() {
            let name = name.to_string();
            info!("Getting version string from gameinfo");
            if let Some(version) = lines.last() {
                let version = version.trim().to_string();
                Ok(GameInfo {
                    name: name,
                    version: version,
                })
            } else {
                error!("Could not get version from gameinfo");
                Err(gog::ErrorKind::MissingField("name".to_string()).into())
            }
        } else {
            error!("Could not fetch name from gameinfo file");
            Err(gog::ErrorKind::MissingField("name".to_string()).into())
        }
    }
}
pub struct WriteHandler {
    pub writer: File,
    pub pb: Option<ProgressBar>,
}
impl Handler for WriteHandler {
    fn write(&mut self, data: &[u8]) -> std::result::Result<usize, WriteError> {
        self.writer.write_all(data).expect("Couldn't write to file");
        if self.pb.is_some() {
            let pb = self.pb.take().unwrap();
            pb.inc(data.len() as u64);
            self.pb.replace(pb);
        }
        Ok(data.len())
    }
}
#[derive(Serialize, Debug)]
pub struct GamesList {
    pub games: Vec<Game>,
}
#[derive(Serialize, Debug)]
pub enum Game {
    ProductInfo(gog::gog::ProductDetails),
    GameInfo(gog::gog::GameDetails, i64),
}
impl Game {
    pub fn title(&self) -> String {
        match self {
            Game::ProductInfo(details) => details.title.clone(),
            Game::GameInfo(details, _id) => details.title.clone(),
        }
    }
}
