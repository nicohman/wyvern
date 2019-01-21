#[cfg(feature = "eidolonint")]
extern crate libeidolon;
#[macro_use]
extern crate structopt;
#[macro_use]
extern crate human_panic;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate clap_verbosity_flag;
extern crate confy;
extern crate crc;
extern crate dirs;
extern crate gog;
extern crate indicatif;
extern crate inflate;
extern crate rayon;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate zip;
mod args;
mod config;
use crate::args::Connect::*;
use crate::args::Sync::*;
use crate::args::Wyvern;
use crate::args::Wyvern::Download;
use crate::args::Wyvern::*;
use crate::config::*;
use crc::crc32;
use gog::extract::*;
use gog::gog::{connect::ConnectGameStatus::*, connect::*, FilterParam::*, *};
use gog::token::Token;
use gog::Error;
use gog::ErrorKind::*;
use gog::Gog;
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use regex::Regex;
use std::env::current_dir;
use std::fs;
use std::fs::*;
use std::io;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom::*;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use structopt::StructOpt;
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
    let mut sync_saves = config.sync_saves.clone();
    if sync_saves.is_some() {
        sync_saves = Some(
            sync_saves
                .unwrap()
                .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
        );
    }
    confy::store("wyvern", config)?;
    match args {
        List { id, verbose } => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
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
            all,
            mut desktop,
            mut menu,
            shortcuts,
            verbose,
        } => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            if shortcuts {
                desktop = true;
                menu = true;
            }
            if let Some(search) = search {
                info!("Searching for games");
                let search_results =
                    gog.get_filtered_products(FilterParams::from_one(Search(search)));
                if search_results.is_ok() {
                    info!("Game search results OK");
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
                                    info!("Fetching game details");
                                    let details = gog.get_game_details(e[i].id).unwrap();
                                    let pname = details.title.clone();
                                    info!("Beginning download process");
                                    let (name, downloaded_windows) =
                                        download_prep(&gog, details, windows_auto, windows_force)
                                            .unwrap();
                                    if install_after.is_some() && !downloaded_windows {
                                        println!("Installing game");
                                        info!("Opening installer file");
                                        let mut installer = fs::File::open(name).unwrap();
                                        info!("Installing game");
                                        install(
                                            &mut installer,
                                            install_after.unwrap(),
                                            pname,
                                            desktop,
                                            menu,
                                        );
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
                        info!("Downloading first game from results");
                        let details = gog.get_game_details(e[0].id).unwrap();
                        let pname = details.title.clone();
                        info!("Beginning download process");
                        let (name, downloaded_windows) =
                            download_prep(&gog, details, windows_auto, windows_force).unwrap();
                        if install_after.is_some() && !downloaded_windows {
                            println!("Installing game");
                            info!("Opening installer file");
                            let mut installer = fs::File::open(name).unwrap();
                            info!("Installing game");
                            install(&mut installer, install_after.unwrap(), pname, desktop, menu);
                        }
                    }
                } else {
                    println!("Could not find any games.");
                }
            } else if let Some(id) = id {
                let details = gog.get_game_details(id).unwrap();
                let pname = details.title.clone();
                info!("Beginning download process");
                let (name, downloaded_windows) =
                    download_prep(&gog, details, windows_auto, windows_force).unwrap();

                if install_after.is_some() && !downloaded_windows {
                    println!("Installing game");
                    info!("Opening installer file");
                    let mut installer = fs::File::open(name).unwrap();
                    info!("Installing game");
                    install(&mut installer, install_after.unwrap(), pname, desktop, menu);
                }
            } else if all {
                println!("Downloading all games in library");
                let games = gog.get_games().unwrap();
                for game in games {
                    let details = gog.get_game_details(game).unwrap();
                    info!("Beginning download process");
                    download_prep(&gog, details, windows_auto, windows_force).unwrap();
                }
                if install_after.is_some() {
                    println!("--install does not work with --all");
                }
            } else {
                println!("Did not specify a game to download. Exiting.");
            }
        }
        Install {
            installer_name,
            path,
            mut desktop,
            mut menu,
            shortcuts,
            verbose,
        } => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            if shortcuts {
                desktop = true;
                menu = true;
            }
            info!("Opening installer");
            let mut installer = File::open(&installer_name);
            if installer.is_ok() {
                info!("Starting installation");
                install(&mut installer.unwrap(), path, installer_name, desktop, menu);
            } else {
                error!(
                    "Could not open installer. Error: {}",
                    installer.err().unwrap()
                );
            }
        }
        #[cfg(feature = "eidolonint")]
        UpdateEidolon { force, verbose } => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            use libeidolon::games::*;
            let eidolon_games = get_games();
            for game in eidolon_games {
                if let Ok(read) = read_game(game.as_str()) {
                    if read.typeg == GameType::WyvernGOG {
                        println!("Attempting to update {}", read.pname);
                        let path = PathBuf::from(read.command);
                        let ginfo_path = path.clone().join("gameinfo");
                        update(&gog, path, ginfo_path, force, false);
                    }
                } else {
                    println!("Could not check {}", game);
                }
            }
        }
        Sync { .. } => match args {
            Sync(Push {
                game_dir,
                sync_to,
                verbose,
            }) => {
                verbose
                    .setup_env_logger("wyvern")
                    .expect("Couldn't set up logger");
                if sync_saves.is_some() {
                    let mut sync_saves = sync_saves.unwrap();
                    if sync_to.is_some() {
                        info!("Using manual argument sync path");
                        sync_saves = sync_to.unwrap().to_str().unwrap().to_string();
                    }
                    info!("Opening gameinfo file");
                    let gameinfo = File::open(game_dir.join("gameinfo"));
                    if gameinfo.is_ok() {
                        let mut ginfo_string = String::new();
                        info!("Reading from gameinfo file");
                        gameinfo.unwrap().read_to_string(&mut ginfo_string).unwrap();
                        info!("Parsing gameinfo");
                        let gameinfo = parse_gameinfo(ginfo_string);
                        info!("Fetching details about game from GOG");
                        if let Ok(details) =
                            gog.get_products(FilterParams::from_one(Search(gameinfo.name.clone())))
                        {
                            let id = details[0].id;
                            let savedb_path = PathBuf::from(sync_saves.clone()).join("savedb.json");
                            let mut save_db = SaveDB::load(&savedb_path).unwrap();
                            let mut path: PathBuf;
                            if save_db.saves.contains_key(&format!("{}", id)) {
                                info!("Savedb has path to saves confgured already");
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
                                let save_path =
                                    current_dir().unwrap().join(PathBuf::from(&input.trim()));
                                info!("Inserting saveinfo into savedb");
                                save_db.saves.insert(
                                    format!("{}", id),
                                    SaveInfo {
                                        path: save_path.to_str().unwrap().to_string(),
                                        identifier: SaveType::GOG(id),
                                    },
                                );
                                path = save_path;
                                info!("Storing savedb");
                                save_db.store(&savedb_path).unwrap();
                            }
                            let mut path_string = path.to_str().unwrap().to_string();
                            if path.exists() && path.is_dir() {
                                path_string += "/";
                            }
                            let save_dir = PathBuf::from(sync_saves);
                            let save_folder =
                                save_dir.clone().join("saves").join(format!("gog_{}", id));
                            if fs::metadata(&save_folder).is_err() {
                                info!("Creating directories for files");
                                fs::create_dir_all(&save_folder).unwrap();
                            }
                            info!("Start Rsyncing files");
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
                            error!("Could not find a game named {}.", gameinfo.name);
                        }
                    } else {
                        error!("Game directory or gameinfo file missing")
                    }
                } else {
                    error!("You have not configured a directory to sync your saves to. Edit ~/.config/wyvern/wyvern.toml to get started!");
                }
            }
            Sync(Pull {
                game_dir,
                sync_from,
                force,
                ignore_older,
                verbose,
            }) => {
                verbose
                    .setup_env_logger("wyvern")
                    .expect("Couldn't set up logger");
                if sync_saves.is_some() {
                    let sync_saves = sync_saves
                        .unwrap()
                        .replace("~", dirs::home_dir().unwrap().to_str().unwrap());
                    info!("Opening gameinfo file");
                    let gameinfo = File::open(game_dir.join("gameinfo"));
                    if gameinfo.is_ok() {
                        let mut ginfo_string = String::new();
                        info!("Reading in gameinfo file");
                        gameinfo
                            .unwrap()
                            .read_to_string(&mut ginfo_string)
                            .expect("Couldn't read from gameinfo file.");
                        info!("Parsing gameinfo file");
                        let gameinfo = parse_gameinfo(ginfo_string);
                        if let Ok(details) =
                            gog.get_products(FilterParams::from_one(Search(gameinfo.name.clone())))
                        {
                            let id = details[0].id;
                            let mut savedb_path =
                                PathBuf::from(sync_saves.clone()).join("savedb.json");
                            if sync_from.is_some() {
                                savedb_path = sync_from.unwrap().join("savedb.json");
                            }
                            info!("Loading savedb");
                            let mut save_db = SaveDB::load(&savedb_path).unwrap();
                            if save_db.saves.contains_key(&format!("{}", id)) {
                                let save_path = PathBuf::from(sync_saves.clone())
                                    .join("saves")
                                    .join(format!("gog_{}", id));
                                let save_files = save_db.saves.get(&format!("{}", id)).unwrap();
                                let saved_path = PathBuf::from(
                                    save_files
                                        .path
                                        .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
                                );
                                info!("Syncing files now");
                                sync(save_path, saved_path, force, ignore_older);
                            } else {
                                error!("This game's saves have not been configured to be synced yet. Push first!");
                            }
                        } else {
                            error!("Could not find a game named {}", gameinfo.name)
                        }
                    } else {
                        error!("Game directory or gameinfo file missing");
                    }
                } else {
                    error!("You have not config a directory to sync your saves from. Edit ~/.config/wyvern/wyvern.toml to get started!");
                }
            }
            Sync(DbPull {
                path,
                force,
                ignore_older,
                verbose,
            }) => {
                verbose
                    .setup_env_logger("wyvern")
                    .expect("Couldn't set up logger");
                let dbpath: PathBuf;
                if path.is_some() {
                    info!("Using db passed in arguments");
                    dbpath = path.unwrap();
                } else if sync_saves.is_some() {
                    info!("Using configured db path");
                    dbpath = PathBuf::from(sync_saves.unwrap());
                } else {
                    error!("You have not specified a sync directory in the config yet. Specify one or call db with a path to your db.");
                    std::process::exit(0);
                }
                info!("Loading savedb");
                let savedb = SaveDB::load(&dbpath.join("savedb.json")).unwrap();
                for (key, value) in savedb.saves.iter() {
                    println!("Syncing {} now", key);
                    let save_path = PathBuf::from(
                        value
                            .path
                            .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
                    );
                    let mut folder_name = key.clone();
                    if let SaveType::GOG(id) = value.identifier {
                        folder_name = format!("gog_{}", id);
                    }
                    let synced_path = dbpath.join("saves").join(folder_name);
                    info!("Syncing files now.");
                    sync(synced_path, save_path, force, ignore_older);
                    println!("Synced {}", key);
                }
            }
            Sync(DbPush {
                path,
                force,
                ignore_older,
                verbose,
            }) => {
                verbose
                    .setup_env_logger("wyvern")
                    .expect("Couldn't set up logger");
                let dpath: PathBuf;
                if path.is_some() {
                    info!("Using db passed in arguments");
                    dpath = path.unwrap();
                } else if sync_saves.is_some() {
                    info!("Using configured db path");
                    dpath = PathBuf::from(sync_saves.unwrap());
                } else {
                    error!("You have not specified a sync directory in the config yet. Specify one or call db with a path to your db.");
                    std::process::exit(0);
                }
                info!("Loading savedb");
                let savedb = SaveDB::load(dpath.clone().join("savedb.json")).unwrap();
                for (key, value) in savedb.saves {
                    println!("Syncing {} now", key);
                    let save_path = PathBuf::from(
                        value
                            .path
                            .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
                    );
                    let mut folder_name = key.clone();
                    if let SaveType::GOG(id) = value.identifier {
                        folder_name = format!("gog_{}", id);
                    }
                    let mut dest_path = dpath.join("saves").join(&folder_name);
                    info!("Syncing file snow");
                    sync(save_path, dest_path, force, ignore_older);
                    println!("Synced {}", key);
                }
            }
            Sync(Saves {
                game_dir,
                saves,
                db,
                verbose,
            }) => {
                verbose
                    .setup_env_logger("wyvern")
                    .expect("Couldn't set up logger");
                let dpath: PathBuf;
                if db.is_some() {
                    info!("Using db passed in arguments");
                    dpath = db.unwrap();
                } else if sync_saves.is_some() {
                    info!("Using configured db path");
                    dpath = PathBuf::from(sync_saves.unwrap());
                } else {
                    error!("You have not specified a sync directory in the config yet. Specify one or call saves with a path to your db.");
                    std::process::exit(0);
                }
                let dbpath = current_dir()
                    .unwrap()
                    .join(dpath.clone())
                    .join("savedb.json");
                info!("Loading savedb");
                let mut savedb = SaveDB::load(dbpath.clone()).unwrap();
                let gameinfo_path = current_dir().unwrap().join(game_dir).join("gameinfo");
                if gameinfo_path.is_file() {
                    let mut ginfo_string = String::new();
                    info!("Opening and reading gameinfo file");
                    File::open(&gameinfo_path)
                        .unwrap()
                        .read_to_string(&mut ginfo_string)
                        .unwrap();
                    let gameinfo = parse_gameinfo(ginfo_string);
                    if let Ok(details) =
                        gog.get_products(FilterParams::from_one(Search(gameinfo.name.clone())))
                    {
                        let id = details[0].id;
                        info!("Inserting record into savedb");
                        savedb.saves.insert(
                            format!("{}", id),
                            SaveInfo {
                                path: current_dir()
                                    .unwrap()
                                    .join(saves)
                                    .to_str()
                                    .unwrap()
                                    .to_string(),
                                identifier: SaveType::GOG(id),
                            },
                        );
                        info!("Storing savedb");
                        savedb.store(dbpath).unwrap();
                    } else {
                        error!("Could not find a game named {}", gameinfo.name);
                    }
                } else {
                    error!("No gameinfo file at {}", gameinfo_path.to_str().unwrap());
                }
            }
            _ => println!("Wow, you should not be seeing this message."),
        },

        Update {
            mut path,
            force,
            verbose,
            delta,
        } => {
            verbose
                .setup_env_logger("wyvern")
                .expect("Couldn't set up logger");
            if path.is_none() {
                info!("Path not specified. Using current dir");
                path = Some(PathBuf::from(".".to_string()));
            }
            let path = path.unwrap();
            let game_info_path = path.clone().join("gameinfo");
            info!("Updating game");
            update(&gog, path, game_info_path, force, delta);
        }
        Connect { .. } => {
            let uid: i64 = gog.get_user_data().unwrap().user_id.parse().unwrap();
            info!("Getting GOG Connect steam account");
            let linked = gog.connect_account(uid);
            if linked.is_err() {
                error!("You don't have a steam account linked to GOG! Go to https://www.gog.com/connect to link one.");
                return Ok(());
            } else {
                info!("Scanning for Connect games");
                gog.connect_scan(uid).unwrap();
            }
            match args {
                Connect(ListConnect {
                    claim,
                    quiet,
                    verbose,
                }) => {
                    verbose
                        .setup_env_logger("wyvern")
                        .expect("Couldn't set up logger");
                    info!("Getting GOG Connect status");
                    let status = gog.connect_status(uid);
                    if status.is_ok() {
                        let mut items = status.unwrap().items;
                        let left_over: Vec<(String, ConnectGame)> = items
                            .into_iter()
                            .filter_map(|x| {
                                if !claim || x.1.status == READY_TO_LINK {
                                    info!("Getting details for connect game");
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
                            NotAvailable => error!("No GOG Connect games are available."),
                            _ => error!("{}", err),
                        };
                    }
                }
                Connect(ClaimAll { verbose }) => {
                    verbose
                        .setup_env_logger("wyvern")
                        .expect("Couldn't set up logger");
                    gog.connect_claim(uid).unwrap();
                    println!("Claimed all available games");
                }
                _ => error!("Tell someone about this, because it should not be happening"),
            }
        }
    };
    Ok(())
}
fn update(gog: &Gog, path: PathBuf, game_info_path: PathBuf, force: bool, delta: bool) {
    if let Ok(mut gameinfo) = File::open(&game_info_path) {
        let regex = Regex::new(r"(.*) \(gog").unwrap();
        let mut ginfo_string = String::new();
        info!("Reading in gameinfo file");
        gameinfo.read_to_string(&mut ginfo_string).unwrap();
        info!("Parsing gameinfo");
        let ginfo = parse_gameinfo(ginfo_string);
        let name = ginfo.name.clone();
        let version = ginfo.version.clone();
        info!("Searching GOG products for {}", name);
        let product = gog.get_filtered_products(FilterParams::from_one(Search(name.clone())));
        if product.is_ok() {
            info!("Fetching the GameDetails for first result of search");
            let details = gog.get_game_details(product.unwrap()[0].id).unwrap();
            info!("Getting game's linux downloads");
            let downloads = details
                .downloads
                .linux
                .expect("Game has no linux downloads");
            if delta {
                info!("Using delta-based updating");
                let data = gog.extract_data(downloads).unwrap();
                println!("{:?}", data);
                for file in data.files {
                    if !file.filename.contains("meta") && !file.filename.contains("scripts") {
                        let path = game_info_path
                            .parent()
                            .unwrap()
                            .join(&file.filename.replace("/noarch", "").replace("data/", ""));
                        let is_dir = path.extension().is_none()
                            || file.filename.clone().pop().unwrap() == '/';
                        if path.is_file() {
                            info!("Checking file {:?}", path);
                            let mut buffer = vec![];
                            info!("Opening file");
                            let mut fd =
                                File::open(&path).expect(&format!("Couldn't open file {:?}", path));
                            fd.read_to_end(&mut buffer).unwrap();
                            let checksum = crc32::checksum_ieee(buffer.as_slice());
                            if checksum == file.crc32 {
                                info!("File {:?} is the same", path);
                                continue;
                            }
                            println!("File {:?} is different. Downloading.", path);
                        } else if !path.exists() && is_dir {
                            continue;
                        } else if is_dir {
                            fs::create_dir_all(path);
                            continue;
                        } else {
                            println!("File {:?} does not exist. Downloading.", path);
                        }
                        fs::create_dir_all(path.parent().unwrap());
                        info!("Fetching file from installer");
                        let mut bytes = gog
                            .download_request_range(
                                &data.url,
                                file.start_offset as i64,
                                file.end_offset as i64,
                            )
                            .unwrap();
                        let bytes_len = bytes.len();
                        let mut bytes_cur = Cursor::new(bytes);
                        let mut header_buffer = [0; 4];
                        bytes_cur.read_exact(&mut header_buffer).unwrap();
                        if u32::from_le_bytes(header_buffer) != 0x04034b50 {
                            error!("Bad local file header");
                        }
                        bytes_cur.seek(Start(28)).unwrap();
                        let mut buffer = [0; 2];
                        info!("Reading length of extra field");
                        bytes_cur.read_exact(&mut buffer).unwrap();
                        let extra_length = u16::from_le_bytes(buffer);
                        info!("Seeking to beginning of file");
                        bytes_cur
                            .seek(Current((file.filename_length + extra_length) as i64))
                            .unwrap();
                        let mut bytes = vec![0; bytes_len - bytes_cur.position() as usize];
                        bytes_cur.read_exact(&mut bytes).unwrap();
                        let mut fd = OpenOptions::new()
                            .write(true)
                            .create(true)
                            .open(&path)
                            .expect(&format!("Couldn't open file {:?}", path));
                        if file.external_file_attr != Some(0) {
                            info!("Setting permissions");
                            fd.set_permissions(Permissions::from_mode(
                                file.external_file_attr.unwrap() >> 16,
                            ))
                            .expect("Couldn't set permissions");
                        }
                        info!("Decompressing file");
                        let def = inflate::inflate_bytes(bytes.as_slice()).unwrap();
                        info!("Writing decompressed file to disk");
                        fd.write_all(&def)
                            .expect(&format!("Couldn't write to file {:?}", path));
                    }
                }
            } else {
                info!("Using regex to fetch version string");

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
                    let name = download(&gog, downloads).unwrap();
                    println!("Installing.");
                    info!("Opening installer file");
                    let mut installer = File::open(name.clone()).unwrap();
                    info!("Starting installation");
                    install(&mut installer, path, name, false, false);
                    println!("Game finished updating!");
                }
            }
        } else {
            error!("Could not find game on GOG");
            println!("Can't find game {} in your library.", name);
        }
    } else {
        error!(
            "Could not open gameinfo file that should be at {}.",
            game_info_path.to_str().unwrap()
        );
        println!("Game installation missing a gameinfo file to check for update with.");
    }
}
fn parse_gameinfo(ginfo: String) -> GameInfo {
    let mut lines = ginfo.trim().lines();
    info!("Getting name from gameinfo");
    let name = lines.next().unwrap().to_string();
    info!("Getting version string from gameinfo");
    let version = lines.last().unwrap().trim().to_string();
    GameInfo {
        name: name,
        version: version,
    }
}
fn sync(sync_from: PathBuf, sync_to: PathBuf, ignore_older: bool, force: bool) {
    let from_meta = fs::metadata(&sync_from);
    let to_meta = fs::metadata(&sync_to);
    if from_meta.is_err() {
        error!("Can't sync nonexistent files! There should be save files at {}, but there are not. Aborting.", sync_from.to_str().unwrap());
        return;
    }
    if let Ok(to_meta) = to_meta {
        info!("File already synced to locations. Getting modified times");
        let to_modified = to_meta.modified().unwrap();
        let from_modified = from_meta.unwrap().modified().unwrap();
        if to_modified > from_modified && !force {
            if ignore_older {
                println!("Aborting due to --ignore-older flag and newer save files being present");
                return;
            }
            print!("Synced save files are more recent. Are you sure you want to proceed?(y/N)");
            let mut answer = String::new();
            io::stdout().flush().unwrap();
            io::stdin().read_line(&mut answer).unwrap();
            if answer.trim() == "y" || answer.trim() == "Y" {
                println!("Proceeding as normal.");
            } else {
                println!("Sync aborted.");
                return;
            }
        }
    } else {
        info!("No files synced to location currently");
    }
    info!("Rsyncing files to location");
    Command::new("rsync")
        .arg(sync_from.to_str().unwrap().to_string() + "/")
        .arg(
            sync_to
                .parent()
                .unwrap()
                .to_path_buf()
                .to_str()
                .unwrap()
                .to_string()
                + "/",
        )
        .arg("-a")
        .arg("--force")
        .output()
        .unwrap();
}
fn download_prep(
    gog: &Gog,
    details: GameDetails,
    windows_auto: bool,
    windows_force: bool,
) -> Result<(String, bool), Error> {
    if details.downloads.linux.is_some() && !windows_force {
        info!("Downloading linux downloads");
        let name = download(gog, details.downloads.linux.unwrap()).unwrap();
        return Ok((name, false));
    } else {
        if !windows_auto && !windows_force {
            info!("Asking user about downloading windows version");
            let mut choice = String::new();
            loop {
                print!("This game does not support linux! Would you like to download the windows version to run under wine?(y/n)");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut choice).unwrap();
                match choice.to_lowercase().trim() {
                    "y" => {
                        println!("Downloading windows files. Note: wyvern does not support automatic installation from windows games");
                        info!("Downloading windows downloads");
                        let name = download(&gog, details.downloads.windows.unwrap()).unwrap();
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
            info!("Downloading windows downloads");
            let name = download(&gog, details.downloads.windows.unwrap()).unwrap();
            return Ok((name, true));
        }
    }
}
fn install(installer: &mut File, path: PathBuf, name: String, desktop: bool, menu: bool) {
    info!("Starting installer extraction process");
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
    info!("Opening extracted zip");
    let file = File::open("/tmp/data.zip").unwrap();
    // Extract code taken mostly from zip example
    let archive = zip::ZipArchive::new(BufReader::new(file)).unwrap();
    let len = archive.len();
    let pb = ProgressBar::new(len as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}")
            .progress_chars("#>-"),
    );
    info!("Starting zip extraction process");
    (0..len).into_par_iter().for_each(|i| {
        info!("Starting extraction of file #{}. Opening archive", i);
        let mut archive =
            zip::ZipArchive::new(BufReader::new(File::open("/tmp/data.zip").unwrap())).unwrap();
        info!("Getting file from archive");
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
                info!("Creating dir");
                fs::create_dir_all(&outpath).unwrap();
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p).unwrap();
                    }
                }
                info!("Creating file");
                let mut outfile = fs::File::create(&outpath).unwrap();
                info!("Copying to file");
                io::copy(&mut file, &mut outfile).unwrap();
            }
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                info!("Setting permissions for file");
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).unwrap();
            }
        } else {
            info!("File {} not being extracted", filtered_path);
        }
        pb.inc(1);
    });
    pb.finish_with_message("Game installed!");
    #[cfg(feature = "eidolonint")]
    {
        info!("Compiled with eidolon integration. Adding game to registry");
        use libeidolon::games::*;
        use libeidolon::helper::*;
        use libeidolon::*;
        let proc_name = create_procname(name.clone());
        info!("Creating game object");
        let game = Game {
            name: proc_name,
            pname: name,
            command: current_dir().unwrap().to_str().unwrap().to_string(),
            typeg: GameType::WyvernGOG,
        };
        info!("Adding game to eidolon");
        add_game(game);
        println!("Added game to eidolon registry!");
    }
    if menu || desktop {
        info!("Creating shortcuts");
        let game_path = current_dir().unwrap().join(&path);
        info!("Creating text of shortcut");
        let shortcut = desktop_shortcut(name.as_str(), &game_path);
        if menu {
            info!("Adding menu shortcut");
            let desktop_path = dirs::home_dir().unwrap().join(format!(
                ".local/share/applications/gog_com-{}_1.desktop",
                name
            ));
            info!("Created menu file");
            let fd = File::create(&desktop_path);
            if fd.is_ok() {
                info!("Writing to file");
                fd.unwrap()
                    .write(shortcut.as_str().as_bytes())
                    .expect("Couldn't write to menu shortcut");
            } else {
                error!(
                    "Could not create menu shortcut. Error: {}",
                    fd.err().unwrap()
                );
            }
        }
        if desktop {
            info!("Adding desktop shortcut");
            let desktop_path = dirs::home_dir()
                .unwrap()
                .join(format!("Desktop/gog_com-{}_1.desktop", name));
            let fd = File::create(&desktop_path);
            if fd.is_ok() {
                info!("Writing to file.");
                let mut fd = fd.unwrap();
                fd.write(shortcut.as_str().as_bytes())
                    .expect("Couldn't write to desktop shortcut");
                info!("Setting permissions");
                fd.set_permissions(Permissions::from_mode(0o0774))
                    .expect("Couldn't make desktop shortcut executable");
            } else {
                error!(
                    "Could not create desktop shortcut. Error: {}",
                    fd.err().unwrap()
                );
            }
        }
    }
}
pub fn login() -> Token {
    println!("It appears that you have not logged into GOG. Please go to the following URL, log into GOG, and paste the code from the resulting url's ?code parameter into the input here.");
    println!("https://login.gog.com/auth?client_id=46899977096215655&layout=client2%22&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    let mut token: Token;
    loop {
        info!("Atttempting to read input line for token");
        io::stdin().read_line(&mut code).unwrap();
        info!("Creating token from input");
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
fn download(gog: &Gog, downloads: Vec<gog::gog::Download>) -> Result<String, Error> {
    info!("Downloading files");
    let mut names = vec![];
    for download in downloads.iter() {
        names.push(download.name.clone());
    }
    info!("Getting responses to requests");
    let responses = gog.download_game(downloads);
    let count = responses.len();
    for (idx, mut response) in responses.into_iter().enumerate() {
        if response.is_err() {
            println!(
                "Error downloading file. Error message:{}",
                response.err().unwrap()
            );
        } else {
            let response = response.unwrap();
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
            info!("Creating file");
            let mut fd = fs::File::create(name.clone())?;
            let mut perms = fd.metadata()?.permissions();
            info!("Setting permissions to executable");
            perms.set_mode(0o744);
            fd.set_permissions(perms)?;
            let mut pb_read = pb.wrap_read(response);
            io::copy(&mut pb_read, &mut fd)?;
            pb.finish();
        }
    }
    println!("Done downloading!");
    return Ok(names[0].clone());
}
fn desktop_shortcut<N: Into<String>>(name: N, path: &std::path::Path) -> String {
    let name = name.into();
    let path = current_dir().unwrap().join(path);
    format!("[Desktop Entry]\nEncoding=UTF-8\nValue=1.0\nType=Application\nName={}\nGenericName={}\nComment={}\nIcon={}\nExec=\"{}\" \"\"\nCategories=Game;\nPath={}",name,name,name,path.join("support/icon.png").to_str().unwrap(),path.join("start.sh").to_str().unwrap(), path.to_str().unwrap())
}
