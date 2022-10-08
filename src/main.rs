extern crate unqlite;

use unqlite::{UnQLite};
use gmi::url::{Path, Url};
use gmi::request;
use gmi::gemtext::{parse_gemtext, GemtextNode};
use log::*;
use std::sync::{Arc,Mutex};
use std::collections::HashMap;

// type Db = Arc<Mutex<UnQLite>>;
type Db = Arc<Mutex<HashMap<String, String>>>;
#[tokio::main]
async fn main() {
    // let unqlite = Arc::new(Mutex::new(UnQLite::create("test.db")));
    let db = Arc::new(Mutex::new(HashMap::new()));
    stderrlog::new().verbosity(2).quiet(false).init().unwrap();

    // get command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args[1].clone();
    let url = args[2].clone();

    // match command
    match command.as_str() {
        "scan" => {
            let u = Url::try_from(url.as_str()).expect("url couldn't be parsed");
            let page = download(u.clone()).await.expect("couldn't download gemini url");
            let gmi = parse_gemtext(&page.clone());

            for node in gmi {
                match node {
                    GemtextNode::Link(link, descriptor) => parse_link_download(u.clone(), Path::from(link.as_str()), db.clone()).await,
                    _ => (),
                }
            };
        },
        "read" => {
            let root = args[3].clone();
            let u = Url::try_from(url.as_str()).expect("url couldn't be parsed");
            // use fs::read_to_string to open gemlog.gmi
            let contents = std::fs::read_to_string(url.clone()).expect("Something went wrong reading the file");
            let gmi = parse_gemtext(&contents.to_string());
            // iterate over gmi
            for node in gmi {
                match node {
                    GemtextNode::Link(link, descriptor) => parse_link_download(u.clone(), Path::from(link.as_str()), db.clone()).await,
                    _ => ()
                }
            };

        },
        _ => {
            println!("Command not found");
        }
    }
    println!("{:?}", db);

}

async fn parse_link_download(mut root: Url, link: Path, db: Db) {
    info!("found path {}", link);
    root.path = Some(link);
    tokio::spawn(async move {
        let page = download(root.clone()).await;
        match page {
            // match page
            Ok(s) => {
                // lock the Arc and Mutex
                let mut db = db.lock().unwrap();
                // db.kv_store(url, page).unwrap();
                db.insert(root.to_string(), s);
            },
            Err(_) => ()
        }
    });
}

async fn download(url: Url) -> Result<String, Box<dyn std::error::Error>> {
    // use gmi to get a page
    let page = request::make_request(&url);

    match page {
        Ok(page) => {
            let s = String::from_utf8_lossy(&page.data);
            // match s, if it is a String or a str
            let s = match s {
                std::borrow::Cow::Borrowed(s) => s.to_string(),
                std::borrow::Cow::Owned(s) => s
            };
            Ok(s)
        },
        Err(err) => {
            warn!("couldn't download {}", url);
            Err(Box::new(err))
        }
    }
}
