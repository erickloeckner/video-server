use std::{env, process};
use std::fs;

use chrono::prelude::*;
use serde::Deserialize;
use warp::Filter;

#[derive(Clone, Deserialize)]
struct Config {
    port: u16,
    fs_path: String,
    uri_path: String,
    instance_name: String,
    video_count: u8,
}

fn list_files(path: &str) -> Vec<String> {
    let mut out = Vec::new();
    match std::fs::read_dir(path) {
        Ok(v) => {
            for file in v {
                match file {
                    Ok(f) => {
                        out.push(format!("{}", f.file_name().to_str().unwrap_or(" ")));
                    }
                    Err(_) => {}
                }
            }
        }
        
        Err(_) => {}
    }
    out.sort();
    out.reverse();
    out
}

fn render_list(files: Vec<String>, path: &str, instance_name: &str, video_count: u8) -> String {
    let html_head = format!("<!DOCTYPE html>
<html>
  <head>
    <title>Video Server</title>
    <meta charset=\"utf-8\">
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{}/static/style.css\">
  </head>
  <body>
", path);

    let html_foot = "  </body>\n</html>\n";

    let mut html_body = String::new();
    html_body.push_str("    <div class=\"main-div\">\n");
    html_body.push_str(&format!("    <div><h1>{}</div>\n", instance_name));

    for (i, v) in files.iter().enumerate() {
        if i < (video_count as usize) {
            html_body.push_str(&format!(
                "      <div class=\"video-div\"><video class=\"video-el\" controls><source src=\"{}/video/{}\"/></video></div>\n",
                path, v
            ));
        }
        let date = parse_ts(v);
        html_body.push_str(&format!("      <div class=\"link-div\"><a href=\"{}/video/{}\">{}</a></div>\n", path, v, date));
    }

    html_body.push_str(&format!("    </div>\n"));

    format!("{}{}{}", html_head, html_body, html_foot)
}

fn parse_ts(name: &str) -> String {
    match chrono::DateTime::parse_from_str(name.split(".").nth(0).unwrap_or("0"), "%s") {
        Ok(v) => {
            let dt_tz = v.with_timezone(&Local);
            dt_tz.format("%e %b %Y %T").to_string()
        }
        Err(_) => {
            String::from(name)
        }
    }
}

#[tokio::main]
async fn main() {
    let config_path = env::args().nth(1).unwrap_or_else(|| {
        println!("no config file specified");
        process::exit(1);
    });
    let config_raw = fs::read_to_string(&config_path).unwrap_or_else(|err| {
        println!("error reading config: {}", err);
        process::exit(1);
    });
    let config: Config = toml::from_str(&config_raw).unwrap_or_else(|err| {
        println!("error parsing config: {}", err);
        process::exit(1);
    });

    let video_dir = warp::path("video")
        .and(warp::fs::dir(config.fs_path.clone()));

    let static_dir = warp::path("static")
        .and(warp::fs::dir("./static"))
        .map(|reply| {
            warp::reply::with_header(reply, "cache-control", "no-cache")
        });

    let port = config.port;

    let config_filter = warp::any().map(move || config.clone());

    let index = warp::get()
        .and(warp::path::end())
        .and(config_filter.clone())
        .map(|c: Config| {
            let files = list_files(&c.fs_path);
            let content = render_list(files, &c.uri_path, &c.instance_name, c.video_count);
            warp::reply::html(content)
        });

    let routes = index
        .or(video_dir)
        .or(static_dir);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}
