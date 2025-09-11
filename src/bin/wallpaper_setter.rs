use clap::Parser;
use std::{fs,io, error::Error, path::PathBuf};
use rand::seq::IndexedRandom;

use wallpaper_utils::utils;

fn get_files(path: &PathBuf) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let file = entry?;
            let file_path = file.path().canonicalize()?;

            files.push(file_path);
        }
    }

    Ok(files)
}

/// wallpaper setter for linux
#[derive(Parser, Debug)]
struct Cli {
    /// arg to search for wallpapers
    #[arg(long, default_value_t = false)]
    find_wal: bool,

    /// decide to be using pywal after selecting the wallpaper
    #[arg(short, long, default_value_t = true)]
    pywal: bool,

    /// decice whether to use a random wallpaper
    #[arg(short, long, default_value_t = false)]
    random: bool,

    /// wallpaper to use
    #[arg(long)]
    wallpaper_file: Option<PathBuf>,

    /// if using random wallpaper, choose the directory to select from
    #[arg(long, default_value_os_t = utils::get_default_wall_dir())]
    wallpaper_dir: PathBuf,

    /// decide to set the same wallpaper across multiple monitors
    #[arg(short, long, default_value_t = true)]
    multiple_monitors: bool
}
fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::parse();
    let path = args.wallpaper_dir;
    let desktop_env = utils::get_desktop_env();

    if args.random {
        let files = get_files(&path)?;
        if let Some(random_file) = files.choose(&mut rand::rng()) {
            let file_path_str = random_file.to_str().unwrap();
            println!("Chosen file: {}", file_path_str);
            if args.pywal {
                utils::apply_pywal(file_path_str)?;
            }
            utils::apply_wallpaper(file_path_str, desktop_env, args.multiple_monitors)?;
        }
    } else {
        if let Some(wallpaper_path) = args.wallpaper_file {
            let file_path = wallpaper_path.canonicalize()?;
            let file_path_str = file_path.to_str().unwrap();
            println!("Chosen file: {}", file_path_str);
            if args.pywal {
                utils::apply_pywal(file_path_str)?;
            }
            utils::apply_wallpaper(file_path_str, desktop_env, args.multiple_monitors)?;
        }

    }

    Ok(())
}
