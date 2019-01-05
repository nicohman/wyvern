use fs::OpenOptions;
use gog::token::Token;
use serde;
use serde::de::Deserialize;
use serde::ser::SerializeMap;
use serde::ser::Serializer;
use serde::Deserializer;
use serde::Serialize;
use serde_derive;
use serde_json;
use std::collections::HashMap;
use std::default::Default;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use toml;
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
#[derive(Serialize, Deserialize)]
pub struct SaveInfo {
    pub identifier: SaveType,
    pub path: String,
}
impl Default for Config {
    fn default() -> Config {
        Config {
            version: 1,
            sync_saves: Some("hi".to_string()),
            token: None,
        }
    }
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
