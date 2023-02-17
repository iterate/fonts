use eyre::{eyre, Result};

use once_cell::sync::Lazy;
use regex::Regex;

// r"(@font-face \{\w*\})"
// static FONT_FACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@font-face \{.*\}").unwrap());
static FONT_FACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@font-face\{(?P<data>[\s\S]*?)\}").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"url\((?P<data>[\S]*?)\)").unwrap());

pub fn parse_css_doc(text: &mut String) -> Result<Vec<String>> {
    // can use retain since variable s is mutable
    text.retain(|c| !c.is_whitespace());

    let matches: Vec<&str> = FONT_FACE_RE
        .captures_iter(&text.trim())
        .filter_map(|c| c.name("data"))
        .map(|c| -> &str { c.as_str() })
        .collect();

    if matches.is_empty() {
        return Err(eyre!("Could not find font-face attribute"));
    }

    let urls: Vec<String> = matches
        .iter()
        .flat_map(|s| {
            let content = s.split(";").to_owned(); // trims any white-space and split on css delimiter
            content
                .filter(|s| s.contains("src"))
                .flat_map(|s| s.split(",")) // src is required for font-face to work: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/src
        })
        .filter_map(|url: &str| -> Option<String> {
            URL_RE
                .captures(url.trim())
                .and_then(|cap| cap.name("data"))
                .map(|c| c.as_str().replace(&['\"', '\''], ""))
        })
        .collect();

    if urls.is_empty() {
        return Err(eyre!("Could not find url in font-face attribute"));
    }

    Ok(urls)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eyre::Result;

    use crate::crawler::css_parser::parse_css_doc;

    #[test]
    fn get_urls_from_css() -> Result<()> {
        let mut css_file =
            fs::read_to_string("test_files/test_mindjek.css").expect("Could not load css file");

        println!("{}", css_file);

        let urls = parse_css_doc(&mut css_file)?;

        let expected_results = vec![
            "fonts/fontawesome-webfont.eot?v=4.4.0",
            "fonts/fontawesome-webfont.eot?#iefix&v=4.4.0",
            "../fonts/fontawesome-webfont.woff2?v=4.4.0",
            "../fonts/fontawesome-webfont.woff?v=4.4.0",
            "../fonts/fontawesome-webfont.ttf?v=4.4.0",
            "../fonts/fontawesome-webfont.svg?v=4.4.0#fontawesomeregular",
        ];

        assert_eq!(urls, expected_results);

        let mut css_file =
            fs::read_to_string("test_files/test_nrk.css").expect("Could not load css file");

        let urls = parse_css_doc(&mut css_file)?;

        let expected_results = vec![
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
            "https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable_Italic.woff2",
        ];

        assert_eq!(urls, expected_results);

        Ok(())
    }
}
