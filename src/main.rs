

extern crate unqlite;

use unqlite::{UnQLite, Config, KV, Cursor};
use gmi::url::Url;
use gmi::request;
use gmi::gemtext::parse_gemtext;

#[tokio::main]
async fn main() {
    let unqlite = UnQLite::create("test.db");

    // get command line arguments
    let args: Vec<String> = std::env::args().collect();
    // get command
    let command = args[1].clone();
    // get key
    let url = args[2].clone();

    // match command
    match command.as_str() {
        "scan" => {
            let page = download(url.clone()).await.expect("couldn't download gemini url");
            unqlite.kv_store(url, page).unwrap();
        },
        _ => {
            println!("Command not found");
        }
    }

}

async fn download(url: String) -> Result<String, Box<dyn std::error::Error>> {
    // use gmi to get a page
    let url = Url::try_from(url.as_str())?;
    let page = request::make_request(&url)?;

    let s = String::from_utf8_lossy(&page.data);
    // match s, if it is a String or a str
    let s = match s {
        std::borrow::Cow::Borrowed(s) => s.to_string(),
        std::borrow::Cow::Owned(s) => s
    };
    println!("{}: {}", url, s);
    Ok(s)
}