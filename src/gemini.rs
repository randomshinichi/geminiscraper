use gmi::gemtext::{parse_gemtext, GemtextNode};
use gmi::protocol::Response;
use gmi::request;
use gmi::url::{Path, Url};
use log::*;
use std::sync::{Arc, Mutex};
use std::thread;
use unqlite::{Config, Cursor, UnQLite, KV};

pub type Db = Arc<Mutex<UnQLite>>;

pub fn get_links(url: Url, page: &str) -> Vec<String> {
    let mut links = vec![];
    let gmi = parse_gemtext(page);
    for node in gmi {
        let mut url = url.clone();
        match node {
            GemtextNode::Link(link, _) => {
                // some links are https://, gopher:// etc. Don't process those.
                if !check_link_is_inside_geminispace(link.clone()) {
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

pub fn download(url: Url) -> Result<(String, String), Box<dyn std::error::Error>> {
    // use gmi to get a page
    let page = request::make_request(&url);

    match page {
        Ok(page) => {
            if page.meta.contains("text/gemini") || page.meta.contains("text/plain") {
                let s = String::from_utf8_lossy(&page.data);
                // match s, if it is a String or a str
                let s = match s {
                    std::borrow::Cow::Borrowed(s) => s.to_string(),
                    std::borrow::Cow::Owned(s) => s,
                };
                Ok((s, page.meta))
            } else {
                // return an error with format!("{} did not have meta=text/gemini, probably binary, not returning",url)
                Err(format!("{} had meta='{}', not returning", url, page.meta).into())
            }
        }
        Err(err) => Err(Box::new(err)),
    }
}

pub fn check_link_could_be_gemtext(s: String) -> bool {
    if s.ends_with(".xml") || s.ends_with(".jpg") || s.ends_with(".mp4") {
        return false;
    }
    return true;
}

pub fn check_link_is_inside_geminispace(s: String) -> bool {
    if s.starts_with("gopher://") || s.starts_with("https://") || s.starts_with("http://") {
        return false;
    }
    return true;
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
