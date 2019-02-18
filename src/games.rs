use crate::*;
use rayon::prelude::*;
use std::io::BufReader;
use std::io::Cursor;
use std::io::Read;
use std::io::Seek;
pub fn update(gog: &Gog, _path: PathBuf, game_info_path: PathBuf, dlc: bool) {
    if let Ok(mut gameinfo) = File::open(&game_info_path) {
        let mut ginfo_string = String::new();
        info!("Reading in gameinfo file");
        gameinfo.read_to_string(&mut ginfo_string).unwrap();
        info!("Parsing gameinfo");
        let ginfo = parse_gameinfo(ginfo_string);
        let name = ginfo.name.clone();
        info!("Searching GOG products for {}", name);
        if let Ok(product) = gog.get_filtered_products(FilterParams::from_one(Search(name.clone())))
        {
            info!("Fetching the GameDetails for first result of search");
            if product.products.len() < 1 {
                error!("Could not find a game named {} in your library.", name);
                std::process::exit(64);
            }
            let details = gog.get_game_details(product.products[0].id).unwrap();
            info!("Getting game's linux downloads");
            let mut downloads;
            if dlc {
                info!("Using DLC to update");
                downloads = all_downloads(details, true);
            } else {
                downloads = details
                    .downloads
                    .linux
                    .expect("Game has no linux downloads");
            }
            info!("Fetching installer data.");
            let data = gog.extract_data(downloads).unwrap();
            println!("Fetched installer data. Checking files.");
            io::stdout().flush().expect("Couldn't flush stdout");
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
                            let mut fd =
                                File::open(&path).expect(&format!("Couldn't open file {:?}", path));
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
                        if file.compression_method == 8 {
                            info!("Decompressing file");
                            let def = inflate::inflate_bytes(bytes.as_slice()).unwrap();
                            info!("Writing decompressed file to disk");
                            fd.write_all(&def)
                                .expect(&format!("Couldn't write to file {:?}", path));
                        } else {
                            println!("Writing file to disk normally");
                            fd.write_all(&bytes)
                                .expect(&format!("Couldn't write to file {:?}", path));
                        }

                        pb.inc(1);
                    }
                });
                pb.finish_with_message("Updated game!");
            });
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
pub fn download_prep(
    gog: &Gog,
    details: GameDetails,
    options: &DownloadOptions,
) -> Result<(Vec<String>, bool), Error> {
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
            if Confirmation::new().with_text("This game does not support linux! Would you like to download the windows version to run under wine?").interact().unwrap() {
                println!("Downloading windows files. Note: wyvern does not support automatic installation from windows games");
                info!("Downloading windows downloads");
                let name;
                if options.dlc {
                    info!("Downloading DLC as well");
                    name = download(&gog, all_downloads(details, false), options).unwrap();
                } else {
                    name = download(gog, details.downloads.windows.unwrap(), options).unwrap();
                }
                return Ok((name, true));

            } else {
                error!("No suitable downloads found. Exiting");
                std::process::exit(0);
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
pub fn download(
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
    let count = downloads.len();
    for idx in 0..count {
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
            let final_name_encoded = url.path_segments().unwrap().last().unwrap().to_string();
            let final_name = url::percent_encoding::percent_decode(&final_name_encoded.as_bytes())
                .decode_utf8()
                .unwrap()
                .to_string();
            let final_name_path = PathBuf::from(&final_name);
            if options.original {
                name = final_name;
            }
            if let Some(output) = options.output.clone() {
                if output.is_dir() {
                    name = output
                        .join(PathBuf::from(&name))
                        .to_str()
                        .unwrap()
                        .to_string();
                } else {
                    name = output.to_str().unwrap().to_string();
                }
            }
            if options.preserve_extension {
                if let Some(extension) = final_name_path.extension() {
                    name = name + "." + &extension.to_string_lossy().to_string();
                }
            }
            let mut i = 1;
            let mut name_path = PathBuf::from(&name);
            let filename = name_path.file_name().unwrap().to_str().unwrap().to_string();
            loop {
                if name_path.exists() {
                    info!("Current path exists. Incrementing tail, trying again");
                    name_path.set_file_name(format!("{}_{}", filename, i));
                } else {
                    break;
                }
            }
            name = name_path.to_str().unwrap().to_string();
            names[idx] = name.clone();
            let temp_name = name.clone() + ".tmp";
            if options.resume {
                if let Ok(mut meta) = fs::metadata(&temp_name) {
                    if meta.len() >= total_size {
                        println!("Resuming {}, {} of {}", name, idx + 1, count);
                        pb.set_position(meta.len());
                        let mut fd = OpenOptions::new().append(true).open(&temp_name)?;
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
            let mut fd = fs::File::create(&temp_name)?;
            let mut perms = fd.metadata()?.permissions();
            info!("Setting permissions to executable");
            perms.set_mode(0o744);
            fd.set_permissions(perms)?;
            let mut pb_read = pb.wrap_read(response);
            io::copy(&mut pb_read, &mut fd)?;
            fs::rename(&temp_name, &name)?;
            pb.finish();
        }
    }
    println!("Done downloading!");
    return Ok(names);
}
pub fn parse_gameinfo(ginfo: String) -> GameInfo {
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
pub fn install_all(
    names: Vec<String>,
    path: PathBuf,
    name: String,
    shortcut_opts: &ShortcutOptions,
    windows: bool,
    external_zip: bool,
) {
    for name in names
        .iter()
        .filter(|x| if windows { x.contains("exe") } else { true })
    {
        install(
            name.as_str(),
            path.clone(),
            name.clone(),
            &ShortcutOptions {
                menu: false,
                desktop: false,
                shortcuts: false,
            },
            windows,
            external_zip,
        );
    }
    shortcuts(&name, path.as_path(), shortcut_opts);
}
pub fn install(
    installer: impl Into<String>,
    path: PathBuf,
    name: String,
    shortcut_opts: &ShortcutOptions,
    windows: bool,
    external_zip: bool,
) {
    info!("Starting installer extraction process");
    if windows {
        info!("Extracting windows game using innoextract");
        let output = Command::new("innoextract")
            .arg("--exclude-temp")
            .arg("--gog")
            .arg("--output-dir")
            .arg(path.to_str().expect("Couldn't convert path to string"))
            .arg(installer.into())
            .output();
        if let Ok(output) = output {
            if output.status.success() {
                info!("innoextract successfully run");
                if path.join("app").is_dir() {
                    info!("Game is nested within app directory. Attempting to rename.");
                    fs::rename(path.join("app"), "tmp").expect("Couldn't rename app folder");
                    fs::remove_dir_all(&path).expect("Couldn't remove old folder");
                    fs::rename("tmp", &path).expect("Couldn't rename tmp folder");
                }
                return;
            } else {
                error!("Could not run innoextract. Are you sure it's installed and in $PATH?");
                error!("Stdout: {}", String::from_utf8_lossy(&output.stdout));
                error!("Stderr: {}", String::from_utf8_lossy(&output.stderr));
                std::process::exit(64);
            }
        } else {
            error!("Could not run innoextract. Are youu sure it's installed and in $PATH?");
            error!("Error: {:?}", output.err().unwrap());
            std::process::exit(64);
        }
    } else {
        if external_zip {
            info!("Unzipping using unzip command");
            if let Err(err) = fs::create_dir("tmp") {
                warn!("Could not create temporary extract dir. Error: {:?}", err);
            }
            println!("Starting unzip of installer.");
            let output = Command::new("unzip")
                .arg("-d")
                .arg("tmp")
                .arg("/tmp/data.zip")
                .output();
            if let Err(err) = output {
                error!("Unzip command failed. Error: {:?}", err);
            } else {
                let output = output.unwrap();
                if output.status.success() {
                    info!("Unzip command succeeded. Beginning path processing.");
                    for entry in WalkDir::new("tmp").into_iter().filter_map(|e| e.ok()) {
                        let new_path = path
                            .join(
                                entry
                                    .path()
                                    .strip_prefix(Path::new("tmp"))
                                    .expect("Couldn't strip path")
                                    .to_str()
                                    .unwrap()
                                    .replace("/noarch", "")
                                    .replace("data/", "")
                                    .to_owned(),
                            )
                            .to_path_buf();
                        let str_new = format!("{}", new_path.clone().to_str().unwrap());
                        if !str_new.contains("meta") && !str_new.contains("scripts") {
                            if str_new.ends_with("/") {
                                info!("Creating dir");
                                fs::create_dir_all(new_path).expect("Couldn't create directory");
                            } else {
                                if let Some(p) = new_path.as_path().parent() {
                                    if !p.exists() {
                                        fs::create_dir_all(&p)
                                            .expect("Couldn't create parent directory!");
                                    }
                                }
                                if entry.path().is_dir() {
                                    fs::create_dir_all(new_path);
                                } else {
                                    info!("Moving file");
                                    fs::rename(entry.path(), new_path.as_path())
                                        .expect("Couldn't move file to proper directory");
                                }
                            }
                        }
                    }
                    fs::remove_dir_all("tmp").expect("Could not remove temp directory");
                } else {
                    error!(
                        "Unzip command failed.\n Stdout: {:?}\n Stderr: {:?}",
                        output.stdout, output.stderr
                    );
                }
            }
        } else {
            if let Ok(mut installer) = File::open(installer.into()) {
                extract(
                    &mut installer,
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
                        .template(
                            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len}",
                        )
                        .progress_chars("#>-"),
                );
                info!("Starting zip extraction process");
                (0..len).into_par_iter().for_each(|i| {
                    info!("Starting extraction of file #{}. Opening archive", i);
                    let mut archive =
                        zip::ZipArchive::new(BufReader::new(File::open("/tmp/data.zip").unwrap()))
                            .unwrap();
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
                            fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                                .unwrap();
                        }
                    } else {
                        info!("File {} not being extracted", filtered_path);
                    }
                    pb.inc(1);
                });
                pb.finish_with_message("Game installed!");
                shortcuts(&name, path.as_path(), shortcut_opts);
            } else {
                error!("Could not open installer file");
                return;
            }
        }
    }
    #[cfg(feature = "eidolonint")]
    {
        if !windows {
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
    }
}
pub struct WriteHandler {
    pub writer: File,
    pub pb: Option<ProgressBar>,
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
#[derive(Serialize, Debug)]
pub struct GamesList {
    pub games: Vec<Game>,
}
#[derive(Serialize, Debug)]
pub enum Game {
    ProductInfo(ProductDetails),
    GameInfo(GameDetails, i64),
}
impl Game {
    pub fn title(&self) -> String {
        match self {
            Game::ProductInfo(details) => details.title.clone(),
            Game::GameInfo(details, _id) => details.title.clone(),
        }
    }
}
