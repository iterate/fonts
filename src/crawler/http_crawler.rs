use std::time::Duration;

use eyre::{eyre, Result};

use reqwest::{header::ACCEPT, Client};
use url::{ParseError, Url};

use crate::{
    crawler::{
        css_parser::parse_css_doc,
        html_parser::{get_elements_from_page, Element},
    },
    tasks::Page,
    CustomError,
};

#[derive(Debug)]
pub struct HttpCrawler {
    http_client: Client,
}

impl HttpCrawler {
    pub fn new() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(6))
            .gzip(true)
            .brotli(true)
            .build()?;
        Ok(HttpCrawler { http_client })
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_page_content(&self, base_url: &str) -> Result<String> {
        self.http_client
            .get(base_url)
            .header(ACCEPT, "text/html")
            .send()
            .await?
            .text()
            .await
            .map_err(|err| eyre!(err))
    }

    #[tracing::instrument(skip(self, page), fields(url=page.base_url))]
    pub async fn get_font_urls_from_page(&self, page: &Page) -> crate::Result<Vec<Url>> {
        let elements: Vec<Element> = get_elements_from_page(&page.page_content);

        if elements.is_empty() {
            return Err(CustomError::NoElementsFound(page.base_url.to_owned()));
        }

        // want to end up with urls that are possible to visit after this map
        let mut all_font_urls: Vec<Url> = vec![];

        for element in elements {
            match element {
                Element::CssLink(element) => {
                    let css_url = match get_parsed_url(&element, &page.base_url) {
                        Ok(parsed_url) => parsed_url,
                        Err(_) => continue,
                    };

                    let font_urls = match self.get_font_urls_from_css_url(css_url.as_str()).await {
                        Ok(fonts_urls) => fonts_urls,
                        Err(_) => {
                            // TODO: Handle error somehow?
                            continue;
                        }
                    };

                    let font_urls = font_urls.iter().filter_map(|url|
                            // could have used .ok(), but this probably be logged?
                                match get_parsed_url(&url, &page.base_url) {
                                Ok(parsed_url) => return Some(parsed_url),
                                Err(err) => {
                                    eprintln!("Could not parse url: {}", err);
                                    return None;
                                }
                            });
                    all_font_urls.extend(font_urls)
                }
                Element::FontLink(element) => {
                    let font_url = match get_parsed_url(&element, &page.base_url) {
                        Ok(parsed_url) => parsed_url,
                        Err(_) => continue,
                    };
                    all_font_urls.push(font_url);
                }
                _ => {}
            }
        }

        if all_font_urls.is_empty() {
            return Err(CustomError::NoFontUrlsFound(page.base_url.to_owned()));
        }

        Ok(all_font_urls)
    }

    pub async fn get_font_urls_from_css_url(&self, css_url: &str) -> eyre::Result<Vec<String>> {
        //println!("Visiting {}", css_url);
        let res = self.http_client.get(css_url).send().await?; //.bytes().await?;

        if !res.status().is_success() {
            eprintln!("Got status {} for {}", res.status(), css_url);

            return Err(eyre!("Not able to get response from site"));
        }

        let b = res.bytes().await?;

        // need to handle that content-encoding is not [gzip, brotli] (defined as features in reqwest)
        // should be enough to check if string text is utf-8 encodable
        let s = match std::str::from_utf8(&b) {
            Ok(s) => s,
            Err(err) => {
                return Err(eyre!(
                    "Not able to parse bytes to utf-8 string. Might be encoding issue. Err: {}",
                    err
                ))
            }
        };

        Ok(parse_css_doc(&mut s.to_owned())?)
    }

    pub async fn get_font_file_as_bytes(&self, font_url: &str) -> eyre::Result<Vec<u8>> {
        //println!("Fetching {}", font_url);

        let res = self.http_client.get(font_url).send().await?;

        if !res.status().is_success() {
            eprintln!("Got status {} for {}", res.status(), font_url);

            return Err(eyre!("Not able to download font {}", font_url));
        }

        let content: Vec<u8> = res.bytes().await?.into_iter().collect();

        Ok(content)
    }
}

fn get_parsed_url(url: &str, base_url: &str) -> Result<Url> {
    let maybe_not_base = Url::parse(&url);

    let parsed_url = match maybe_not_base {
        Ok(url) => url,
        Err(err) => {
            if err == ParseError::RelativeUrlWithoutBase {
                return Url::parse(base_url)
                    .and_then(|base| base.join(&url))
                    .map_err(|err| eyre!(err));
            }
            return Err(eyre!("Unable to parse font url correctly for {}", url));
        }
    };

    Ok(parsed_url)
}
