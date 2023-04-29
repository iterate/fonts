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

#[cfg(test)]
mod tests {

    use eyre::Result;

    use crate::crawler::html_parser::{get_elements_from_page, Element};

    use super::BrowserCrawler;

    #[test]
    #[cfg_attr(not(feature = "network"), ignore)]
    fn get_elements_with_browser_crawler() -> Result<()> {
        // This test is mainly used for developing.
        // But, should probably find (or, create) a stable site that outputs something I could use for this test
        let browser = BrowserCrawler::new()?;

        let content = browser.get_page_content("https://ense.no")?;

        Ok(())
    }
}
