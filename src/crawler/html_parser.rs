use std::collections::HashSet;

use maplit::hashset;
use once_cell::sync::Lazy;
use scraper::Html;

static INCLUDE_ELEMENTS: Lazy<HashSet<&str>> = Lazy::new(|| hashset!["script", "style", "link"]);

#[derive(Debug, PartialEq)]
pub enum Link {
    Css(String),
    Font(String),
}

pub fn parse_html_doc(text: &String) -> Vec<Link> {
    let document = Html::parse_document(&text);

    // Find links to follow.
    // Either links to stylesheet or links to fonts
    // might be able to do this smarter. if one assumes that relevant information will be in head tag, i won't have to traverse the whole tree
    // so, maybe, traverse the tree, find element tag that is head, than only traverse the children
    let links: Vec<Link> = document
        .tree
        .nodes()
        .filter_map(|node| match node.value() {
            scraper::Node::Element(element) => {
                let tag_name = element.name();

                if INCLUDE_ELEMENTS.contains(tag_name) {
                    let mut attrs = element.attrs();

                    while let Some(attr) = attrs.next() {
                        match attr {
                            ("rel", "stylesheet") => {
                                let href = element.attr("href");

                                if let Some(href) = href {
                                    return Some(Link::Css(href.to_owned()));
                                }
                            }
                            ("type", value) => {
                                if value.starts_with("font") {
                                    let href = element.attr("href");
                                    if let Some(href) = href {
                                        return Some(Link::Font(href.to_owned()));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                None
            }
            _ => None,
        })
        .collect();

    links
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eyre::Result;

    use crate::crawler::html_parser::{parse_html_doc, Link};

    #[test]
    fn get_links_from_html() -> Result<()> {
        let html_file =
            fs::read_to_string("test_files/test_iterateno.html").expect("Could not load html file");

        let links = parse_html_doc(&html_file);
        let expected_results = vec![Link::Css("https://uploads-ssl.webflow.com/5ea18b09bf3bfd55814199f9/css/iterate-104ab8-23d141065ef1b8634c6a653a.webflow.f3ca629db.css".to_owned())];

        assert_eq!(links, expected_results);

        let html_file =
            fs::read_to_string("test_files/test_nrkno.html").expect("Could not load html file");

        let links = parse_html_doc(&html_file);
        let expected_results = vec![Link::Font("https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2".to_owned()), 
                                    Link::Css("https://static.nrk.no/publisering/kurator-visning/assets/index-4167d179.css".to_owned()), 
                                    Link::Css("https://static.nrk.no/dh/module/nrkno-eksperimenter/assets/front-module.5c672c95.css".to_owned()), 
                                    Link::Css("https://static.nrk.no/dh/module/nrkno-eksperimenter/assets/front-module.5c309e74.css".to_owned()), 
                                    Link::Css("https://static.nrk.no/dh/module/langlesing/static//langlesingEtasje-33739c845c1bc1af2a00.css".to_owned()), 
                                    Link::Css("https://static.nrk.no/nrkno/serum/2.0.484/singelton/bottommenu/bottommenu.css".to_owned())];

        assert_eq!(links, expected_results);

        Ok(())
    }
}
