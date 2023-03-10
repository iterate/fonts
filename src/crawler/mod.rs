use async_trait::async_trait;
use eyre::Result;

mod browser_crawler;
mod css_parser;
mod html_parser;
pub mod http_crawler;

#[async_trait]
trait PageFetcher {
    async fn get_page_content(&self, base_url: &str) -> Result<String>;
}
