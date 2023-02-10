use std::{collections::HashSet, fs, time::Duration};

use eyre::{eyre, Result};
use maplit::hashset;
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use url::{ParseError, Url};

use scraper::Html;

// r"(@font-face \{\w*\})"
static INCLUDE_ELEMENTS: Lazy<HashSet<&str>> = Lazy::new(|| hashset!["script", "style", "link"]);
// static FONT_FACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@font-face \{.*\}").unwrap());
static FONT_FACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@font-face\{(?P<data>[\s\S]*?)\}").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"url\((?P<data>[\S]*?)\)").unwrap());

enum Link {
    Css(String),
    Font(String),
}

pub struct Crawler {
    http_client: Client,
}

impl Crawler {
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(6))
            .gzip(true)
            .brotli(true)
            .build()?;
        Ok(Crawler { http_client })
    }

    pub fn urls_from_file(&self, filepath: &str) -> Result<Vec<String>> {
        let content = fs::read_to_string(filepath)?;

        let str_arr: Vec<String> = content.split("\n").map(|s| s.to_owned()).collect();

        Ok(str_arr)
    }

    pub async fn scrape_urls_from_file(&self, filepath: &str) -> Result<()> {
        let urls = self.urls_from_file(filepath)?;

        println!("{:?}", urls);

        // todo: implement loop

        Ok(())
    }

    async fn scrape_page(&self, base_url: &str) -> Result<Vec<Link>> {
        println!("Scraping: {}", base_url);

        let res = self.http_client.get(base_url).send().await?.text().await?;

        let document = Html::parse_document(&res);

        println!("{:?}", document.tree.root().children().count());
        println!("{:?}", document.tree.nodes().count());

        // Find links to follow.
        // Either links to stylesheet or links to fonts
        // might be able to do this smarter. if one assumes that relevant information will be in head tag, i wont have to traverse the whole tree
        // so, maybe, traverse the tree, find element tag that is head, than only traverse the children
        let links: Vec<Link> = document
            .tree
            .nodes()
            .filter_map(|node| match node.value() {
                scraper::Node::Element(element) => {
                    let tag_name = element.name();

                    if INCLUDE_ELEMENTS.contains(tag_name) {
                        let mut attrs = element.attrs();

                        while let Some(attr) = attrs.next() {
                            match attr {
                                ("rel", "stylesheet") => {
                                    let href = element.attr("href");

                                    if let Some(href) = href {
                                        return Some(Link::Css(href.to_owned()));
                                    }
                                }
                                ("type", value) => {
                                    if value.starts_with("font") {
                                        let href = element.attr("href");
                                        if let Some(href) = href {
                                            return Some(Link::Font(href.to_owned()));
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    None
                }
                _ => None,
            })
            .collect();

        if links.is_empty() {
            return Err(eyre!(
                "No font link or stylesheet url found for {}",
                base_url
            ));
        }

        Ok(links)
    }

    pub async fn get_font_urls_from_page(&self, base_url: &str) -> Result<Vec<Url>> {
        // Get links to follow
        let links: Vec<Link> = self.scrape_page(base_url).await?;

        // want to end up with urls that are possible to visit after this map
        let mut all_font_urls: Vec<Url> = vec![];

        for link in links {
            match link {
                Link::Css(link) => {
                    let url = Url::parse(&link);

                    let css_url = match url {
                        Ok(url) => url,
                        Err(err) => {
                            if err == ParseError::RelativeUrlWithoutBase {
                                Url::parse(base_url)?.join(&link)?
                            } else {
                                continue;
                            }
                        }
                    };

                    let font_urls = match self.get_font_urls_from_css_url(css_url.as_str()).await {
                        Ok(fonts_urls) => fonts_urls,
                        Err(_) => {
                            // TODO: Handle error somehow?
                            continue;
                        }
                    };

                    let font_urls = font_urls.iter().filter_map(|url| {
                        let maybe_not_base = Url::parse(&url);

                        let css_url = match maybe_not_base {
                            Ok(url) => Some(url),
                            Err(err) => {
                                if err == ParseError::RelativeUrlWithoutBase {
                                    return Url::parse(base_url)
                                        .and_then(|base| base.join(&url))
                                        .ok();
                                }
                                eprintln!(
                                    "Unable to parse found font url correctly for {}",
                                    css_url
                                );
                                return None;
                            }
                        };
                        return css_url;
                    });
                    all_font_urls.extend(font_urls)
                }
                Link::Font(link) => {
                    let url = Url::parse(&link);

                    let font_url = match url {
                        Ok(url) => url,
                        Err(err) => {
                            if err == ParseError::RelativeUrlWithoutBase {
                                Url::parse(base_url)?.join(&link)?
                            } else {
                                continue;
                            }
                        }
                    };
                    all_font_urls.push(font_url);
                }
            }
        }

        Ok(all_font_urls)
    }

    pub async fn get_font_urls_from_css_url(&self, css_url: &str) -> Result<Vec<String>> {
        println!("Visiting {}", css_url);
        let res = self.http_client.get(css_url).send().await?; //.bytes().await?;

        if !res.status().is_success() {
            eprintln!("Got status {} for {}", res.status(), css_url);

            return Err(eyre!("Not able to get response from site"));
        }

        let mut s = res.text().await?;

        // todo: need to handle that content-encoding is not [gzip, brotli] (defined as features in reqwest)
        //       might be enough to check if string text is utf-encoded
        // content-encoding header should be standardized
        // let content_encoding = res
        //     .headers()
        //     .get("content-encoding")
        //     .and_then(|val| val.to_str().ok());

        // if let Some(content_encoding) = content_encoding {
        //     if content_encoding == "gzip" {
        //         let data = res.bytes().await?;
        //         let mut d = GzDecoder::new(&data[..]);
        //         d.read_to_string(&mut s)?;
        //     } else {
        //         eprintln!(
        //             "Cant parse content-encoding {} for {}",
        //             content_encoding, css_url
        //         );
        //         return Err(eyre!("Not able to parse css file from site"));
        //     }
        // } else {

        // }

        Ok(parse_css_file(&mut s)?)
    }
}

fn parse_css_file(text: &mut String) -> Result<Vec<String>> {
    // can use retain since variable s is mutable
    text.retain(|c| !c.is_whitespace());

    let matches: Vec<&str> = FONT_FACE_RE
        .captures_iter(&text.trim())
        .filter_map(|c| c.name("data"))
        .map(|c| -> &str { c.as_str() })
        .collect();

    if matches.is_empty() {
        return Err(eyre!("Could not find font-face attribute"));
    }

    let urls: Vec<String> = matches
        .iter()
        .flat_map(|s| {
            let content = s.split(";").to_owned(); // trim any white-space and split on css delimiter
            content
                .filter(|s| s.contains("src"))
                .flat_map(|s| s.split(",")) // src is required for font-face to work: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/src
        })
        .filter_map(|url: &str| -> Option<String> {
            URL_RE
                .captures(url.trim())
                .and_then(|cap| cap.name("data"))
                .map(|c| c.as_str().to_owned())
        })
        .collect();

    if urls.is_empty() {
        return Err(eyre!("Could not find url in font-face attribute"));
    }

    Ok(urls)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eyre::Result;

    use crate::crawler::parse_css_file;

    #[test]
    fn get_urls_from_css() -> Result<()> {
        let mut css_file =
            fs::read_to_string("test_files/test_mindjek.css").expect("Could not load css file");

        println!("{}", css_file);

        let urls = parse_css_file(&mut css_file)?;

        let expected_results = vec![
            "'fonts/fontawesome-webfont.eot?v=4.4.0'",
            "'fonts/fontawesome-webfont.eot?#iefix&v=4.4.0'",
            "'../fonts/fontawesome-webfont.woff2?v=4.4.0'",
            "'../fonts/fontawesome-webfont.woff?v=4.4.0'",
            "'../fonts/fontawesome-webfont.ttf?v=4.4.0'",
            "'../fonts/fontawesome-webfont.svg?v=4.4.0#fontawesomeregular'",
        ];

        assert_eq!(urls, expected_results);

        let mut css_file =
            fs::read_to_string("test_files/test_nrk.css").expect("Could not load css file");

        let urls = parse_css_file(&mut css_file)?;

        let expected_results = vec![
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
        ];

        assert_eq!(urls, expected_results);

        Ok(())
    }
}
