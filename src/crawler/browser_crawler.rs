use headless_chrome::Browser;

use eyre::{eyre, Result};

pub struct BrowserCrawler {
    client: Browser,
}

impl BrowserCrawler {
    pub fn new() -> Result<Self> {
        let client = Browser::default().map_err(|err| eyre!(err))?;
        Ok(BrowserCrawler { client })
    }
}
