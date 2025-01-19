use reqwest as request;
use clap::Parser;
use std::fs::File;
use std::{error::Error, io::Write};
use serde_json::Value;
use image::load_from_memory;
use viuer::Config;
use std::process::Command;
use hyprland::data::Monitor;
use hyprland::prelude::*;
use dotenvy;
use std::env;

/// A CLI tool that allows you to set a wallpaper for hyprland using api (wallhaven.cc).
#[derive(Parser)]
struct Args {
    /// wallpaper type to set (anime, other)
    #[arg(short, long, default_value_t = String::from("other"))]
    wal_type: String,

    /// wallpaper query to search
    #[arg(short, long, default_value_t = String::new())]
    query: String,

    /// pywal after wallpaper is set
    #[arg(short, long, default_value_t = false)]
    pywal: bool
}

enum Category {
    Anime,
    OtherAnime,
    AnimeNsfw,
    Other,
}


async fn get_wallpapers(api_key: &str, wall_type: &Category, query: &str) -> Result<Value, Box<dyn Error>> {
    let category = match wall_type {
        Category::Anime => "010",
        Category::Other => "100",
        Category::AnimeNsfw => "010",
        _ => "000"
    };

    let purity = match wall_type {
        Category::AnimeNsfw => "011",
        _ => "100"
    };
    
    let url = String::from("https://wallhaven.cc/api/v1/search?sorting=random&resolutions=1920x1080&q=")
        + format!("&categories={}&purity={}&q={}&apikey={}", category, purity, query, api_key).as_str();

    let result = request::get(url)
        .await
        .unwrap();

    match result.status() {
        request::StatusCode::OK => {
            println!("Success!");
        },
        _ => {
            eprintln!("Request Error");
        }
    }

    let ret_text = result.text().await?;
    let json: Value = serde_json::from_str(&ret_text)?;

    Ok(json)
}

fn get_wallpaper(category: &Category, wallpapers: &Value, index: usize) -> Option<String> {
    match category {
        Category::OtherAnime => Some(String::from("https://pic.re/image")),
        _ => {
            if let Some(url) = wallpapers["data"][index]["path"].as_str() {
                Some(url.to_string())
            } else {
                None
            }
        }
   }
}

async fn fetch_image(url: &str) -> Result<(bytes::Bytes, Option<String>), Box<dyn Error>> {
    let response = request::get(url).await?;
    let headers = response.headers().clone();
    let image_bytes = response.bytes().await?;

    let image_source = headers.get("image_source").and_then(|header_value| {
        header_value.to_str().ok().map(|s| format!("'{}'.{}", s.to_string().replace("/", "%2F"), headers.get("Content-Type").unwrap().to_str().unwrap().split("/").last().unwrap()))
    });

    Ok((image_bytes, image_source))
}

async fn print_wal(image_bytes: &bytes::Bytes) -> Result<(), Box<dyn Error>> {
    let img = load_from_memory(image_bytes)?;

    let vieur_conf = Config {
        absolute_offset: false,

        ..Config::default()
    };

    viuer::print(&img, &vieur_conf)?;

    Ok(())
}

fn set_wal(file_path: &str, file_name: &str) -> Result<(), Box<dyn Error>> {
    Command::new("hyprctl")
        .args(["hyprpaper", "unload", "all"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Hyprctl unload err");
    Command::new("hyprctl")
        .args(["hyprpaper", "preload", file_path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Hyprctl preload err");

    let monitor = Monitor::get_active()?.name;
    Command::new("hyprctl")
        .args(["hyprpaper", "wallpaper", format!("{},{}", monitor, file_path).as_str()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("Hyprctl wallpaper err");

    Command::new("notify-send")
        .args(["-i", file_path, "Wallpaper set", ("New wallpaper set to ".to_string() + file_name).as_str()])
        .stdout(std::process::Stdio::null())
        .stdin(std::process::Stdio::null())
        .spawn()
        .expect("notify error send");

    Ok(())
}

fn apply_pywal(file_path: &str) -> Result<(), Box<dyn Error>> {
    Command::new("wal")
        .args(["-q", "-i", file_path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("Pywal err");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    dotenvy::dotenv()?;

    let home_dir = home::home_dir().unwrap();

    let category = match args.wal_type.as_str() {
        "anime" | "a" => Category::Anime,
        "other" | "o" => Category::Other,
        "oa" => Category::OtherAnime,
        "n" | "nsfw" => Category::AnimeNsfw,
        _ => Category::Other
    };

    println!("Selected category: {}", match category {
        Category::Anime => "Anime",
        Category::Other => "Other",
        Category::OtherAnime => "Anime (2nd api)",
        Category::AnimeNsfw => "Anime (nsfw)"
    });

    let wallpaper_path = home_dir.join("pics/wallpapers").join(match category {
        Category::Anime => "anime",
        Category::Other => "other",
        Category::OtherAnime => "anime_art",
        Category::AnimeNsfw => "nsfw"
    });


    let api_key = env::var("API_KEY").unwrap_or(String::new());

    let wallpapers = get_wallpapers(&api_key, &category, &args.query).await?;

    let mut counter = 0;
    let mut current_url = get_wallpaper(&category, &wallpapers, counter);
    let mut current_img = bytes::Bytes::new();
    let mut img_src = String::new();

    loop {
        if let Some(url) = &current_url {
            let (image_bytes, image_source) = fetch_image(url).await?;
            current_img = image_bytes;
            print_wal(&current_img).await?;

            img_src = match category {
                Category::OtherAnime => {
                    if let Some(src) = image_source {
                        src
                    } else {
                        String::new()
                    }
                },
                _ => String::new()
            }
        }
        println!("Would you like to set this wallpaper? (y/n)");
        let mut inp = String::new();
        std::io::stdin().read_line(&mut inp)?;

        match inp.trim().to_lowercase().as_str() {
            "y" => break,
            "n" => counter += 1,
            _ => eprintln!("Not an option")
        };

        current_url = get_wallpaper(&category, &wallpapers, counter);
    }

    let url = current_url.unwrap();
    let file_name = match category {
        Category::OtherAnime => img_src,
        _ => url.split("/").last().unwrap().to_string()
    };
    let file_path = wallpaper_path.join(file_name);
    
    let mut file = File::create(&file_path)
        .expect("Couldn't write to file");
    file.write_all(&current_img)?;

    drop(file); // make sure it's written by closing it

    let file_path_str = file_path.to_str().unwrap();
    let file_name = file_path.file_name().unwrap();

    if args.pywal {
        apply_pywal(&file_path_str)?;
    }
    set_wal(&file_path_str, &file_name.to_str().unwrap())?;

    let mut saved = File::create(home_dir.join(".current_wall.txt"))
        .expect("Coudln't write to file");
    saved.write_all(file_path_str.as_bytes())?;

    Ok(())
}
