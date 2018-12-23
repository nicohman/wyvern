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
use crate::args::Connect::*;
use crate::config::Config;
use gog::error::Error;
use gog::gog::FilterParam::*;
use gog::gog::*;
use gog::token::Token;
use gog::Gog;
use gog::gog::connect::ConnectGameStatus::*;
use gog::gog::connect::*;
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
        List {id} => {
            if let Some(id)  = id {
                let details = gog.get_game_details(id).unwrap();
                println!("Title - GameID");
                println!("{} - {}", details.title, id);
            } else {
                list_owned(gog).unwrap();
            }
        }
        Download { id, search } => {
            if let Some(search) = search {
                let search_results = gog.get_filtered_products(FilterParams::from_one(Search(search)));
                if search_results.is_ok() {
                    let e = search_results.unwrap();
                    for (idx, pd) in e.iter().enumerate() {
                        println!("{}. {} - {}", idx, pd.title, pd.id);
                    }
                    let mut choice = String::new();
                    loop {
                        print!("Select a game to download:");
                        io::stdout().flush().unwrap();
                        io::stdin().read_line(&mut choice).unwrap();
                        let parsed = choice.trim().parse::<usize>();
                        if let Ok(i) = parsed {
                            if e.len() > i {
                                let details = gog.get_game_details(e[i].id).unwrap();
                                download(gog, details).unwrap();
                                break;
                            } else {
                                println!("Please enter a valid number corresponding to an available download");
                            }
                        } else {
                            println!("Please enter a number corresponding to an available download");
                        }
                    }
                } else {
                    println!("Could not find any games.");
                }
            } else if let Some(id) = id {
                let details = gog.get_game_details(id).unwrap();
                download(gog, details).unwrap();
            } else {
                println!("Did not specify a game to download");
            }

        },
        Connect { .. } => {
            let uid : i64 = gog.get_user_data().unwrap().user_id.parse().unwrap();
            let linked = gog.connect_account(uid);
            if linked.is_err() {
                println!("You don't have a steam account linked to GOG! Go to https://www.gog.com/connect to link one.");
                return Ok(());
            } else {
                println!("Using steam account {} for linking.", linked.unwrap().user.steam_username);
                gog.connect_scan(uid).unwrap();
            }
            match args {
                Connect(ListConnect { claim }) => {
                    let mut items = gog.connect_status(uid).unwrap().items;
                    let left_over : Vec<(String, ConnectGame)>= items.into_iter().filter_map(|x| {
                        if !claim || x.1.status == READY_TO_LINK {
                            let details = gog.product(vec![x.1.id],vec![]);
                            if details.is_ok() {
                                println!("{} - {:?}", details.unwrap()[0].title, x.1.status);
                                return None;
                            }
                        }
                        return Some(x);
                    }).collect();
                    println!("{} items not shown due to options", left_over.len());
                },
                _ => println!("Tell someone about this, because it should not be happening")
            }
        }
    };
    Ok(())
}
pub fn login() -> Token {
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
    println!("Title - GameID");
    for game in games {
        println!("{} - {}", game.title, game.id);
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
            fd.set_permissions(perms)?;
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
