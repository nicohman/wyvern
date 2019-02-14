/// This module provides wyvern's interactive mode, which lets the user use wyvern through a text-based GUI instead of by running single commands
use args::*;
use crate::parse_args;
use dialoguer::*;
use gog::gog::FilterParam::*;
use gog::gog::*;
use gog::*;
use std::fs;
use structopt::StructOpt;
pub fn interactive(gog: Gog, sync_saves: Option<String>) -> Gog {
    let options = ["List", "Download", "Extras", "Connect"];
    let mut gog = gog;
    loop {
        let pick = Select::new()
            .default(0)
            .items(&options[..])
            .interact()
            .unwrap();
        match options[pick] {
            // Both download and extras use an alphabetically-sorted list of all games owned
            "Download" | "Extras" => {
                let mut games = gog
                    .get_all_filtered_products(FilterParams::from_one(MediaType(1)))
                    .expect("Couldn't fetch games");
                games.sort_by(|a, b| a.title.partial_cmp(&b.title).unwrap());
                if options[pick] == "Download" {
                    let mut check = Checkboxes::new();
                    let mut picks = check.with_prompt("Select games to download").paged(true);
                    for g in games.iter() {
                        picks.item(g.title.as_str());
                    }
                    let picked = picks.interact().unwrap();
                    let install = Confirmation::new()
                        .with_text("Do you want to install these games after they are downloaded?")
                        .interact()
                        .unwrap();
                    for g in picked {
                        let id = games[g].id.to_string();
                        let mut args = vec!["wyvern", "download", "--id", id.as_str()];
                        if install {
                            args.push("--install");
                            args.push(games[g].title.as_str());
                            if fs::create_dir(&games[g].title).is_err() {
                                error!(
                                    "Could not make install directory named {}. Skipping.",
                                    games[g].title
                                );
                                continue;
                            }
                        }
                        let parsed = Wyvern::from_iter_safe(&args).unwrap();
                        gog = parse_args(parsed, gog, sync_saves.clone()).unwrap();
                    }
                } else
                // Extras
                {
                    let mut select = Select::new();
                    let mut pick = select.with_prompt("Select game to download extras from").paged(true);
                    for g in games.iter() {
                        pick.item(g.title.as_str());
                    }
                    let picked = pick.interact().unwrap();
                    let parsed = Wyvern::from_iter_safe(&vec![
                        "wyvern",
                        "extras",
                        "--first",
                        games[picked].title.as_str(),
                    ])
                    .unwrap();
                    gog = parse_args(parsed, gog, sync_saves.clone()).unwrap();
                }
            }
            "Connect" => {
                let actions = ["Claim", "List", "Quit"];
                loop {
                    let pick = Select::new().default(0).items(&actions).interact().unwrap();
                    match actions[pick] {
                        "Quit" => {
                            break;
                        }
                        _ => {
                            let parsed = Wyvern::from_iter_safe(&vec![
                                "wyvern",
                                "connect",
                                actions[pick].to_lowercase().as_str(),
                            ])
                            .unwrap();
                            gog = crate::connect::parse_args(gog, parsed);
                        }
                    }
                }
            }
            _ => {
                let parsed =
                    Wyvern::from_iter_safe(&["wyvern", options[pick].to_lowercase().as_str()])
                        .unwrap();
                gog = parse_args(parsed, gog, sync_saves.clone()).unwrap();
            }
        }
    }
}
