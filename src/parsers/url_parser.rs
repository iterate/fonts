use tap::TapFallible;
use url::{ParseError, Url};

use eyre::{Context, Result};

#[derive(Debug)]
pub enum FontUrl {
    Http(Url),
    Data(Url),
}

pub fn parse_to_font_urls(urls: Vec<String>, base_url: &str) -> Result<Vec<FontUrl>> {
    // Parse using Url to map to FontUrl enum
    let urls: Vec<FontUrl> = urls
        .into_iter()
        .filter_map(|url| {
            parse_to_url(&url, &base_url)
                .tap_err(|err| tracing::error!(error = ?err, "Unable to parse url: {url}"))
                .ok()
        })
        .filter_map(|parsed| match parsed.scheme() {
            "http" | "https" => Some(FontUrl::Http(parsed)),
            "data" => Some(FontUrl::Data(parsed)),
            _ => {
                tracing::error!("Unknown scheme: {}", parsed.scheme());
                None
            }
        })
        .collect();

    Ok(urls)
}

pub fn parse_to_url(url: &str, base_url: &str) -> Result<Url> {
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

#[cfg(test)]
mod tests {

    use std::fs;

    use eyre::Result;

    use crate::parsers::{css_parser::parse_css_doc, url_parser::FontUrl};

    use super::parse_to_font_urls;

    #[test]
    fn parse_base64_url() -> Result<()> {
        let css_file = fs::read("test_files/test_base64_url.css").expect("Could not load css file");

        let base_url = "http://test.no";
        let urls = parse_css_doc(css_file)?;

        let font_urls = parse_to_font_urls(urls, base_url)?;

        assert!(font_urls.len() == 1, "font urls length was not 1");

        let font_url = font_urls.first().expect("could not get font url item");

        if let FontUrl::Data(_) = font_url {
            // Just want to check that it parses to FontUrl::Data for now
            return Ok(());
        }

        println!("font url: {:?}", font_url);
        Err(eyre::eyre!("Did not parse to FontUrl::Data"))
    }
}
