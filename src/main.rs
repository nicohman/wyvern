#[macro_use]
extern crate structopt;
extern crate confy;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
use structopt::StructOpt;
extern crate zip;
#[macro_use]
extern crate human_panic;
extern crate dirs;
extern crate gog;
extern crate indicatif;
#[cfg(feature = "eidolonint")]
extern crate libeidolon;
extern crate regex;
use regex::Regex;
mod args;
mod config;
use crate::args::Connect::*;
use crate::args::Sync::*;
use crate::args::Wyvern;
use crate::args::Wyvern::Download;
use crate::args::Wyvern::*;
use crate::config::*;
use gog::extract::*;
use gog::gog::{connect::ConnectGameStatus::*, connect::*, FilterParam::*, *};
use gog::token::Token;
use gog::Error;
use gog::ErrorKind::*;
use gog::Gog;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::fs::*;
use std::io;
use std::io::Read;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
fn main() -> Result<(), ::std::io::Error> {
    #[cfg(not(debug_assertions))]
    setup_panic!();
    let mut config: Config = confy::load("wyvern")?;
    let args = Wyvern::from_args();
    if config.token.is_none() {
        let token = login();
        config.token = Some(token);
    }
    config.token = Some(config.token.unwrap().refresh().unwrap());
    print!("");
    let gog = Gog::new(config.token.clone().unwrap());
    let sync_saves = config.sync_saves.clone();
    confy::store("wyvern", config)?;
    match args {
        List { id } => {
            if let Some(id) = id {
                let details = gog.get_game_details(id).unwrap();
                println!("Title - GameID");
                println!("{} - {}", details.title, id);
            } else {
                list_owned(gog).unwrap();
            }
        }
        Download {
            id,
            search,
            install_after,
            windows_auto,
            windows_force,
            first,
        } => {
            if let Some(search) = search {
                let search_results =
                    gog.get_filtered_products(FilterParams::from_one(Search(search)));
                if search_results.is_ok() {
                    let e = search_results.unwrap();
                    if !first {
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
                                    let pname = details.title.clone();
                                    let (name, downloaded_windows) =
                                        download_prep(gog, details, windows_auto, windows_force)
                                            .unwrap();
                                    if install_after.is_some() && !downloaded_windows {
                                        println!("Installing game");
                                        let mut installer = fs::File::open(name).unwrap();
                                        install(&mut installer, install_after.unwrap(), pname);
                                    }
                                    break;
                                } else {
                                    println!("Please enter a valid number corresponding to an available download");
                                }
                            } else {
                                println!(
                                    "Please enter a number corresponding to an available download"
                                );
                            }
                        }
                    } else {
                        let details = gog.get_game_details(e[0].id).unwrap();
                        let pname = details.title.clone();
                        let (name, downloaded_windows) =
                            download_prep(gog, details, windows_auto, windows_force).unwrap();
                        if install_after.is_some() && !downloaded_windows {
                            println!("Installing game");
                            let mut installer = fs::File::open(name).unwrap();
                            install(&mut installer, install_after.unwrap(), pname);
                        }
                    }
                } else {
                    println!("Could not find any games.");
                }
            } else if let Some(id) = id {
                let details = gog.get_game_details(id).unwrap();
                let pname = details.title.clone();
                let (name, downloaded_windows) =
                    download_prep(gog, details, windows_auto, windows_force).unwrap();

                if install_after.is_some() && !downloaded_windows {
                    println!("Installing game");
                    let mut installer = fs::File::open(name).unwrap();
                    install(&mut installer, install_after.unwrap(), pname);
                }
            } else {
                println!("Did not specify a game to download. Exiting.");
            }
        }
        Install {
            installer_name,
            path,
        } => {
            let mut installer = File::open(&installer_name);
            if installer.is_ok() {
                install(&mut installer.unwrap(), path, installer_name);
            } else {
                println!("File {} does not exist", installer_name)
            }
        }
        Sync { .. } => match args {
            Sync(Push { game_dir, sync_to }) => {
                if sync_saves.is_some() {
                    let mut sync_saves = sync_saves.unwrap();
                    if sync_to.is_some() {
                        sync_saves = sync_to.unwrap().to_str().unwrap().to_string();
                    }
                    sync_saves =
                        sync_saves.replace("~", dirs::home_dir().unwrap().to_str().unwrap());
                    let gameinfo = File::open(game_dir.join("gameinfo"));
                    if gameinfo.is_ok() {
                        let mut ginfo_string = String::new();
                        gameinfo.unwrap().read_to_string(&mut ginfo_string).unwrap();
                        let gameinfo = parse_gameinfo(ginfo_string);
                        if let Ok(details) =
                            gog.get_products(FilterParams::from_one(Search(gameinfo.name.clone())))
                        {
                            let id = details[0].id;
                            let savedb_path = PathBuf::from(sync_saves.clone()).join("savedb.json");
                            let mut save_db = SaveDB::load(&savedb_path).unwrap();
                            let mut path: PathBuf;
                            if save_db.saves.contains_key(&format!("{}", id)) {
                                path = PathBuf::from(
                                    save_db.saves.get(&format!("{}", id)).unwrap().path.clone(),
                                );
                            } else {
                                let mut input = String::new();
                                let mut yn = String::new();
                                println!("You haven't specified where this game's save files are yet. Please insert a path to where they are located.");
                                loop {
                                    io::stdout().flush().unwrap();
                                    io::stdin().read_line(&mut input).unwrap();
                                    print!(
                                        "Are you sure this where the save files are located?(Y/n)"
                                    );
                                    io::stdout().flush().unwrap();
                                    io::stdin().read_line(&mut yn).unwrap();
                                    if &yn == "n" || &yn == "N" {
                                        continue;
                                    }
                                    break;
                                }
                                save_db.saves.insert(
                                    format!("{}", id),
                                    SaveInfo {
                                        path: input.clone().trim().to_string(),
                                        identifier: SaveType::GOG(id),
                                    },
                                );
                                path = PathBuf::from(input.trim());
                                save_db.store(&savedb_path).unwrap();
                            }
                            let mut path_string = path.to_str().unwrap().to_string();
                            if path.exists() && path.is_dir() {
                                path_string += "/";
                            }
                            let save_dir = PathBuf::from(sync_saves);
                            let save_folder =
                                save_dir.clone().join("saves").join(format!("gog_{}", id));
                            println!("{:?}", save_folder);
                            println!("{:?}", path);
                            if fs::metadata(&save_folder).is_err() {
                                fs::create_dir_all(&save_folder).unwrap();
                            }
                            Command::new("rsync")
                                .arg(
                                    path_string
                                        .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
                                )
                                .arg(save_folder.to_str().unwrap())
                                .arg("-a")
                                .output()
                                .unwrap();
                            println!("Synced save files to save folder!");
                        } else {
                            println!("Could not find a game named {}.", gameinfo.name);
                        }
                    } else {
                        println!("Game directory or gameinfo file missing.")
                    }
                } else {
                    println!("You have not configured a directory to sync your saves to. Edit ~/.config/wyvern/wyvern.toml to get started!");
                }
            }
            Sync(Pull {
                game_dir,
                sync_from,
                force,
            }) => {
                if sync_saves.is_some() {
                    let sync_saves = sync_saves
                        .unwrap()
                        .replace("~", dirs::home_dir().unwrap().to_str().unwrap());
                    let gameinfo = File::open(game_dir.join("gameinfo"));
                    if gameinfo.is_ok() {
                        let mut ginfo_string = String::new();
                        gameinfo
                            .unwrap()
                            .read_to_string(&mut ginfo_string)
                            .expect("Couldn't read from gameinfo file.");
                        let gameinfo = parse_gameinfo(ginfo_string);
                        if let Ok(details) =
                            gog.get_products(FilterParams::from_one(Search(gameinfo.name.clone())))
                        {
                            let id = details[0].id;
                            let mut savedb_path =
                                PathBuf::from(sync_saves.clone()).join("savedb.json");
                            if sync_from.is_some() {
                                savedb_path = sync_from.unwrap();
                            }
                            let mut save_db = SaveDB::load(&savedb_path).unwrap();
                            if save_db.saves.contains_key(&format!("{}", id)) {
                                let save_path = PathBuf::from(sync_saves.clone())
                                    .join("saves")
                                    .join(format!("gog_{}", id));
                                let metadata = fs::metadata(&save_path);
                                if metadata.is_ok() {
                                    let save_files = save_db.saves.get(&format!("{}", id)).unwrap();
                                    let current_saves_try =
                                        fs::metadata(&save_files.path.replace(
                                            "~",
                                            dirs::home_dir().unwrap().to_str().unwrap(),
                                        ));
                                    if current_saves_try.is_ok() {
                                        let updated_saves = metadata.unwrap();
                                        let cur_updated =
                                            current_saves_try.unwrap().modified().unwrap();
                                        let up_updated = updated_saves.modified().unwrap();
                                        if cur_updated > up_updated && !force {
                                            print!("Current save files are more recent. Are you sure you want to proceed?(y/N)");
                                            let mut answer = String::new();
                                            io::stdout().flush().unwrap();
                                            io::stdin().read_line(&mut answer).unwrap();
                                            if answer.as_str() == "y" || answer.as_str() == "Y" {
                                                println!("Proceeding as normal.");
                                            } else {
                                                println!("Sync aborted.");
                                                std::process::exit(0);
                                            }
                                        }
                                    }
                                    let to_copy_path = save_path.to_str().unwrap().to_string();
                                    let mut dest_path =
                                        PathBuf::from(save_files.path.clone().replace(
                                            "~",
                                            dirs::home_dir().unwrap().to_str().unwrap(),
                                        ));
                                    dest_path = dest_path.parent().unwrap().to_path_buf();
                                    println!("{:?}", dest_path);
                                    println!("{}", to_copy_path);
                                    Command::new("rsync")
                                        .arg(to_copy_path + "/")
                                        .arg(dest_path.to_str().unwrap().to_string() + "/")
                                        .arg("-a")
                                        .arg("--force")
                                        .output()
                                        .unwrap();
                                    println!("Pulled save files");
                                } else {
                                    println!("Saves do not exist.");
                                }
                            } else {
                                println!("This game's saves have not been configured to be synced yet. Push first!");
                            }
                        } else {
                            println!("Could not find a game named {}.", gameinfo.name)
                        }
                    } else {
                        println!("Game directory or gameinfo file missing.");
                    }
                } else {
                    println!("You have not config a directory to sync your saves from. Edit ~/.config/wyvern/wyvern.toml to get started!");
                }
            }
            _ => println!("Wow, you should not be seeing this message."),
        },
        Update { mut path, force } => {
            if path.is_none() {
                path = Some(PathBuf::from(".".to_string()));
            }
            let path = path.unwrap();

            let game_info_path = path.clone().join("gameinfo");
            println!("{:?}", game_info_path);
            if let Ok(mut gameinfo) = File::open(game_info_path) {
                let regex = Regex::new(r"(.*) \(gog").unwrap();
                let mut ginfo_string = String::new();
                gameinfo.read_to_string(&mut ginfo_string).unwrap();
                let ginfo = parse_gameinfo(ginfo_string);
                let name = ginfo.name.clone();
                let version = ginfo.version.clone();
                let product =
                    gog.get_filtered_products(FilterParams::from_one(Search(name.clone())));
                if product.is_ok() {
                    let details = gog.get_game_details(product.unwrap()[0].id).unwrap();
                    let downloads = details.downloads.linux.unwrap();
                    let current_version = regex
                        .captures(&(downloads[0].version.clone().unwrap()))
                        .unwrap()[1]
                        .trim()
                        .to_string();
                    println!(
                        "Installed version : {}. Version Online: {}",
                        version, current_version
                    );
                    if version == current_version && !force {
                        println!("No newer version to update to. Sorry!");
                    } else {
                        if force && version == current_version {
                            println!("Forcing reinstall due to --force option.");
                        }
                        println!("Updating {} to version {}", name, current_version);
                        let name = download(gog, downloads).unwrap();
                        println!("Installing.");
                        let mut installer = File::open(name.clone()).unwrap();
                        install(&mut installer, path, name);
                        println!("Game finished updating!");
                    }
                } else {
                    println!("Can't find game {} in your library.", name);
                }
            } else {
                println!("Game installation missing a gameinfo file to check for update with.");
            }
        }
        Connect { .. } => {
            let uid: i64 = gog.get_user_data().unwrap().user_id.parse().unwrap();
            let linked = gog.connect_account(uid);
            if linked.is_err() {
                println!("You don't have a steam account linked to GOG! Go to https://www.gog.com/connect to link one.");
                return Ok(());
            } else {
                gog.connect_scan(uid).unwrap();
            }
            match args {
                Connect(ListConnect { claim, quiet }) => {
                    let status = gog.connect_status(uid);
                    if status.is_ok() {
                        let mut items = status.unwrap().items;
                        let left_over: Vec<(String, ConnectGame)> = items
                            .into_iter()
                            .filter_map(|x| {
                                if !claim || x.1.status == READY_TO_LINK {
                                    let details = gog.product(vec![x.1.id], vec![]);
                                    if details.is_ok() {
                                        println!(
                                            "{} - {:?}",
                                            details.unwrap()[0].title,
                                            x.1.status
                                        );
                                        return None;
                                    }
                                }
                                return Some(x);
                            })
                            .collect();
                        if !quiet {
                            println!("{} items not shown due to options", left_over.len());
                        }
                    } else {
                        let err = status.err().unwrap();
                        match err.kind() {
                            NotAvailable => println!("No GOG Connect games are available."),
                            _ => panic!("{:?}", err),
                        };
                    }
                }
                Connect(ClaimAll {}) => {
                    gog.connect_claim(uid).unwrap();
                    println!("Claimed all available games");
                }
                _ => println!("Tell someone about this, because it should not be happening"),
            }
        }
    };
    Ok(())
}
fn parse_gameinfo(ginfo: String) -> GameInfo {
    let mut lines = ginfo.trim().lines();
    let name = lines.next().unwrap().to_string();
    let version = lines.last().unwrap().trim().to_string();
    GameInfo {
        name: name,
        version: version,
    }
}
fn download_prep(
    gog: Gog,
    details: GameDetails,
    windows_auto: bool,
    windows_force: bool,
) -> Result<(String, bool), Error> {
    if details.downloads.linux.is_some() && !windows_force {
        let name = download(gog, details.downloads.linux.unwrap()).unwrap();
        return Ok((name, false));
    } else {
        if !windows_auto && !windows_force {
            let mut choice = String::new();
            loop {
                println!("This game does not support linux! Would you like to download the windows version to run under wine?(y/n)");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut choice).unwrap();
                match choice.to_lowercase().as_str() {
                    "y" => {
                        println!("Downloading windows files. Note: wyvern does not support automatic installation from windows games");
                        let name = download(gog, details.downloads.windows.unwrap()).unwrap();
                        return Ok((name, true));
                    }
                    "n" => {
                        println!("No suitable downloads found. Exiting");
                        std::process::exit(0);
                    }
                    _ => println!("Please enter y or n to proceed."),
                }
            }
        } else {
            if !windows_force {
                println!("No linux version available. Downloading windows version.");
            }
            let name = download(gog, details.downloads.windows.unwrap()).unwrap();
            return Ok((name, true));
        }
    }
}
fn install(installer: &mut File, path: PathBuf, name: String) {
    extract(
        installer,
        "/tmp",
        ToExtract {
            unpacker: false,
            mojosetup: false,
            data: true,
        },
    )
    .unwrap();
    let file = File::open("/tmp/data.zip").unwrap();
    // Extract code taken mostly from zip example
    let mut archive = zip::ZipArchive::new(file).unwrap();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let filtered_path = file
            .sanitized_name()
            .to_str()
            .unwrap()
            .replace("/noarch", "")
            .replace("data/", "")
            .to_owned();
        //Extract only files for the game itself
        if !filtered_path.contains("meta") && !filtered_path.contains("scripts") {
            let outpath = path.clone().join(PathBuf::from(filtered_path));
            if (&*file.name()).ends_with('/') {
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p).unwrap();
                    }
                }
                let mut outfile = fs::File::create(&outpath).unwrap();
                io::copy(&mut file, &mut outfile).unwrap();
            }
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }
    #[cfg(feature = "eidolonint")]
    {
        use libeidolon::games::*;
        use libeidolon::helper::*;
        use libeidolon::*;
        let proc_name = create_procname(name.clone());
        let game = Game {
            name: proc_name,
            pname: name,
            command: std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            typeg: GameType::WyvernGOG,
        };
        add_game(game);
    }
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
fn download(gog: Gog, downloads: Vec<gog::gog::Download>) -> Result<String, Error> {
    let mut names = vec![];
    for download in downloads.iter() {
        names.push(download.name.clone());
    }
    let mut responses = gog.download_game(downloads);
    let count = responses.len();
    for (idx, mut response) in responses.iter_mut().enumerate() {
        let total_size = response
            .headers()
            .get("Content-Length")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"));
        let name = names[idx].clone();
        println!("Downloading {}, {} of {}", name, idx + 1, count);
        let mut fd = fs::File::create(name.clone())?;
        let mut perms = fd.metadata()?.permissions();
        perms.set_mode(0o744);
        fd.set_permissions(perms)?;
        let mut pb_read = pb.wrap_read(response);
        io::copy(&mut pb_read, &mut fd)?;
        pb.finish();
    }
    println!("Done downloading!");
    return Ok(names[0].clone());
}
