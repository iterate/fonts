use eyre::{eyre, Result};

use once_cell::sync::Lazy;
use regex::Regex;

// r"(@font-face \{\w*\})"
// static FONT_FACE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"@font-face \{.*\}").unwrap());
static FONT_FACE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"@font-face\{(?P<data>[\s\S]*?)\}").unwrap());
static URL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\((?P<data>[\S]*?)\)").unwrap());

pub fn parse_css_doc(text: &mut String) -> Result<Vec<String>> {
    // can use retain since variable s is mutable. just want to remove whitespace
    text.retain(|c| !c.is_whitespace());

    let matches: Vec<&str> = FONT_FACE_RE
        .captures_iter(&text.trim())
        .filter_map(|c| c.name("data"))
        .map(|c| -> &str { c.as_str() })
        .collect();

    if matches.is_empty() {
        return Err(eyre!("Could not find font-face attribute"));
    }

    // At this point, content has been extracted from @font-face{}
    // Maybe this should be extracted into its own method?
    let urls: Vec<String> = matches
        .iter()
        .flat_map(|s| s.split("url"))
        .filter_map(|url| -> Option<String> {
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
    fn get_urls_from_css_file() -> Result<()> {
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

    #[test]
    fn get_urls_from_inline_css() -> Result<()> {
        let inline_css_strings = vec![
                                    "\n      .tk-franklin-gothic-urw {\n        font-family: \"franklin-gothic-urw\", sans-serif;\n      }\n    ".to_owned(), 
                                    "\n      @font-face {\n        font-family: tk-franklin-gothic-urw-n4;\n        src: url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff2\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"opentype\");\n        font-weight: 400;\n        font-style: normal;\n        font-stretch: normal;\n        font-display: auto;\n      }\n    ".to_owned(), 
                                    "\n      body,\n      html {\n        height: 100%;\n        font-family: franklin-gothic-urw, sans-serif;\n        font-weight: 400;\n        font-size: 20px;\n        color: #333e48;\n        margin: 0;\n        box-sizing: border-box;\n      }\n      * {\n        box-sizing: inherit;\n        color: currentColor;\n      }\n      .title-wrapper p:first-of-type {\n        margin-top: 40px;\n        margin-bottom: 13px;\n      }\n      hr {\n        display: none;\n      }\n      p {\n        margin: 0 0 18px;\n      }\n    ".to_owned(), 
                                    "\n      [_nghost-xbj-3] {\n        flex-flow: column nowrap;\n        height: 100vh;\n        padding: 0 39px;\n        width: 100vw;\n      }\n      .top[_ngcontent-xbj-3],\n      [_nghost-xbj-3] {\n        display: flex;\n      }\n      .top[_ngcontent-xbj-3] {\n        height: 10vh;\n        min-height: 100px;\n        justify-content: space-between;\n        padding-top: 29px;\n        z-index: 2;\n      }\n      .top[_ngcontent-xbj-3] a[_ngcontent-xbj-3] {\n        text-decoration: none;\n      }\n      .top[_ngcontent-xbj-3] a[_ngcontent-xbj-3]:hover {\n        text-decoration: underline;\n      }\n      .middle[_ngcontent-xbj-3] {\n        height: 69vh;\n        display: flex;\n        align-items: center;\n      }\n      .bottom[_ngcontent-xbj-3] {\n        display: flex;\n        height: 21vh;\n        justify-content: flex-end;\n      }\n      .bottom[_ngcontent-xbj-3],\n      .middle[_ngcontent-xbj-3],\n      .top[_ngcontent-xbj-3] {\n        width: 100%;\n      }\n      .middle[_ngcontent-xbj-3] {\n        position: relative;\n      }\n      .left-arrow[_ngcontent-xbj-3],\n      .right-arrow[_ngcontent-xbj-3] {\n        position: absolute;\n        top: 0;\n        bottom: 0;\n        width: 50%;\n      }\n      .left-arrow[_ngcontent-xbj-3] {\n        left: 0;\n        cursor: url(/assets/left.png), w-resize;\n      }\n      .right-arrow[_ngcontent-xbj-3] {\n        right: 0;\n        cursor: url(/assets/right.png), e-resize;\n      }\n      .image-wrapper[_ngcontent-xbj-3] {\n        align-items: center;\n        display: flex;\n        justify-content: center;\n        margin: 0 auto;\n        height: 100%;\n        width: 80vw;\n      }\n      svg[_ngcontent-xbj-3] {\n        fill: #333e48;\n      }\n      .title-wrapper[_ngcontent-xbj-3] {\n        flex: 0 1 40%;\n        height: 21vh;\n        max-width: 500px;\n        min-width: 360px;\n        text-align: right;\n      }\n      p[_ngcontent-xbj-3] {\n        margin: 0;\n      }\n      .title-wrapper[_ngcontent-xbj-3] hr[_ngcontent-xbj-3] {\n        display: none;\n      }\n      .image[_ngcontent-xbj-3] {\n        background-size: contain;\n        background-repeat: no-repeat;\n        background-position: 50%;\n        background-color: #fff;\n        height: 69vh;\n        max-width: 800px;\n        width: 80vw;\n      }\n    ".to_owned(), 
                                    "\n      @font-face {\n        font-family: franklin-gothic-urw;\n        src: url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff2\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"opentype\");\n        font-weight: 400;\n        font-style: normal;\n        font-stretch: normal;\n        font-display: auto;\n      }\n    ".to_owned(), 
                                    "\n      a[_ngcontent-xbj-1] {\n        text-decoration: none;\n      }\n      .hover[_ngcontent-xbj-1] a[_ngcontent-xbj-1]:hover {\n        text-decoration: underline;\n      }\n    ".to_owned()
                                    ];

        let urls: Vec<String> = inline_css_strings
            .iter()
            .filter_map(|inline_css| parse_css_doc(&mut inline_css.to_owned()).ok())
            .flatten()
            .collect();

        let expected_results = vec![
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
            "https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3",
        ];

        assert_eq!(urls, expected_results);

        Ok(())
    }
}
