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
use std::io::Read;
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use url::form_urlencoded;

#[derive(Deserialize)]
struct Config {
    api_key: String,
    user_key: String,
}

fn load_cfg() -> Result<Config, Box<Error>> {
    let mut cfg_path = env::home_dir().ok_or("no home directory")?;
    cfg_path.push(".podog");

    let file = File::open(cfg_path)?;

    let cfg: Config = serde_json::from_reader(file)?;

    Ok(cfg)
}

fn push_msg(cfg: Config, msg: &str) -> Result<(), Box<Error>> {
    let query = vec![("token", cfg.api_key), ("user", cfg.user_key), ("message", String::from(msg))];

    let body = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(query.iter())
        .finish();

    let tls = NativeTlsClient::new()?;
    let connector = HttpsConnector::new(tls);
    let client = Client::with_connector(connector);
    let mut response = client.post("https://api.pushover.net/1/messages.json").body(&body[..]).send()?;

    Ok(())
}

/*
fn get_content(url: &str) -> hyper::Result<String> {
    let client = Client::new();
    let mut response = client.get(url).send()?;
    let mut buf = String::new();
    response.read_to_string(&mut buf)?;
    Ok(buf)
}
*/

fn main() {
    let cfg: Config = match load_cfg() {
        Ok(c) => c,
        Err(_) => panic!("Failed to load cfg"),
    };

    /*
    let buf = match get_content("http://httpbin.org/status/200") {
        Ok(r) => r,
        Err(e) => panic!("oh shit, {}", e),
    };

    println!("buf: {}", buf);
    */

    match push_msg(cfg, "this is a test") {
        Ok(_) => println!("pushed!"),
        Err(e) => panic!("failed to push, {}", e),
    };
}
