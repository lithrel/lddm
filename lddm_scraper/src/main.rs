extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate regex;
extern crate select;
extern crate slug;
extern crate rustc_serialize;
extern crate chrono;

use std::env;
use std::error::Error;
use std::io::prelude::*;
use std::process;

use hyper::Client;
use hyper::Url;
use hyper::header::Connection;

use std::fs::File;
use std::path::Path;
use slug::slugify;

use select::document::Document;
use select::predicate::{Attr, Name};

use regex::{Regex, Captures};
use std::str::FromStr;
use std::collections::BTreeMap;
use rustc_serialize::json::{self, Json, ToJson};
use chrono::prelude::*;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() <= 1 {
        println!("\nUrl missing, abort.\n");
        process::exit(1);
    }

    // fetch url content
    println!("Loading content...");
    let url = Url::parse(&args[1].clone()).unwrap();
    let content = get_url_content(&url);
    println!("Content loaded.");

    // parse content
    parse_content(content, url.as_ref());

    //println!("content: {}", content);
}


fn parse_content(content: String, url: &str) -> () {
    let document = Document::from(content.as_ref());

    // Id
    let title_id = document.find(Name("h1")).first().unwrap();
    let title_id_text = title_id.text();
    let id_regex = Regex::new(r"#([0-9]{1,3})").unwrap();
    let cap = id_regex.captures(&title_id_text).unwrap();
    let episode_id: usize = cap[1].parse().unwrap();
    println!("id : {}", episode_id);

    // Title
    let title_node = document.find(Attr("class", "show-subtitle")).first().unwrap();
    let episode_title = title_node.text();
    println!("title : {}", episode_title);

    // PubDate
    let time_node = document.find(Name("time")).first().unwrap();
    let time_text = time_node.attr("datetime").unwrap();
    #[derive(Debug)]
    let datetime = DateTime::parse_from_str(&time_text, "%Y-%m-%dT%H:%M:%S %z").unwrap();
    let episode_date = datetime.format("%d/%m/%Y").to_string();

    // Track list
    let spoiler_node = document.find(Attr("class", "spoiler")).first().unwrap();
    #[derive(Debug)]
    let mut spoiler_text = str::replace(&spoiler_node.text(), "\n\n", "\n");
    spoiler_text = str::replace(&spoiler_text, "\n\n", "\n"); // @todo : erk
    //println!("{}", spoiler_text);

    #[derive(Debug)]
    let split = spoiler_text.trim().split("\n");
    // println!("{:?}", split);

    // Url
    // Duration
    // Tags

    let mut track_count = 0;
    for (index, line) in split.enumerate() {
        let curr_line = line.trim();
        if curr_line.is_empty() {
            continue;
        }
        // println!("\nline : {}", line);
        track_count += 1;
        let mut track = parse_line(curr_line);

        track.id = episode_id.to_string();
        track.id.push_str("-");
        track.id.push_str(&track_count.to_string());
        track.episode_id = episode_id;
        track.num_played = track_count;

        //println!("{:#?}", track);
        println!("{:}", json::as_pretty_json(&track.to_json()).indent(4));
    }

    let episode = Episode {
        id: episode_id,
        title: String::from(episode_title),
        date: String::from(episode_date),
        url: String::from(url),
        duration: String::from("PT"),
        tags: String::from(""),
        guest: String::from(""),
        track_count: track_count
    };
    println!("{:}", json::as_pretty_json(&episode).indent(4));
    // println!("{}", spoiler_text);
}

fn get_url_content(url: &Url) -> String {
    let filename = slugify(url.to_string());
    let cache_path = "cache/".to_string() + &filename.to_string() + ".html";

    // Check & write cache if not exists
    let path = Path::new(&cache_path);
    if !path.exists() {
        write_url_content_to_cache(&url, &path);
    }

    get_cache_content(&cache_path)
}

fn get_cache_content(cache_path: &String) -> String {
    let mut file = match File::open(&cache_path) {
        Err(e) => {
            panic!("Couldn't open file {} : {}",
                   cache_path.to_string(),
                   e.description())
        }
        Ok(file) => file,
    };

    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(e) => {
            panic!("Couldn't read file {} : {}",
                   cache_path.to_string(),
                   e.description())
        }
        Ok(_) => true,
    };
    s
}

