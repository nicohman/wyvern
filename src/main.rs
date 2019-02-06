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
extern crate curl;
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
mod connect;
mod sync;
use args::Command::Download;
use args::Command::*;
use args::Wyvern;
use args::{DownloadOptions, ShortcutOptions};
use config::*;
use crc::crc32;
use curl::easy::Easy;
use curl::easy::{Easy2, Handler, WriteError};
use gog::extract::*;
use gog::gog::{FilterParam::*, *};
use gog::token::Token;
use gog::Error;
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
    let gog = Gog::new(config.token.clone().unwrap());
    let mut sync_saves = config.sync_saves.clone();
    if sync_saves.is_some() {
        sync_saves = Some(
            sync_saves
                .unwrap()
                .replace("~", dirs::home_dir().unwrap().to_str().unwrap()),
        );
    }
    args.verbose
        .setup_env_logger("wyvern")
        .expect("Couldn't set up logger");
    confy::store("wyvern", config)?;
    match args.command {
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
            options,
            mut shortcuts,
        } => {
            if shortcuts.shortcuts {
                shortcuts.desktop = true;
                shortcuts.menu = true;
            }
            if let Some(search) = options.search.clone() {
                info!("Searching for games");
                let search_results =
                    gog.get_filtered_products(FilterParams::from_one(Search(search)));
                if search_results.is_ok() {
                    info!("Game search results OK");
                    let e = search_results.unwrap().products;
                    if !options.first {
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
                                        download_prep(&gog, details, &options).unwrap();
                                    if options.install_after.is_some() && !downloaded_windows {
                                        println!("Installing game");
                                        info!("Installing game");
                                        install_all(
                                            name,
                                            options.install_after.unwrap(),
                                            pname,
                                            &shortcuts,
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
                            download_prep(&gog, details, &options).unwrap();
                        if options.install_after.is_some() && !downloaded_windows {
                            println!("Installing game");
                            info!("Installing game");
                            install_all(name, options.install_after.unwrap(), pname, &shortcuts);
                        }
                    }
                } else {
                    println!("Could not find any games.");
                }
            } else if let Some(id) = options.id {
                let details = gog.get_game_details(id).unwrap();
                let pname = details.title.clone();
                info!("Beginning download process");
                let (name, downloaded_windows) = download_prep(&gog, details, &options).unwrap();
                if options.install_after.is_some() && !downloaded_windows {
                    println!("Installing game");
                    info!("Installing game");
                    install_all(name, options.install_after.unwrap(), pname, &shortcuts);
                }
            } else if options.all {
                println!("Downloading all games in library");
                let games = gog.get_games().unwrap();
                for game in games {
                    let details = gog.get_game_details(game).unwrap();
                    info!("Beginning download process");
                    download_prep(&gog, details, &options).unwrap();
                }
                if options.install_after.is_some() {
                    println!("--install does not work with --all");
                }
            } else {
                println!("Did not specify a game to download. Exiting.");
            }
        }
        Install {
            installer_name,
            path,
            mut shortcuts,
        } => {
            if shortcuts.shortcuts {
                shortcuts.desktop = true;
                shortcuts.menu = true;
            }
            info!("Opening installer");
            let mut installer = File::open(&installer_name);
            if installer.is_ok() {
                info!("Starting installation");
                install(&mut installer.unwrap(), path, installer_name, &shortcuts);
            } else {
                error!(
                    "Could not open installer. Error: {}",
                    installer.err().unwrap()
                );
            }
        }
        #[cfg(feature = "eidolonint")]
        UpdateEidolon { force, delta } => {
            use libeidolon::games::*;
            let eidolon_games = get_games();
            for game in eidolon_games {
                if let Ok(read) = read_game(game.as_str()) {
                    if read.typeg == GameType::WyvernGOG {
                        println!("Attempting to update {}", read.pname);
                        let path = PathBuf::from(read.command);
                        let ginfo_path = path.clone().join("gameinfo");
                        update(&gog, path, ginfo_path, force, delta);
                    }
                } else {
                    println!("Could not check {}", game);
                }
            }
        }
        Sync(..) => sync::parse_args(gog, sync_saves, args),
        Connect { .. } => connect::parse_args(gog, args),
        Update {
            mut path,
            force,

            delta,
        } => {
            if path.is_none() {
                info!("Path not specified. Using current dir");
                path = Some(PathBuf::from(".".to_string()));
            }
            let path = path.unwrap();
            let game_info_path = path.clone().join("gameinfo");
            info!("Updating game");
            update(&gog, path, game_info_path, force, delta);
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
            let details = gog
                .get_game_details(product.unwrap().products[0].id)
                .unwrap();
            info!("Getting game's linux downloads");
            let downloads = details
                .downloads
                .linux
                .expect("Game has no linux downloads");
            if delta {
                info!("Using delta-based updating");
                println!("Fetching installer data.");
                let data = gog.extract_data(downloads).unwrap();
                println!("Fetched installer data. Checking files.");
                io::stdout().flush();

                let access_token = gog.token.borrow().access_token.clone();
                data.par_iter().for_each(|data| {
                    let pb = ProgressBar::new(data.files.len() as u64);
                    pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}",
                        )
                        .progress_chars("#>-"),
                );
                    data.files.par_iter().for_each(|file| {
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
                                let mut fd = File::open(&path)
                                    .expect(&format!("Couldn't open file {:?}", path));
                                fd.read_to_end(&mut buffer).unwrap();
                                let checksum = crc32::checksum_ieee(buffer.as_slice());
                                if checksum == file.crc32 {
                                    info!("File {:?} is the same", path);
                                    pb.inc(1);
                                    return;
                                }
                                pb.println(format!("File {:?} is different. Downloading.", path));
                            } else if !path.exists() && is_dir {
                                pb.inc(1);
                                return;
                            } else if is_dir {
                                fs::create_dir_all(path);
                                pb.inc(1);
                                return;
                            } else {
                                pb.println(format!("File {:?} does not exist. Downloading.", path));
                            }
                            fs::create_dir_all(path.parent().unwrap());
                            info!("Fetching file from installer");
                            let easy = Gog::download_request_range_at(
                                access_token.as_str(),
                                data.url.as_str(),
                                gog::Collector(Vec::new()),
                                file.start_offset as i64,
                                file.end_offset as i64,
                            )
                            .unwrap();
                            let bytes = easy.get_ref().0.clone();
                            drop(easy);
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
                            pb.inc(1);
                        }
                    });
                    pb.finish_with_message("Updated game!");
                });
            } else {
                info!("Using regex to fetch version string. Will not work with DLCs.");
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
                    let name = download(&gog, downloads, &DownloadOptions::default()).unwrap();
                    println!("Installing.");
                    info!("Opening installer file");
                    let mut installer = File::open(&name[0]).unwrap();
                    info!("Starting installation");
                    install(
                        &mut installer,
                        path,
                        name[0].clone(),
                        &ShortcutOptions {
                            menu: false,
                            desktop: false,
                            shortcuts: false,
                        },
                    );
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
fn shortcuts(name: &String, path: &std::path::Path, shortcut_opts: &ShortcutOptions) {
    if shortcut_opts.menu || shortcut_opts.desktop {
        info!("Creating shortcuts");
        let game_path = current_dir().unwrap().join(&path);
        info!("Creating text of shortcut");
        let shortcut = desktop_shortcut(name.as_str(), &game_path);
        if shortcut_opts.menu {
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
        if shortcut_opts.desktop {
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
fn install_all(names: Vec<String>, path: PathBuf, name: String, shortcut_opts: &ShortcutOptions) {
    for name in names {
        info!("Installing {}", name);
        let mut installer = File::open(&name).expect("Could not open installer file");
        install(
            &mut installer,
            path.clone(),
            name.clone(),
            &ShortcutOptions {
                menu: false,
                desktop: false,
                shortcuts: false,
            },
        );
    }
    shortcuts(&name, path.as_path(), shortcut_opts);
}
fn download_prep(
    gog: &Gog,
    details: GameDetails,
    options: &DownloadOptions,
) -> Result<(Vec<String>, bool), Error> {
    if options.extras {
        println!("Downloading extras for game {}", details.title);
        let folder_name = PathBuf::from(format!("{} Extras", details.title));
        if fs::metadata(&folder_name).is_err() {
            fs::create_dir(&folder_name).expect("Couldn't create extras folder");
        }
        let extra_responses: Vec<Result<reqwest::Response, Error>> = details
            .extras
            .iter()
            .map(|x| {
                let mut url = "https://gog.com".to_string() + &x.manual_url;
                let mut response;
                loop {
                    let temp_response = gog.client_noredirect.borrow().get(&url).send();
                    if temp_response.is_ok() {
                        response = temp_response.unwrap();
                        let headers = response.headers();
                        // GOG appears to be inconsistent with returning either 301/302, so this just checks for a redirect location.
                        if headers.contains_key("location") {
                            url = headers
                                .get("location")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();
                        } else {
                            break;
                        }
                    } else {
                        return Err(temp_response.err().unwrap().into());
                    }
                }
                Ok(response)
            })
            .collect();
        for (i, extra) in extra_responses.into_iter().enumerate() {
            let mut extra = extra.expect("Couldn't fetch extra");
            let mut real_response = gog
                .client_noredirect
                .borrow()
                .get(extra.url().clone())
                .send()
                .expect("Couldn't fetch extra data");
            let name = extra
                .url()
                .path_segments()
                .unwrap()
                .last()
                .unwrap()
                .to_string();
            let n_path = folder_name.join(&name);
            if fs::metadata(&n_path).is_ok() {
                warn!("This extra has already been downloaded. Skipping.");
                continue;
            }
            println!("Starting download of {}", name);
            let pb = ProgressBar::new(extra.content_length().unwrap());
            pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .progress_chars("#>-"));
            let mut pb_read = pb.wrap_read(real_response);
            let mut file = File::create(n_path).expect("Couldn't create file");
            io::copy(&mut pb_read, &mut file).expect("Couldn't copy to target file");
            pb.finish();
        }
        std::process::exit(0);
    } else {
        if details.downloads.linux.is_some() && !options.windows_force {
            info!("Downloading linux downloads");
            let name;
            if options.dlc {
                info!("Downloading DLC");
                name = download(gog, all_downloads(details, true), options).unwrap();
            } else {
                name = download(gog, details.downloads.linux.unwrap(), options).unwrap();
            }
            return Ok((name, false));
        } else {
            if !options.windows_auto && !options.windows_force {
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
                            let name;
                            if options.dlc {
                                info!("Downloading DLC as well");
                                name =
                                    download(&gog, all_downloads(details, false), options).unwrap();
                            } else {
                                name = download(gog, details.downloads.windows.unwrap(), options)
                                    .unwrap();
                            }
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
                if !options.windows_force {
                    println!("No linux version available. Downloading windows version.");
                }
                info!("Downloading windows downloads");
                let name;
                if options.dlc {
                    info!("Downloading DLC as well");
                    name = download(&gog, all_downloads(details, false), options).unwrap();
                } else {
                    name = download(gog, details.downloads.windows.unwrap(), options).unwrap();
                }
                return Ok((name, true));
            }
        }
    }
}
fn install(installer: &mut File, path: PathBuf, name: String, shortcut_opts: &ShortcutOptions) {
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
    shortcuts(&name, path.as_path(), shortcut_opts);
}
pub fn login() -> Token {
    println!("It appears that you have not logged into GOG. Please go to the following URL, log into GOG, and paste the code from the resulting url's ?code parameter into the input here.");
    println!("https://login.gog.com/auth?client_id=46899977096215655&layout=client2%22&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code");
    io::stdout().flush().unwrap();
    let mut code = String::new();
    let token: Token;
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
    let games = gog.get_all_filtered_products(FilterParams::from_one(MediaType(1)))?;
    println!("Title - GameID");
    for game in games {
        println!("{} - {}", game.title, game.id);
    }
    Ok(())
}
fn download(
    gog: &Gog,
    downloads: Vec<gog::gog::Download>,
    options: &DownloadOptions,
) -> Result<Vec<String>, Error> {
    info!("Downloading files");
    let mut names = vec![];
    for download in downloads.iter() {
        names.push(download.name.clone());
    }
    info!("Getting responses to requests");
    let responses = gog.download_game(downloads.clone());
    let count = responses.len();
    for (idx, mut response) in responses.into_iter().enumerate() {
        let mut responses = gog.download_game(vec![downloads[idx].clone()]);
        let mut response = responses.remove(0);
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
            let mut name = names[idx].clone();
            let url = response.url().clone();
            let final_name = url.path_segments().unwrap().last().unwrap().to_string();
            if options.original {
                name = final_name;
                names[idx] = name.clone();
            }
            if options.resume {
                if let Ok(mut meta) = fs::metadata(&name) {
                    if meta.len() >= total_size {
                        println!("Resuming {}, {} of {}", name, idx + 1, count);
                        pb.set_position(meta.len());
                        let mut fd = OpenOptions::new().append(true).open(&name)?;
                        let handler = WriteHandler {
                            writer: fd,
                            pb: Some(pb),
                        };
                        let mut result = Gog::download_request_range_at(
                            gog.token.borrow().access_token.as_str(),
                            url.as_str(),
                            handler,
                            meta.len() as i64,
                            total_size as i64,
                        )?;
                        let fd_ref = result.get_mut();
                        fd_ref.pb.take().unwrap().finish();
                        continue;
                    } else {
                        error!("This file is larger than or equal to the total size of the file. Not downloading anything.");
                        continue;
                    }
                } else {
                    info!("No file to resume from. Continuing as normal.");
                }
            } else {
                if PathBuf::from(name.as_str()).exists() {
                    error!("The file {} already exists.", name);
                    std::process::exit(64);
                }
            }
            println!("Downloading {}, {} of {}", name, idx + 1, count);
            info!("Creating file");
            let mut fd = fs::File::create(&name)?;
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
    return Ok(names);
}
fn desktop_shortcut(name: impl Into<String>, path: &std::path::Path) -> String {
    let name = name.into();
    let path = current_dir().unwrap().join(path);
    format!("[Desktop Entry]\nEncoding=UTF-8\nValue=1.0\nType=Application\nName={}\nGenericName={}\nComment={}\nIcon={}\nExec=\"{}\" \"\"\nCategories=Game;\nPath={}",name,name,name,path.join("support/icon.png").to_str().unwrap(),path.join("start.sh").to_str().unwrap(), path.to_str().unwrap())
}
fn all_downloads(details: GameDetails, linux: bool) -> Vec<gog::gog::Download> {
    let downloads;
    if linux {
        downloads = details.downloads.linux.unwrap();
    } else {
        downloads = details.downloads.windows.unwrap();
    }
    downloads
        .into_iter()
        .chain(
            details
                .dlcs
                .into_iter()
                .map(|x| {
                    let title = x.title.clone();
                    let mut d;
                    if linux {
                        d = x.downloads.linux.unwrap();
                    } else {
                        d = x.downloads.windows.unwrap();
                    }
                    d = d
                        .into_iter()
                        .map(|mut y| {
                            y.name = title.clone();
                            y
                        })
                        .collect();
                    d
                })
                .flatten(),
        )
        .collect()
}
struct WriteHandler {
    writer: File,
    pb: Option<ProgressBar>,
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
