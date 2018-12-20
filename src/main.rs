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
extern crate indicatif;
mod args;
mod config;
use crate::args::Wyvern;
use crate::args::Wyvern::Download;
use crate::args::Wyvern::*;
use crate::config::Config;
use gog::error::Error;
use gog::gog::FilterParam::*;
use gog::gog::*;
use gog::token::Token;
use gog::Gog;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
fn main() -> Result<(), ::std::io::Error> {
    #[cfg(not(debug_assertions))]
    setup_panic!();
    let mut config: Config = confy::load("wyvern")?;
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
        List {} => {
            list_owned(gog).unwrap();
        }
        Download { id } => {
            let details = gog.get_game_details(id).unwrap();
            download(gog, details).unwrap();
        }
    };
    Ok(())
}
fn login() -> Token {
    println!("It appears that you have not logged into GOG. Please go to the following URL, log into GOG, and paste the code from the resulting url's ?code parameter into the input here.");
    println!("https://login.gog.com/auth?client_id=46899977096215655&layout=client2%22&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    let mut token: Token;
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
fn list_owned(gog: Gog) -> Result<(), Error> {
    let games = gog.get_filtered_products(FilterParams::from_one(MediaType(1)))?;
    println!("Title - Slug - GameID");
    for game in games {
        println!("{} - {} - {}", game.title, game.slug, game.id);
    }
    Ok(())
}
fn download(gog: Gog, game: GameDetails) -> Result<(), Error> {
    if game.downloads.linux.is_some() {
        let l_downloads = game.downloads.linux.unwrap();
        let mut names = vec![];
        for download in l_downloads.iter() {
            names.push(download.name.clone());
        }
        let mut responses = gog.download_game(l_downloads);
        let count = responses.len();
        for (idx, mut response) in responses.iter_mut().enumerate() {
            let total_size = response.headers().get("Content-Length").unwrap().to_str().unwrap().parse().unwrap();
            let pb = ProgressBar::new(total_size);
            pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"));
            let name = names[idx].clone();
            println!("Downloading {}, {} of {}", name, idx+1, count);
            let mut fd = fs::File::create(name.clone())?;
            let mut perms = fd.metadata()?.permissions();
            perms.set_mode(0o744);
            fd.set_permissions(perms);
            let mut pb_read = pb.wrap_read(response);
            io::copy(&mut pb_read, &mut fd)?;
            pb.finish();
        }
        println!("Done downloading!");
    } else {
        // TODO: Add capability for windows downloads
        println!("This game does not support linux!");
    }
    Ok(())
}
