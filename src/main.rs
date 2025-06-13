use std::collections::HashMap;
use std::{env, process};
use std::fs;
use std::fmt::Write;

use chrono::prelude::*;
use serde::Deserialize;
use warp::Filter;

#[derive(Clone, Deserialize)]
struct Config {
    port: u16,
    fs_path: String,
    uri_path: String,
    instance_name: String,
    video_count: u16,
    sort_descending: bool,
    parse_timestamps: bool,
    disable_static_cache: bool, 
}

fn list_files(path: &str, sort_descending: bool) -> Vec<String> {
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
    if sort_descending == true {
        out.reverse();
    }
    out
}

fn render_list(files: Vec<String>, config: &Config, mut page: u16) -> String {
    let html_head = format!("<!DOCTYPE html>
<html>
  <head>
    <title>Video Server</title>
    <meta charset=\"utf-8\">
    <link rel=\"stylesheet\" type=\"text/css\" href=\"{}/static/style.css\">
  </head>
  <body>
", config.uri_path);

    let html_foot = "  </body>\n</html>\n";

    let mut html_body = String::new();
    write!(&mut html_body, "    <div class=\"main-div\">\n").ok();
    write!(&mut html_body, "      <div><h1>{}</div>\n", config.instance_name).ok();
    
    let max_pages = match files.len() % (config.video_count as usize) {
        0 => (files.len() / (config.video_count as usize)) as u16,
        _ => (files.len() / (config.video_count as usize) + 1) as u16,
    };
    let navigation = render_navigation(&config.uri_path, page, max_pages);
    write!(&mut html_body, "{}", &navigation).ok();

    if page == 0 { page += 1 }
    for file in files.iter().skip(((page - 1) * config.video_count) as usize).take(config.video_count as usize) {
        write!(&mut html_body,
            "      <div class=\"video-div\"><video class=\"video-el\" controls><source src=\"{}/video/{}\"/></video></div>\n",
            config.uri_path, file
        ).ok();
        if config.parse_timestamps == true {
            let date = parse_ts(file);
            write!(&mut html_body, "      <div class=\"link-div\"><a href=\"{}/video/{}\">{}</a></div>\n", config.uri_path, file, date).ok();
        } else {
            write!(&mut html_body, "      <div class=\"link-div\"><a href=\"{}/video/{}\">{}</a></div>\n", config.uri_path, file, file).ok();
        }
    }

    write!(&mut html_body, "    </div>\n").ok();

    format!("{}{}{}", html_head, html_body, html_foot)
}

fn render_navigation(path: &str, page: u16, max_pages: u16) -> String {
    let mut out = String::new();
    write!(&mut out, "      <div class=\"navigation-div\">").ok();
    if page == 1 {
        write!(&mut out, "<div class=\"navigation-c\">prev</div>").ok();
    } else {
        write!(&mut out, "<div class=\"navigation-c\"><a href=\"{}?page={}\">prev</a></div>", path, page - 1).ok();
    }
    write!(&mut out, "<div class=\"navigation-c\"> PAGE {} </div>", page).ok();
    if page < max_pages {
        write!(&mut out, "<div class=\"navigation-c\"><a href=\"{}?page={}\">next</a></div>", path, page + 1).ok();
    } else {
        write!(&mut out, "<div class=\"navigation-c\">next</div>").ok();
    }
    write!(&mut out, "</div>\n").ok();
    out
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

    let port = config.port;

    let config_filter = warp::any().map(move || config.clone());

    let static_dir = warp::path("static")
        .and(warp::fs::dir("./static"))
        .and(config_filter.clone())
        .map(|reply, c: Config| {
            if c.disable_static_cache == true {
                warp::reply::with_header(reply, "cache-control", "no-cache")
            } else {
                warp::reply::with_header(reply, "cache-control", "max-age=14400")
            }
        });

    let index = warp::get()
        .and(warp::path::end())
        .and(config_filter.clone())
        .and(warp::query::<HashMap<String, u16>>())
        .map(|c: Config, q: HashMap<String, u16>| {
            let files = list_files(&c.fs_path, c.sort_descending);
            let page = q.get("page").unwrap_or(&1);
            let content = render_list(files, &c, *page);
            warp::reply::html(content)
        });

    let routes = index
        .or(video_dir)
        .or(static_dir);

    warp::serve(routes)
        .run(([0, 0, 0, 0], port))
        .await;
}
