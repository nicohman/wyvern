#[macro_use]
extern crate structopt;
extern crate confy;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
use structopt::StructOpt;
#[macro_use]
extern crate human_panic;
extern crate gog;
mod config;
mod args;
use crate::args::Wyvern;
use crate::args::Wyvern::*;
use crate::config::Config;
use std::io;
use std::io::Read;
use std::io::Write;
use gog::token::Token;
use gog::Gog;
use gog::error::Error;
use gog::gog::*;
use gog::gog::FilterParam::*;
fn main() -> Result<(), ::std::io::Error> {
    setup_panic!();
    let mut config : Config = confy::load("wyvern")?;
    if config.token.is_none() {
        let token = login();
        config.token = Some(token);
    }
    config.token = Some(config.token.unwrap().refresh().unwrap());
    print!("");
    let gog = Gog::new(config.token.clone().unwrap());
    confy::store("wyvern", config)?;
    let args = Wyvern::from_args();
    match args {
        List {} => list_owned(gog)
    };
    Ok(())
}
fn login() -> Token {
    println!("It appears that you have not logged into GOG. Please go to the following URL, log into GOG, and paste the code from the resulting url's ?code parameter into the input here.");
    println!("https://login.gog.com/auth?client_id=46899977096215655&layout=client2%22&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    let mut token : Token;
    loop {
        io::stdin().read_line(&mut code).unwrap();
        let attempt_token = Token::from_login_code(code.as_str());
        if attempt_token.is_ok() {
            token = attempt_token.unwrap();
            println!("Got token. Thanks!");
            break;
        } else {
            println!("Invalid code. Try again!");
        }
    }
    token
}
fn list_owned(gog: Gog) -> Result<(), Error>{
    let games = gog.get_filtered_products(FilterParams::from_one(MediaType(1)))?;
    for game in games {
        println!("{}", game.title);
    }
    Ok(())
}
