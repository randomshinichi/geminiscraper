use gmi::gemtext::{parse_gemtext, GemtextNode};
use gmi::request;
use gmi::url::{Path, Url};
use log::*;
use std::sync::{Arc, Mutex};
use std::thread;
use unqlite::{Config, Cursor, UnQLite, KV};

type Db = Arc<Mutex<UnQLite>>;

pub fn get_links(url: Url, page: &str) -> Vec<String> {
    let mut links = vec![];
    let gmi = parse_gemtext(page);
    for node in gmi {
        let mut url = url.clone();
        match node {
            GemtextNode::Link(link, _) => {
                // if link starts with gopher:// or http, don't include it
                if link.starts_with("gopher://") || link.starts_with("http") {
                    continue;
                }

                // if link starts with gemini:// replace url.path entirely
                if link.starts_with("gemini://") {
                    url = Url::try_from(link.as_str()).expect("url couldn't be parsed");
                } else {
                    // link is just "dir/filename.gmi". Merge it with
                    // gemini://hostname/phlog/ to get the full path
                    match url.path {
                        Some(path) => {
                            url.path =
                                Some(path.merge_path(&Path::try_from(link.as_str()).unwrap()));
                        }
                        None => {
                            url.path = Some(Path::try_from(link.as_str()).unwrap());
                        }
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

pub fn download_links(links: Vec<String>, db: Db) {
    for link in links {
        let db = Arc::clone(&db);

        // check if db already contains the page
        let db_readonly = db.lock().unwrap();
        if db_readonly.kv_contains(link.clone()) {
            warn!("db already contains {}", link);
            continue;
        }
        drop(db_readonly);

        thread::spawn(move || {
            let page = download(
                Url::try_from(link.as_str()).expect("couldn't convert a String into a URL"),
            );
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

pub fn download(url: Url) -> Result<String, Box<dyn std::error::Error>> {
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

// test get_links
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_links() {
        let url = Url::try_from("gemini://gemini.circumlunar.space/~solderpunk/gemlog/")
            .expect("url couldn't be parsed");

        let page =
            std::fs::read_to_string("test1.gmi").expect("Something went wrong reading the file");
        let links = get_links(url, &page.as_str());
        let expected_links = [
            "gemini://gemini.circumlunar.space/~solderpunk/gemlog/../old-gemlogs.gmi",
            "gemini://gemini.circumlunar.space/~solderpunk/gemlog/atom.xml",
            "gemini://gemini.circumlunar.space/~solderpunk/gemlog/franken-peugeot-updates.gmi",
            "gemini://gemini.circumlunar.space/~solderpunk/gemlog/gemini-mailing-list-down.gmi",
            "gemini://gemini.circumlunar.space/~solderpunk/gemlog/green-days-in-brunei.gmi",
        ];
        assert_eq!(links, expected_links);

        let url =
            Url::try_from("gemini://gemini.circumlunar.space/").expect("url couldn't be parsed");
        let page = std::fs::read_to_string("gemini_circumlunar_space.gmi")
            .expect("Something went wrong reading the file");
        let links = get_links(url, &page.as_str());
        let expected_links = [
            "gemini://gemini.circumlunar.space/news/",
            "gemini://gemini.circumlunar.space/docs/",
            "gemini://gemini.circumlunar.space/software/",
            "gemini://gemini.circumlunar.space/servers/",
            "gemini://gemini.conman.org/test/torture/",
            "gemini://geminispace.info/",
            "gemini://gemini.circumlunar.space/capcom/",
            "gemini://rawtext.club:1965/~sloum/spacewalk.gmi",
            "gemini://calcuode.com/gmisub-aggregate.gmi",
            "gemini://caracolito.mooo.com/deriva/",
            "gemini://gempaper.strangled.net/mirrorlist/",
            "gemini://gemini.circumlunar.space/users/",
        ];
        assert_eq!(links, expected_links);
    }
}