fn write_url_content_to_cache(url: &Url, cache_path: &Path) -> () {
    println!("Creating cache file ...");
    let mut file = match File::create(&cache_path) {
        Err(e) => {
            panic!("Couldn't create {} : {}",
                   cache_path.display(),
                   e.description())
        }
        Ok(file) => file,
    };

    println!("Fetching url {} ...", url.to_string());
    let client = Client::new();
    let mut res = client.get(&url.to_string())
        .header(Connection::close())
        .send()
        .unwrap();

    println!("Caching content ...");
    let mut buffer = Vec::new();
    match res.read_to_end(&mut buffer) {
        Err(e) => panic!("Couldn't read client : {}", e.description()),
        Ok(_) => true,
    };
    match file.write_all(&buffer) {
        Err(e) => {
            panic!("Couldn't buffer to file {} : {}",
                   cache_path.display(),
                   e.description())
        }
        Ok(_) => true,
    };
    println!("Content cached in {}", cache_path.display());
}

fn parse_line(line: &str) -> Track {
    let curr_line = Line { line: String::from(line) };

    let track_type = curr_line.parse_type();
    #[derive(Debug)]
    let captures = curr_line.parse(&track_type);
    #[derive(Debug)]
    let matches = match captures {
        Some(cap) => cap,
        None => {
            println!("line : {}", curr_line.line);
            panic!("Capture failed for type [{}]", track_type)
        }
    };
    // println!("{:#?}", matches);

    let authors_string = str::replace(
        matches.name("authors").map_or("<none>", |m| m.as_str()), " et ", ", "
    );

    Track {
        title: String::from(matches.name("title").map_or("<none>", |m| m.as_str())),
        game_title: String::from(matches.name("game_title").map_or("<none>", |m| m.as_str())),
        author: String::from(authors_string),
        author_link: String::from(""), // @todo
        num_track: usize::from_str(matches.name("track_number").map_or("0", |m| m.as_str()))
            .unwrap(),
        track_type: track_type,
        ..Default::default()
    }
}

#[derive(Debug)]
struct Line {
    line: String,
}

impl Line {
    fn parse_type(&self) -> String {
        let actu_regex = Regex::new(r"Actu [0-9]").unwrap();
        let interlude_regex = Regex::new(r"L'(?:interlude|invité)").unwrap();
        let reprise_regex = Regex::new(r"^Reprise").unwrap();

        if actu_regex.is_match(&self.line) {
            return String::from("actu");
        }
        if interlude_regex.is_match(&self.line) {
            return String::from("interlude");
        }
        if reprise_regex.is_match(&self.line) {
            return String::from("reprise");
        }
        String::from("tracklist")
    }

    fn parse(&self, track_type: &String) -> Option<Captures> {
        #[derive(Debug)]
        let regexp = match track_type.as_ref() {
            "tracklist" => {
                "^(?P<track_number>[0-9]{1,3})\\s?-\\s?\
                        (?P<game_title>[^-]*)\\s?(\\-|,){1}\\s?\
                        (?P<title>[^,]*)?\\s?(\\-|,)?\\s?\
                        (?P<authors>.*)"
            }
            "actu" => {
                "^(?P<track_number>[0-9]{1,3})?[\\s]?-?[\\s]?Actu\\s(?P<actu_number>.*)\\s?(?:-|:)\\s?\
                       (?P<game_title>.*)[\\s]?-[\\s]?(?P<title>.*),[\\s]?(?P<authors>.*)"
            }
            "interlude" => "^L'(?:interlude|invité)\\s.*?\\s?:\\s?(?P<guest>.*)\\s(?:v|n)ous propose\\s(?P<interlude>.*)",
            "reprise" => {
                "^Reprise du mois\\s:\\s(?P<game_title>.*)\\s?-?\\s?(?P<title>.*)\\s?\
                          (par|sur|,)\\s(?P<authors>.*)"
            }
            _ => "^(?P<all>.*)$",
        };
        let line_regex = Regex::new(&regexp).unwrap();
        line_regex.captures(&self.line)
    }
}

#[derive(Debug, Default, RustcEncodable)]
struct Episode {
    id: usize,
    title: String,
    date: String,
    url: String,
    duration: String,
    tags: String,
    guest: String,
    track_count: usize,
}

#[derive(Debug, Default, RustcEncodable)]
struct Track {
    id: String,
    episode_id: usize,
    num_track: usize,
    num_played: usize,
    title: String,
    game_title: String,
    author: String,
    author_link: String,
    track_type: String,
}

impl ToJson for Track {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        // All standard types implement `to_json()`, so use it
        d.insert("id".to_string(), self.id.to_json());
        d.insert("episode_id".to_string(), self.episode_id.to_json());
        d.insert("num_track".to_string(), self.num_track.to_json());
        d.insert("num_played".to_string(), self.num_played.to_json());
        d.insert("title".to_string(), self.title.to_json());
        d.insert("game_title".to_string(), self.game_title.to_json());
        d.insert("author".to_string(), self.author.to_json());
        d.insert("author_link".to_string(), self.author_link.to_json());
        d.insert("type".to_string(), self.track_type.to_json());
        Json::Object(d)
    }
}