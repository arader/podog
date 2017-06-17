extern crate clap;
extern crate hyper;
extern crate hyper_native_tls;
extern crate serde;
extern crate serde_json;
extern crate url;

#[macro_use]
extern crate serde_derive;

use std::env;
use std::error::Error;
use std::fs::File;

use clap::{Arg, App};
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use url::form_urlencoded;

#[derive(Deserialize)]
struct Config {
    api_key: String,
    user_key: String,
}

#[derive(Deserialize)]
struct PoResponse {
    status: u32,
    request: String,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug)]
enum PodogError {
    Hyper(hyper::Error),
    Tls(hyper_native_tls::native_tls::Error),
    Parse(serde_json::Error),
    Service(Vec<String>),
}

impl From<hyper::Error> for PodogError {
    fn from(err: hyper::Error) -> PodogError {
        PodogError::Hyper(err)
    }
}

impl From<hyper_native_tls::native_tls::Error> for PodogError {
    fn from(err: hyper_native_tls::native_tls::Error) -> PodogError {
        PodogError::Tls(err)
    }
}

impl From<serde_json::Error> for PodogError {
    fn from(err: serde_json::Error) -> PodogError {
        PodogError::Parse(err)
    }
}

fn load_cfg() -> Result<Config, Box<Error>> {
    let mut cfg_path = env::home_dir().ok_or("no home directory")?;
    cfg_path.push(".podog");

    let file = File::open(cfg_path)?;

    let cfg: Config = serde_json::from_reader(file)?;

    Ok(cfg)
}

fn push_msg(
    cfg: Config,
    html: bool,
    title: &str,
    msg: &str,
    url: &str,
    url_title: &str,
    devices: &str,
    sound: &str,
    priority: &str,
    retry: &str,
    expires: &str) -> Result<String, PodogError> {
    let mut query = vec![("token", cfg.api_key), ("user", cfg.user_key), ("title", String::from(title)), ("message", String::from(msg))];

    if html {
        query.push(("html", String::from("1")));
    }

    if !url.is_empty() {
        query.push(("url", String::from(url)));
    }

    if !url_title.is_empty() {
        query.push(("url_title", String::from(url_title)));
    }

    if !devices.is_empty() {
        query.push(("device", String::from(devices)));
    }

    if !sound.is_empty() {
        query.push(("sound", String::from(sound)));
    }

    if !priority.is_empty() {
        query.push(("priority", String::from(priority)));
    }

    if !retry.is_empty() {
        query.push(("retry", String::from(retry)));
    }

    if !expires.is_empty() {
        query.push(("expire", String::from(expires)));
    }

    let body = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(query.iter())
        .finish();

    let tls = NativeTlsClient::new()?;
    let connector = HttpsConnector::new(tls);
    let client = Client::with_connector(connector);

    let response = client.post("https://api.pushover.net/1/messages.json").body(&body[..]).send()?;

    let po_response: PoResponse = serde_json::from_reader(response)?;

    match po_response.status {
        1 => Ok(po_response.request),
        _ => Err(PodogError::Service(po_response.errors)),
    }
}

fn retry_validator(val: String) -> Result<(), String> {
    match val.parse::<u32>() {
        Ok(v) => {
            if v >= 30 {
                Ok(())
            }
            else {
                Err(String::from("must be at least 30 seconds"))
            }
        },
        Err(_) => return Err(String::from("must be the number of seconds between retries")),
    }
}

fn expires_validator(val: String) -> Result<(), String> {
    match val.parse::<u32>() {
        Ok(v) => {
            if v <= 10800 {
                Ok(())
            }
            else {
                Err(String::from("must be less than 10800 seconds"))
            }
        },
        Err(_) => return Err(String::from("must be the number of seconds until retries are stopped")),
    }
}

fn main() {
    let matches = App::new("podog")
        .version("0.1.0")
        .author("Andrew Rader <ardr@outlook.com>")
        .about("CLI for Pushover notifications")
        .arg(Arg::with_name("message")
             .index(1)
             .required(true))
        .arg(Arg::with_name("title")
             .short("t")
             .long("title")
             .takes_value(true))
        .arg(Arg::with_name("html")
             .long("html"))
        .arg(Arg::with_name("url")
             .short("u")
             .long("url")
             .takes_value(true))
        .arg(Arg::with_name("url_title")
             .long("url-title")
             .takes_value(true))
        .arg(Arg::with_name("devices")
             .long("devices")
             .short("d")
             .takes_value(true))
        .arg(Arg::with_name("sound")
             .long("sound")
             .short("s")
             .takes_value(true))
        .arg(Arg::with_name("priority")
             .long("priority")
             .short("p")
             .takes_value(true)
             .possible_values(&["-2", "-1", "0", "1", "2"]))
        .arg(Arg::with_name("retry")
             .long("retry")
             .short("r")
             .takes_value(true)
             .value_name("seconds")
             .validator(retry_validator)
             .required_if("priority", "2")
             .help("number of seconds between retries, min: 30"))
        .arg(Arg::with_name("expires")
             .long("expires")
             .short("e")
             .takes_value(true)
             .value_name("seconds")
             .validator(expires_validator)
             .required_if("priority", "2")
             .help("number of seconds to keep retrying, max: 10800 (3 hours)"))
        .get_matches();

    let cfg: Config = match load_cfg() {
        Ok(c) => c,
        Err(_) => panic!("Failed to load cfg"),
    };

    match push_msg(cfg,
                   matches.is_present("html"),
                   matches.value_of("title").unwrap_or(""),
                   matches.value_of("message").unwrap(),
                   matches.value_of("url").unwrap_or(""),
                   matches.value_of("url_title").unwrap_or(""),
                   matches.value_of("devices").unwrap_or(""),
                   matches.value_of("sound").unwrap_or(""),
                   matches.value_of("priority").unwrap_or(""),
                   matches.value_of("retry").unwrap_or(""),
                   matches.value_of("expires").unwrap_or("")) {
        Ok(s) => println!("pushed!, request: {}", s),
        Err(e) => panic!("failed to push, {:?}", e),
    };
}
