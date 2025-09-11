use std::{fs, time::Duration, thread, path::PathBuf, env, io::{self, Write}, process::{Output, Command}};
use home::home_dir;
use hyprland::data::{Monitors};
use hyprland::prelude::*;

#[derive(PartialEq, Debug)]
pub enum DesktopEnv {
    Gnome,
    DWM,
    Hyprland,
    Other
}

pub fn get_desktop_env() -> DesktopEnv {
    match env::var("XDG_SESSION_DESKTOP") {
        Ok(desktop) => match desktop.to_lowercase().as_str() {
            "hyprland" => DesktopEnv::Hyprland,
            "gnome" => DesktopEnv::Gnome,
            "dwm" => DesktopEnv::DWM,
            _ => DesktopEnv::Other
        },
        Err(_) => DesktopEnv::Other
    }
}

fn is_process_running(process: &str) -> io::Result<bool> {
    Ok(String::from_utf8_lossy(&Command::new("pgrep").arg(process).output()?.stdout) != "")
}

fn exec(cmd: &str, args: Vec<&str>) -> std::io::Result<Output> {
    Command::new(cmd).args(args).output()
}

pub fn apply_wallpaper(file_path_str: &str, desktop_env: DesktopEnv, multiple_monitors: bool) -> io::Result<()> {
    match desktop_env {
        DesktopEnv::Gnome => {
            let user = env::var("USER").unwrap_or_else(|_| String::new());
            let gnome_pid_out = Command::new("pgrep")
                .args(["-xu", &user, "gnome-shell"])
                .output()?;

            let gnome_pid = String::from_utf8_lossy(&gnome_pid_out.stdout)
                .lines()
                .next()
                .expect("gnome pid not found")
                .trim()
                .to_string();

            let environ = fs::read(format!("/proc/{}/environ", &gnome_pid))?;
            for var in environ.split(|&b| b == 0) {
                if let Ok(var_str) = std::str::from_utf8(var) {
                    if var_str.starts_with("DBUS_SESSION_BUS_ADDRESS=") {
                        let parts: Vec<&str> = var_str.splitn(2, "=").collect();
                        if parts.len() == 2 {
                            env::set_var(parts[0], parts[1]);
                        }
                        break;
                    }
                }
            }

            let picture_uri = String::from("file:///") + file_path_str;

            Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.desktop.background",
                    "picture-uri",
                    &picture_uri,
                ])
                .stdout(std::process::Stdio::null())
                .status()?;
            Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.desktop.background",
                    "picture-uri-dark",
                    &picture_uri,
                ])
                .stdout(std::process::Stdio::null())
                .status()?;
            Command::new("gsettings")
                .args([
                    "set",
                    "org.gnome.desktop.background",
                    "picture-options",
                    "\"zoom\"",
                ])
                .stdout(std::process::Stdio::null())
                .status()?;
        }

        DesktopEnv::Hyprland => {
            // let hyprland and hyprpaper load up
            while !is_process_running("hyprpaper")? {
                println!("sleeping");
                thread::sleep(Duration::from_millis(100));
            }

            while String::from_utf8_lossy(&exec("hyprctl", Vec::from([ "hyprpaper" ]))?.stdout).contains("sock") {
                println!("hyprpaper not running");
                thread::sleep(Duration::from_millis(100));
            }


            Command::new("hyprctl")
                .args(["hyprpaper", "unload", "all"])
                .stdout(std::process::Stdio::null())
                .status()?;
            

            if let Ok(monitors) = Monitors::get() {
                for monitor in monitors {
                    let monitor_name = monitor.name;
                    Command::new("hyprctl")
                        .args(["hyprpaper", "preload", file_path_str])
                        .stdout(std::process::Stdio::null())
                        .status()?;
                    
                    Command::new("hyprctl")
                        .args([
                            "hyprpaper",
                            "wallpaper",
                            format!("{},{}", monitor_name, file_path_str).as_str(),
                        ])
                        .stdout(std::process::Stdio::null())
                        .status()?;

                    if !multiple_monitors {
                        break;
                    }

                }
            }

        }

        DesktopEnv::DWM => {
            Command::new("xwallpaper")
                .args(["--clear"])
                .stdout(std::process::Stdio::null())
                .status()?;
            Command::new("xwallpaper")
                .args(["--zoom", file_path_str])
                .stdout(std::process::Stdio::null())
                .status()?;
        },

        DesktopEnv::Other => todo!()
    }

    let current_wall = home_dir()
        .expect("Failed to get home directory")
        .join(".current_wall.txt");
    let mut file = fs::File::create(&current_wall).expect("Couldn't write file");
    file.write_all(file_path_str.as_bytes())?;

    Ok(())
}


pub fn get_default_wall_dir() -> PathBuf {
    home_dir()
        .expect("Failed to get home directory")
        .join("pics/wallpapers/anime")
}

pub fn apply_pywal(file_path_str: &str) -> io::Result<()> {
    Command::new("wal")
        .args(["-q", "-i", file_path_str])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    Ok(())
}
