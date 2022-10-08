mod gemini;
extern crate threadpool;

use crate::gemini::*;
use std::sync::{mpsc, Arc, Mutex};
use threadpool::ThreadPool;
use unqlite::UnQLite;

fn main() {
    let pool = ThreadPool::new(4);
    let (sender, receiver) = mpsc::channel();

    let db = Arc::new(Mutex::new(UnQLite::create("test.db")));
    stderrlog::new().verbosity(2).quiet(false).init().unwrap();

    // get command line arguments
    let args: Vec<String> = std::env::args().collect();
    let command = args[1].clone();
    let u = args[2].clone();

    // match command
    match command.as_str() {
        "scan" => {
            let url = gmi::url::Url::try_from(u.as_str()).expect("url couldn't be parsed");
            let page = download(url.clone()).expect("couldn't download gemini url");

            let links = get_links(url, &page);
            println!("{:?}", links);

            download_links(links, db);
        }
        "read" => {
            let root = args[3].clone();
            let url = gmi::url::Url::try_from(root.as_str()).expect("url couldn't be parsed");
            // use fs::read_to_string to open gemlog.gmi
            let contents =
                std::fs::read_to_string(u.clone()).expect("Something went wrong reading the file");
            let links = get_links(url, contents.as_str());
            download_links(links, db);
        }
        _ => {
            println!("Command not found");
        }
    }
}
