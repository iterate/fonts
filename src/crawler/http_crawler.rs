use std::time::Duration;

use eyre::{eyre, Context, Result};

use reqwest::{header::ACCEPT, Client};
use url::Url;

use crate::{
    parsers::{
        css_parser::parse_css_doc,
        html_parser::{get_elements_from_page, Element},
        url_parser::{parse_to_font_urls, parse_to_url, FontUrl},
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
                Element::LinkToCss(url) => {
                    let css_url = match parse_to_url(&url, &page.base_url) {
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

                    let font_urls = match parse_to_font_urls(font_urls, &page.base_url) {
                        Ok(font_urls) => {
                            tracing::info!("Parsed to font urls.");
                            font_urls
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to parse to font urls. Continuing in loop.");
                            continue;
                        }
                    };

                    // TODO: maybe rewrite entire function to output FontUrl
                    // But for now. only include font urls that are http scheme
                    let font_urls: Vec<Url> = font_urls
                        .iter()
                        .filter_map(|font_url| {
                            if let FontUrl::Http(url) = font_url {
                                return Some(url.to_owned());
                            };
                            None
                        })
                        .collect();

                    all_font_urls.extend(font_urls)
                }
                Element::LinkToFont(url) => {
                    let font_url = match parse_to_url(&url, &page.base_url) {
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
                Element::InlineCss(text_css) => {
                    let bytes_css = text_css.as_bytes().to_vec();

                    let font_urls = match parse_css_doc(bytes_css) {
                        Ok(fonts_urls) => {
                            tracing::info!("Got font urls from css urls.");
                            fonts_urls
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to get font urls from css url. Continuing in loop...");
                            continue;
                        }
                    };

                    let font_urls = match parse_to_font_urls(font_urls, &page.base_url) {
                        Ok(font_urls) => {
                            tracing::info!("Parsed to font urls.");
                            font_urls
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, "Failed to parse to font urls. Continuing in loop.");
                            continue;
                        }
                    };

                    // TODO: maybe rewrite entire function to output FontUrl
                    // But for now. only include font urls that are http scheme
                    let font_urls: Vec<Url> = font_urls
                        .iter()
                        .filter_map(|font_url| {
                            if let FontUrl::Http(url) = font_url {
                                return Some(url.to_owned());
                            };
                            None
                        })
                        .collect();

                    all_font_urls.extend(font_urls)
                }
            }
        }

        if all_font_urls.is_empty() {
            return Err(CustomError::NoFontUrlsFound(page.base_url.to_owned()));
        }

        Ok(all_font_urls)
    }

    pub async fn get_content_as_bytes(&self, url: &str) -> eyre::Result<Vec<u8>> {
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
