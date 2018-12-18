use gog::token::Token;
use std::default::Default;
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub version: u8,
    pub token: Option<Token>,
}
impl Default for Config {
    fn default() -> Config {
        Config {
            version:0,
            token: None
        }
    }
}
