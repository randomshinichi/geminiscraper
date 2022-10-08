extern crate unqlite;

use unqlite::{UnQLite,KV, Config, Cursor};
use gmi::url::{Path, Url};
use gmi::request;
use gmi::gemtext::{parse_gemtext, GemtextNode};
use log::*;
use std::sync::{Arc,Mutex};

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
            let page = download(url.clone()).await.expect("couldn't download gemini url");

            download_links_in_page(url, page.to_string(), db).await;
        },
        "read" => {
            let root = args[3].clone();
            let url = Url::try_from(root.as_str()).expect("url couldn't be parsed");
            // use fs::read_to_string to open gemlog.gmi
            let contents = std::fs::read_to_string(u.clone()).expect("Something went wrong reading the file");

            download_links_in_page(url, contents.to_string(), db).await;
        },
        _ => {
            println!("Command not found");
        }
    }
}

async fn download_links_in_page(url: Url, page: String, db: Db) {
    let gmi = parse_gemtext(&page.to_string());
    for node in gmi {
        let db = db.clone();
        let mut url = url.clone();
        match node {
            GemtextNode::Link(link, _) => {
                // only download .gmi links
                if !link.contains(".gmi") {
                    continue;
                }

                // link is just "filename.gmi". Merge it with
                // gemini://hostname/phlog/ to get the full path
                match url.path {
                    Some(path)  => {
                        url.path = Some(path.merge_path(&Path::try_from(link.as_str()).unwrap()));
                    },
                    None => {
                        url.path = Some(Path::try_from(link.as_str()).unwrap());
                    }
                }
                info!("found path {}", url.clone().to_string());

                // check if db already contains the page
                let db_readonly = db.lock().unwrap();
                if db_readonly.kv_contains(url.to_string()) {
                    warn!("db already contains {}", url.to_string());
                    continue
                }
                drop(db_readonly);

                tokio::spawn(async move {
                    let page = download(url.clone()).await;
                    match page {
                        // match page
                        Ok(s) => {
                            // lock the Arc and Mutex
                            let db = db.lock().unwrap();
                            db.kv_store(url.to_string(), s).unwrap();
                        },
                        Err(_) => ()
                    }
                });
            },
            _ => ()
        }
    };
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
            error!("couldn't download {}", url);
            Err(Box::new(err))
        }
    }
}
