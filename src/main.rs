extern crate unqlite;

use gmi::gemtext::{parse_gemtext, GemtextNode};
use gmi::request;
use gmi::url::{Path, Url};
use log::*;
use std::sync::{Arc, Mutex};
use unqlite::{Config, Cursor, UnQLite, KV};

type Db = Arc<Mutex<UnQLite>>;

#[tokio::main]
async fn main() {
    let db = Arc::new(Mutex::new(UnQLite::create("test.db")));
    // let db = Arc::new(Mutex::new(HashMap::new()));
    stderrlog::new().verbosity(2).quiet(false).init().unwrap();

    // get command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args[1].clone();
    let u = args[2].clone();

    // match command
    match command.as_str() {
        "scan" => {
            let url = Url::try_from(u.as_str()).expect("url couldn't be parsed");
            let page = download(url.clone())
                .await
                .expect("couldn't download gemini url");

            let links = get_links(url, page.to_string()).await;
            println!("{:?}", links);

            download_links(links, db).await;
        }
        "read" => {
            let root = args[3].clone();
            let url = Url::try_from(root.as_str()).expect("url couldn't be parsed");
            // use fs::read_to_string to open gemlog.gmi
            let contents =
                std::fs::read_to_string(u.clone()).expect("Something went wrong reading the file");
            let links = get_links(url, contents).await;
            download_links(links, db).await;
        }
        _ => {
            println!("Command not found");
        }
    }
}

async fn get_links(url: Url, page: String) -> Vec<String> {
    let mut links = vec![];
    let gmi = parse_gemtext(&page.to_string());
    for node in gmi {
        let mut url = url.clone();
        match node {
            GemtextNode::Link(link, _) => {
                // only download .gmi links
                // if !link.contains(".gmi") {
                //     continue;
                // }

                // link is just "dir/filename.gmi". Merge it with
                // gemini://hostname/phlog/ to get the full path
                match url.path {
                    Some(path) => {
                        url.path = Some(path.merge_path(&Path::try_from(link.as_str()).unwrap()));
                    }
                    None => {
                        url.path = Some(Path::try_from(link.as_str()).unwrap());
                    }
                }
                info!("found path {}", url.clone().to_string());
                links.push(url.to_string());
            }
            _ => (),
        }
    }
    return links;
}

async fn download_links(links: Vec<String>, db: Db) {
    for link in links {
        let db = Arc::clone(&db);

        // check if db already contains the page
        let db_readonly = db.lock().unwrap();
        if db_readonly.kv_contains(link.clone()) {
            warn!("db already contains {}", link);
            continue;
        }
        drop(db_readonly);

        tokio::spawn(async move {
            let page = download(
                Url::try_from(link.as_str()).expect("couldn't convert a String into a URL"),
            )
            .await;
            match page {
                // match page
                Ok(s) => {
                    // lock the Arc and Mutex
                    let db = db.lock().unwrap();
                    db.kv_store(link, s).unwrap();
                }
                Err(_) => (),
            }
        });
    }
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
                std::borrow::Cow::Owned(s) => s,
            };
            Ok(s)
        }
        Err(err) => {
            error!("couldn't download {}", url);
            Err(Box::new(err))
        }
    }
}
