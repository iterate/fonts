use std::time::Duration;

use eyre::{eyre, Context, Result};

use reqwest::{header::ACCEPT, Client};
use tap::TapFallible;
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
                Element::LinkToCss(element) => {
                    let css_url = match get_parsed_url(&element, &page.base_url) {
                        Ok(parsed_url) => {
                            tracing::info!("Parsed url for css link.");
                            parsed_url
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to parse url. Continuing in loop...");
                            continue;
                        }
                    };

                    let css_content = match self.get_content_as_bytes(css_url.as_str()).await {
                        Ok(content) => {
                            tracing::info!("Got css content from url");
                            content
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to css content from url. Continuing in loop...");
                            continue;
                        }
                    };

                    let font_urls = match parse_css_doc(css_content) {
                        Ok(fonts_urls) => {
                            tracing::info!("Got font urls from css urls.");
                            fonts_urls
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to get font urls from css url. Continuing in loop...");
                            continue;
                        }
                    };

                    let font_urls = font_urls.iter().filter_map(|url| {
                        get_parsed_url(&url, &page.base_url)
                            .tap_err(|err| tracing::error!(error = ?err, "Could not parse url"))
                            .ok()
                    });
                    all_font_urls.extend(font_urls)
                }
                Element::LinkToFont(element) => {
                    let font_url = match get_parsed_url(&element, &page.base_url) {
                        Ok(parsed_url) => {
                            tracing::info!("Parsed url for font link.");
                            parsed_url
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to parse url. Continuing in loop.");
                            continue;
                        }
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

    pub async fn get_content_as_bytes(&self, url: &str) -> eyre::Result<Vec<u8>> {
        //println!("Fetching {}", font_url);

        let res = self
            .http_client
            .get(url)
            .send()
            .await
            .wrap_err("Unable to send response")?;

        if !res.status().is_success() {
            return Err(eyre!(
                "Not able to download content {}. Returned status: {}",
                url,
                res.status()
            ));
        }

        let content: Vec<u8> = res
            .bytes()
            .await
            .wrap_err("Could not get content as bytes")?
            .into_iter()
            .collect();

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
                    .wrap_err(err);
            }
            return Err(err).wrap_err(format!("Unable to parse font url correctly for {}", url));
        }
    };

    Ok(parsed_url)
}
