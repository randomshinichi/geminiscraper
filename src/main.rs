mod gemini;
extern crate threadpool;
use crate::gemini::*;
use crossbeam_channel::unbounded;
use gmi::url::{Path, Url};
use log::*;
use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc, Mutex},
};
use threadpool::ThreadPool;
use unqlite::{Config, Cursor, UnQLite, KV};

fn main() {
    let num_threads = 3;
    let pool = ThreadPool::new(num_threads);
    let (sender, receiver) = unbounded::<String>();
    let db = Arc::new(Mutex::new(UnQLite::create("test.db")));

    for id in 0..num_threads {
        let sender = sender.clone();
        let receiver = receiver.clone();
        let db = Arc::clone(&db);
        pool.execute(move || {
            println!("Hello from worker {}", id);
            loop {
                let link = receiver.recv().unwrap();

                // check if db already contains the page
                // let db_readonly = db.lock().unwrap();
                // if db_readonly.kv_contains(link.clone()) {
                //     info!("{} already in db", link);
                //     continue;
                // }
                // drop(db_readonly);

                let url = Url::try_from(link.as_str())
                    .expect("couldn't convert link in channel to a Url");
                let page = download(url.clone());
                match page {
                    Ok(s) => {
                        // are there any links in this page? if so add them to the channel/queue
                        let links = std::panic::catch_unwind(|| get_links(url.clone(), s.as_str()));
                        match links {
                            Ok(links) => {
                                for link in links {
                                    sender.send(link).unwrap();
                                }
                                // lock the Arc and Mutex
                                let db = db.lock().unwrap();
                                db.kv_store(url.to_string(), s).unwrap();
                            }
                            Err(e) => error!("{} failed@get_links() parsing page: {:?}", url, e),
                        }
                    }
                    Err(e) => warn!("{} failed: {}", url, e),
                }
            }
        });
    }
    stderrlog::new().verbosity(2).quiet(false).init().unwrap();

    // get command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args[1].clone();
    let u = args[2].clone();

    // match command
    match command.as_str() {
        "scan" => {
            sender.send(u).unwrap();
        }
        "read" => {
            let root = args[3].clone();
            let url = gmi::url::Url::try_from(root.as_str()).expect("url couldn't be parsed");
            // use fs::read_to_string to open gemlog.gmi
            let contents =
                std::fs::read_to_string(u.clone()).expect("Something went wrong reading the file");
            let links = get_links(url, contents.as_str());
            for link in links {
                sender.send(link).unwrap();
            }
        }
        _ => {
            println!("Command not found");
        }
    }
    pool.join()
}
