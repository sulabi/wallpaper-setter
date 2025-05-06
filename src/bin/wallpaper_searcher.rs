use reqwest as request;
use clap::Parser;
use std::fs::File;
use std::{error::Error, io::Write};
use serde_json::Value;
use image::load_from_memory;
use viuer::Config;
use std::process::exit;
use wallpaper_utils::utils;

/// A CLI tool that allows you to set a wallpaper for linux using api (wallhaven.cc).
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
    pywal: bool,

    /// decide to set the same wallpaper across multiple monitors
    #[arg(short, long, default_value_t = true)]
    multiple_monitors: bool
}

enum Category {
    Anime,
    OtherAnime,
    AnimeNsfw,
    Other,
}


async fn get_wallpapers(api_key: &str, wall_type: &Category, query: &str, page: i32) -> Result<Value, Box<dyn Error>> {
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
    
    let url = String::from("https://wallhaven.cc/api/v1/search?sorting=random&resolutions=1920x1080")
        + format!("&categories={}&purity={}&q={}&page={}&apikey={}", category, purity, query, page, api_key).as_str();

    let result = request::get(url)
        .await
        .unwrap();

    match result.status() {
        request::StatusCode::OK => {
            // println!("Success!");
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
        Category::OtherAnime => Some(String::from("https://pic.re/image?compress=false")),
        _ => {
            if let Some(url) = wallpapers["data"][index]["path"].as_str() {
                Some(url.to_string())
            } else {
                None
            }
        }
   }
}

async fn fetch_image(url: &str) -> Result<(bytes::Bytes, String), Box<dyn Error>> {
    let response = request::get(url).await?;
    let headers = response.headers().clone();
    let image_bytes = response.bytes().await?;

    let image_id = if let Some(id) = headers.get("image_id") {
        id.to_str().unwrap()
    } else {
        ""
    };

    Ok((image_bytes, image_id.to_string()))
}

async fn print_wal(image_bytes: &bytes::Bytes) -> Result<(), Box<dyn Error>> {
    let img = load_from_memory(image_bytes)?;

    let vieur_conf = Config {
        absolute_offset: false,
        use_sixel: true,
        ..Config::default()
    };

    viuer::print(&img, &vieur_conf)?;
    println!("");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let api_key = dotenvy_macro::dotenv!("API_KEY");

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

    let mut page = 1;
    let mut wallpapers = get_wallpapers(&api_key, &category, &args.query, page).await?;

    let mut counter = 0;
    let mut current_url = get_wallpaper(&category, &wallpapers, counter);
    let mut current_img: bytes::Bytes;
    let mut img_id: String;

    loop {
        let wallpaper_count = wallpapers["meta"]["total"].as_i64()
            .and_then(|total| {
                Some(wallpapers["meta"]["per_page"].as_str()?.parse::<i64>().unwrap().min(total))
            })
            .unwrap();

        if let Some(url) = &current_url {
            println!("{}", url);
            let (image_bytes, image_id) = fetch_image(url).await?;
            img_id = image_id;
            current_img = image_bytes;
            print_wal(&current_img).await?;
        } else {
            if wallpaper_count as usize == counter && page != wallpapers["meta"]["last_page"] {
                page += 1;
                counter = 0;
                println!("going to next page {}", page);
                wallpapers = get_wallpapers(&api_key, &category, &args.query, page).await?;
                current_url = get_wallpaper(&category, &wallpapers, counter);
                continue;
            } else {
                println!("{}", if counter == 0 { "There are no results" } else { "There are no more wallpapers" });
                exit(0);
            }

        }
        match category {
            Category::OtherAnime => {
                println!("Would you like to set this wallpaper? (y/n)");
            },
            _ => {
                println!("Would you like to set this wallpaper? ({}/{})({}/{}) (y/n)", counter + 1, wallpaper_count, page, wallpapers["meta"]["last_page"]);
            }


        }
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
        Category::OtherAnime => format!("{}.png", img_id),
        _ => url.split("/").last().unwrap().to_string()
    };
    let file_path = wallpaper_path.join(file_name);
    
    if current_img.starts_with(b"RIFF") && current_img[8..12] == *b"WEBP" {
        // convert to png if webp image
        let img = load_from_memory(&current_img)?;
        img.save(&file_path)?;
    } else {
        let mut file = File::create(&file_path)
            .expect("Couldn't write to file");
        file.write_all(&current_img)?;

        drop(file); // make sure it's written by closing it
    }
    
    let desktop_env = utils::get_desktop_env();

    let file_path_str = file_path.to_str().unwrap();

    if args.pywal {
        println!("Applying pywal changes");
        utils::apply_pywal(file_path_str)?;
    }
    utils::apply_wallpaper(&file_path_str, desktop_env, args.multiple_monitors)?;
    println!("Set wallpaper");

    let mut saved = File::create(home_dir.join(".current_wall.txt"))
        .expect("Coudln't write to file");
    saved.write_all(file_path_str.as_bytes())?;

    Ok(())
}
