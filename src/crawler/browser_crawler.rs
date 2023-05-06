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

    #[tracing::instrument(skip(self))]
    pub fn get_page_content(&self, base_url: &str) -> Result<String> {
        let tab = self.client.new_tab().map_err(|err| eyre!(err))?;

        // todo: how to ensure all the html is loaded?
        //       maybe just wait x number of seconds
        tab.navigate_to(base_url)
            .and_then(|tab| tab.wait_until_navigated())
            .and_then(|tab| tab.get_content())
            .map_err(|err| eyre!(err))
    }
}
