use args::Sync::*;
use args::Wyvern::*;
use config::*;
use gog::gog::FilterParam::*;
use gog::gog::*;
use gog::*;
use parse_gameinfo;
use std::env::current_dir;
use std::fs::{self, File};
use std::io::{self, *};
use std::path::*;
use std::process::*;
pub fn parse_args(gog: Gog, sync_saves: Option<String>, args: ::args::Wyvern) {
    match args {
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
                                print!("Are you sure this where the save files are located?(Y/n)");
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
                        let mut savedb_path = PathBuf::from(sync_saves.clone()).join("savedb.json");
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
    };
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
