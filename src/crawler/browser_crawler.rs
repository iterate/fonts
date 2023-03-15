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

    fn get_page_content(&self, base_url: &str) -> Result<String> {
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
        let browser = BrowserCrawler::new()?;

        let content = browser.get_page_content("https://iterate.no")?;

        let elements = get_elements_from_page(&content);
        let expected_results = vec![Element::CssLink("https://uploads-ssl.webflow.com/5ea18b09bf3bfd55814199f9/css/iterate-104ab8-23d141065ef1b8634c6a653a.webflow.f3ca629db.css".to_owned())];

        assert_eq!(elements, expected_results);

        let content = browser.get_page_content("https://ense.no")?;

        let elements = get_elements_from_page(&content);

        // is not a font link. need to implement inline-css thing to actually get font from ense.no
        let expected_results = vec![Element::CssLink(
            "main.5606dde6c1acfbce1170bda109e0b739.css".to_owned(),
        )];

        assert_eq!(elements, expected_results);

        Ok(())
    }
}
