use crate::{crawler::http_crawler::HttpCrawler, font_parser::FontData};

use super::Result;

pub mod channel_message;
pub mod html_browser;
pub mod html_http;
pub mod page;
pub mod verifier;

#[derive(Debug, Clone)]
pub struct Page {
    pub base_url: String,
    pub page_content: String,
}

impl Page {
    pub fn new(base_url: String, page_content: String) -> Page {
        Page {
            base_url,
            page_content,
        }
    }
}

#[derive(Debug)]
pub struct SiteData {
    pub url: String,
    pub fonts: Vec<FontData>,
}

impl SiteData {
    pub async fn from_page(crawler: &HttpCrawler, page: &Page) -> Result<SiteData> {
        // Get page content to find links to follow
        // let page_content = crawler.get_page_content(base_url).await?;

        let font_urls = crawler.get_font_urls_from_page(page).await?;

        let mut font_contents: Vec<Vec<u8>> = vec![];

        for font_url in &font_urls {
            let font_content = match crawler.get_content_as_bytes(font_url.as_str()).await {
                Ok(font_content) => font_content,
                Err(err) => {
                    tracing::error!(error = ?err, "Failed to get font content. Continuing...");
                    continue;
                }
            };
            font_contents.push(font_content);
        }

        //println!("Found {} font urls", font_urls.len());

        let all_font_data: Vec<FontData> = font_contents
            .iter()
            .filter_map(|font_content| match FontData::from_bytes(font_content) {
                Ok(font_content) => Some(font_content),
                Err(err) => {
                    tracing::error!(error = ?err, "Failed to parse font data. Continuing...");
                    None
                }
            })
            .collect();

        Ok(SiteData {
            url: page.base_url.to_owned(),
            fonts: all_font_data,
        })
    }
}
