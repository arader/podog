extern crate clap;
extern crate hyper;
extern crate hyper_native_tls;
extern crate serde;
extern crate serde_json;
extern crate url;

#[macro_use]
extern crate serde_derive;

use std::{env, thread, time};
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
struct PushResponse {
    status: u32,
    request: String,
    #[serde(default)]
    receipt: String,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Deserialize)]
struct ReceiptResponse {
    status: u32,
    acknowledged: u32,
    acknowledged_at: u32,
    acknowledged_by: String,
    acknowledged_by_device: String,
    last_delivered_at: u32,
    expired: u32,
    expires_at: u32,
    called_back: u32,
    called_back_at: u32,
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
    cfg: &Config,
    html: bool,
    title: &str,
    msg: &str,
    url: &str,
    url_title: &str,
    devices: &str,
    sound: &str,
    priority: &str,
    retry: &str,
    expires: &str) -> Result<PushResponse, PodogError> {
    let mut query = vec![("token", cfg.api_key.as_str()), ("user", cfg.user_key.as_str()), ("title", title), ("message", msg)];

    if html {
        query.push(("html", "1"));
    }

    if !url.is_empty() {
        query.push(("url", url));
    }

    if !url_title.is_empty() {
        query.push(("url_title", url_title));
    }

    if !devices.is_empty() {
        query.push(("device", devices));
    }

    if !sound.is_empty() {
        query.push(("sound", sound));
    }

    if !priority.is_empty() {
        query.push(("priority", priority));
    }

    if !retry.is_empty() {
        query.push(("retry", retry));
    }

    if !expires.is_empty() {
        query.push(("expire", expires));
    }

    let body = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(query.iter())
        .finish();

    let tls = NativeTlsClient::new()?;
    let connector = HttpsConnector::new(tls);
    let client = Client::with_connector(connector);

    let response = client.post("https://api.pushover.net/1/messages.json").body(&body[..]).send()?;

    let po_response: PushResponse = serde_json::from_reader(response)?;

    match po_response.status {
        1 => Ok(po_response),
        _ => Err(PodogError::Service(po_response.errors)),
    }
}

fn check_receipt(
    cfg: &Config,
    receipt: &str) -> Result<ReceiptResponse, PodogError> {
    let tls = NativeTlsClient::new()?;
    let connector = HttpsConnector::new(tls);
    let client = Client::with_connector(connector);

    let response = client.get(&(format!("https://api.pushover.net/1/receipts/{}.json?token={}", receipt, cfg.api_key))[..]).send()?;

    let po_response: ReceiptResponse = serde_json::from_reader(response)?;

    match po_response.status {
        1 => Ok(po_response),
        _ => Err(PodogError::Service(vec![String::from("failed to query receipt status")])),
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
             .required(true)
             .help("notification message"))
        .arg(Arg::with_name("title")
             .long("title")
             .short("t")
             .takes_value(true)
             .help("notification title"))
        .arg(Arg::with_name("html")
             .long("html")
             .short("m")
             .help("enables HTML markup in message"))
        .arg(Arg::with_name("url")
             .long("url")
             .short("u")
             .takes_value(true)
             .help("url to include with the notification"))
        .arg(Arg::with_name("url_title")
             .long("url-title")
             .takes_value(true)
             .help("title of the url"))
        .arg(Arg::with_name("devices")
             .long("devices")
             .short("d")
             .takes_value(true)
             .help("comma separated list of devices to push to"))
        .arg(Arg::with_name("sound")
             .long("sound")
             .short("s")
             .takes_value(true)
             .help("notification sound"))
        .arg(Arg::with_name("priority")
             .long("priority")
             .short("p")
             .takes_value(true)
             .possible_values(&["-2", "-1", "0", "1", "2"])
             .help("message priority, higher numbers are higher priority"))
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
        .arg(Arg::with_name("wait")
             .long("wait")
             .short("w")
             .help("wait until the notification is acknowledged"))
        .get_matches();

    let cfg: Config = match load_cfg() {
        Ok(c) => c,
        Err(_) => panic!("failed to load cfg"),
    };

    let response: PushResponse = match push_msg(&cfg,
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
        Ok(r) => r,
        Err(e) => panic!("failed to push, {:?}", e),
    };

    if matches.is_present("wait") {
        if response.receipt.is_empty() {
            panic!("request {} did not return a receipt to wait", response.request);
        }
        else {
            let mut failures = 0;

            loop {
                thread::sleep(time::Duration::from_secs(5));

                match check_receipt(&cfg, response.receipt.as_str()) {
                    Ok(result) => {
                        failures = 0;
                        if result.acknowledged == 1 {
                            break;
                        }
                        else if result.expired == 1 {
                            println!("notification expired");
                            break;
                        }
                    },
                    Err(_) => {
                        failures = failures + 1;
                        if failures == 5 {
                            panic!("failed to wait for receipt");
                            break;
                        }
                    },
                }
            }
        }
    }
}
