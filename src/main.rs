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

fn push_msg(cfg: Config, msg: &str) -> Result<String, PodogError> {
    let query = vec![("token", cfg.api_key), ("user", cfg.user_key), ("message", String::from(msg))];

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

fn main() {
    let matches = App::new("podog")
        .version("0.1.0")
        .author("Andrew Rader <ardr@outlook.com>")
        .about("CLI for Pushover notifications")
        .arg(Arg::with_name("message")
             .index(1)
             .required(true))
        .get_matches();

    let cfg: Config = match load_cfg() {
        Ok(c) => c,
        Err(_) => panic!("Failed to load cfg"),
    };

    match push_msg(cfg, matches.value_of("message").unwrap()) {
        Ok(s) => println!("pushed!, request: {}", s),
        Err(e) => panic!("failed to push, {:?}", e),
    };
}
