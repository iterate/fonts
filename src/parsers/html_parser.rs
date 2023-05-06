use std::collections::HashSet;

use maplit::hashset;
use once_cell::sync::Lazy;
use scraper::{Html, Selector};

static INCLUDE_ELEMENTS: Lazy<HashSet<&str>> = Lazy::new(|| hashset!["script", "style", "link"]);

// TODO: Add for base64 encoded font in url like in uxsignals.com
#[derive(Debug, PartialEq)]
pub enum Element {
    LinkToCss(String),
    LinkToFont(String),
    InlineCss(String),
}

pub fn get_elements_from_page(text: &String) -> Vec<Element> {
    let document = Html::parse_document(&text);

    // Find links to follow.
    // Either links to stylesheet or links to fonts
    // might be able to do this smarter. if one assumes that relevant information will be in head tag, i won't have to traverse the whole tree
    // so, maybe, traverse the tree, find element tag that is head, than only traverse the children

    // let head_selector = scraper::Selector::parse("head").unwrap(); // should work

    // let head = document.select(&head_selector).next();

    // println!("{:#?}", head);

    // Fetch inline css elements
    let text_css_selector = Selector::parse("style").expect("could not parse selector");
    let text_css: Vec<Element> = document
        .select(&text_css_selector)
        .into_iter()
        .map(|element| Element::InlineCss(element.inner_html()))
        .collect();

    // Fetch the other elements
    let mut elements: Vec<Element> = document
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
                                    return Some(Element::LinkToCss(href.to_owned()));
                                }
                            }
                            ("type", value) => {
                                if value.starts_with("font") {
                                    let href = element.attr("href");
                                    if let Some(href) = href {
                                        return Some(Element::LinkToFont(href.to_owned()));
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

    // Extend elements with text_css elements
    elements.extend(text_css);
    elements
}

#[cfg(test)]
mod tests {
    use std::fs;

    use eyre::Result;

    use crate::parsers::html_parser::{get_elements_from_page, Element};

    #[test]
    fn get_links_from_html() -> Result<()> {
        let html_file =
            fs::read_to_string("test_files/test_iterateno.html").expect("Could not load html file");

        let elements = get_elements_from_page(&html_file);
        let expected_results = vec![Element::LinkToCss("https://uploads-ssl.webflow.com/5ea18b09bf3bfd55814199f9/css/iterate-104ab8-23d141065ef1b8634c6a653a.webflow.f3ca629db.css".to_owned())];

        assert_eq!(elements, expected_results);

        let html_file =
            fs::read_to_string("test_files/test_nrkno.html").expect("Could not load html file");

        let elements = get_elements_from_page(&html_file);
        let expected_results = vec![Element::LinkToFont("https://static.nrk.no/nrk-sans/1.2.1/NRKSans_Variable.woff2".to_owned()), 
                                    Element::LinkToCss("https://static.nrk.no/publisering/kurator-visning/assets/index-4167d179.css".to_owned()),
                                    Element::LinkToCss("https://static.nrk.no/dh/module/nrkno-eksperimenter/assets/front-module.5c672c95.css".to_owned()), 
                                    Element::LinkToCss("https://static.nrk.no/dh/module/nrkno-eksperimenter/assets/front-module.5c309e74.css".to_owned()), 
                                    Element::LinkToCss("https://static.nrk.no/dh/module/langlesing/static//langlesingEtasje-33739c845c1bc1af2a00.css".to_owned()), 
                                    Element::LinkToCss("https://static.nrk.no/nrkno/serum/2.0.484/singelton/bottommenu/bottommenu.css".to_owned()), 
                                    Element::InlineCss("\n    .radio-multi-plug,.radio-multi-wrap{position:relative}.radio-multi-app{font-family:'NRK Sans Variable','LFT Etica',sans-serif;font-weight:400;text-align:center;overflow:hidden;background:#061629;color:#fff;border-bottom:1.5px solid #e9e9e9;border-radius:6px;box-shadow:0 1px 0 0 rgba(0,0,0,.04)}.radio-multi-main-title,.radio-multi-text{font-weight:700;font-size:15px;line-height:18px;text-align:left;font-style:normal}@media screen and (max-width:700px){.radio-multi-app{padding-bottom:.4rem}}.radio-multi-wrap-header{display:flex;flex-direction:row;justify-content:space-between;align-items:center;padding:15px 15px 10px}.radio-multi-main-title{margin:0}.radio-multi-home-button{text-decoration:none;position:relative;color:#fff}.radio-multi-dual-logo-wrapper:hover h3,.radio-multi-dual-logo-wrapper:hover p,.radio-multi-home-button:hover,.radio-multi-plug:hover{text-decoration:underline}.radio-multi-main-title-left{margin-right:1rem}@media screen and (max-width:480px){.radio-multi-main-title-right{display:none}}.radio-multi-scroll-wrap{padding:2px 0}.radio-multi-scroll{display:flex;margin:0 -.5rem;padding:3px 0 10px;-ms-overflow-style:none;height:15rem}.radio-multi-plug-squaredLogo,.radio-scroll-squaredLogo{height:13rem!important}.radio-multi-plug{flex:0 0 auto;display:flex;width:13rem;height:15rem;margin:0 .3rem;text-decoration:none;flex-direction:column;align-items:baseline}.radio-multi-spacer-elm{width:1rem;flex:0 0 auto}.radio-multi-dual-logo-wrapper,.radio-multi-image{width:13rem;height:13rem;border-radius:6px;margin-bottom:.5rem}.radio-multi-dual-logo-wrapper{background:#0a2343;display:flex;flex-direction:column;align-items:center;justify-content:center}.radio-multi-dual-logo-wrapper svg{margin-bottom:.5rem;margin-top:.5rem}.radio-multi-dual-logo-wrapper h3{margin:0;font-style:normal;font-weight:700;font-size:15px;line-height:18px;color:#fff}.radio-multi-dual-logo-wrapper p{margin:.5rem 1rem;font-style:normal;font-weight:400;font-size:13px;line-height:18px;color:rgba(255,255,255,.7)}.radio-multi-image{object-fit:cover}.radio-multi-image-squaredLogo{margin-bottom:0!important}.radio-multi-text{-webkit-font-smoothing:antialiased;margin:0;z-index:2;overflow:hidden;white-space:nowrap;text-overflow:ellipsis;width:100%}.radio-multi-button{background:0 0;display:none;position:absolute;top:50%;flex-direction:column;justify-content:center;cursor:pointer;border:0 solid;padding:1px 6px;filter:drop-shadow(0px 8px 20px rgba(0, 0, 0, .3))}@media screen and (min-width:750px){.radio-multi-button{display:flex}}.radio-multi-icon{width:40px;height:40px;background:#061629;border-radius:4px;display:flex;justify-content:center;text-align:center;align-items:center}.radio-multi-icon svg{height:25px;width:25px;stroke-width:.7px;color:#fff}.radio-multi-button:disabled{display:none}.radio-multi-button:disabled svg{color:#aaa}.radio-multi-button:focus{outline:0}.radio-multi-button:focus .icon{box-shadow:0 0 0 2px rgba(255,255,255,.75);outline:0}.radio-multi-button-left{left:0}.radio-multi-button-right{right:0}.radio-multi-direkte{position:absolute;left:10px;top:10px;padding:4px 8px;background:#f30707;box-shadow:0 8px 20px rgb(0 0 0 / 30%);border-radius:4px;font-style:normal;font-weight:600;font-size:12px;line-height:16px}\n  ".to_owned()), 
                                    Element::InlineCss("\n.nrk-bottommenu{display:none}\nhtml.no-header .nrk-bottommenu-footer {display: none;}\n.nrk-bottommenu-footer{padding:25px;background:#171717;color:#cbcbcb;text-align:center;font-size:14px;line-height:1.2}\n".to_owned())];

        assert_eq!(elements, expected_results);

        let html_file =
            fs::read_to_string("test_files/test_ense.html").expect("Could not load html file");

        let elements = get_elements_from_page(&html_file);
        let expected_results = vec![Element::LinkToCss("main.5606dde6c1acfbce1170bda109e0b739.css".to_owned()), 
                                    Element::InlineCss("\n      .tk-franklin-gothic-urw {\n        font-family: \"franklin-gothic-urw\", sans-serif;\n      }\n    ".to_owned()), 
                                    Element::InlineCss("\n      @font-face {\n        font-family: tk-franklin-gothic-urw-n4;\n        src: url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff2\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"opentype\");\n        font-weight: 400;\n        font-style: normal;\n        font-stretch: normal;\n        font-display: auto;\n      }\n    ".to_owned()), 
                                    Element::InlineCss("\n      body,\n      html {\n        height: 100%;\n        font-family: franklin-gothic-urw, sans-serif;\n        font-weight: 400;\n        font-size: 20px;\n        color: #333e48;\n        margin: 0;\n        box-sizing: border-box;\n      }\n      * {\n        box-sizing: inherit;\n        color: currentColor;\n      }\n      .title-wrapper p:first-of-type {\n        margin-top: 40px;\n        margin-bottom: 13px;\n      }\n      hr {\n        display: none;\n      }\n      p {\n        margin: 0 0 18px;\n      }\n    ".to_owned()), 
                                    Element::InlineCss("\n      [_nghost-xbj-3] {\n        flex-flow: column nowrap;\n        height: 100vh;\n        padding: 0 39px;\n        width: 100vw;\n      }\n      .top[_ngcontent-xbj-3],\n      [_nghost-xbj-3] {\n        display: flex;\n      }\n      .top[_ngcontent-xbj-3] {\n        height: 10vh;\n        min-height: 100px;\n        justify-content: space-between;\n        padding-top: 29px;\n        z-index: 2;\n      }\n      .top[_ngcontent-xbj-3] a[_ngcontent-xbj-3] {\n        text-decoration: none;\n      }\n      .top[_ngcontent-xbj-3] a[_ngcontent-xbj-3]:hover {\n        text-decoration: underline;\n      }\n      .middle[_ngcontent-xbj-3] {\n        height: 69vh;\n        display: flex;\n        align-items: center;\n      }\n      .bottom[_ngcontent-xbj-3] {\n        display: flex;\n        height: 21vh;\n        justify-content: flex-end;\n      }\n      .bottom[_ngcontent-xbj-3],\n      .middle[_ngcontent-xbj-3],\n      .top[_ngcontent-xbj-3] {\n        width: 100%;\n      }\n      .middle[_ngcontent-xbj-3] {\n        position: relative;\n      }\n      .left-arrow[_ngcontent-xbj-3],\n      .right-arrow[_ngcontent-xbj-3] {\n        position: absolute;\n        top: 0;\n        bottom: 0;\n        width: 50%;\n      }\n      .left-arrow[_ngcontent-xbj-3] {\n        left: 0;\n        cursor: url(/assets/left.png), w-resize;\n      }\n      .right-arrow[_ngcontent-xbj-3] {\n        right: 0;\n        cursor: url(/assets/right.png), e-resize;\n      }\n      .image-wrapper[_ngcontent-xbj-3] {\n        align-items: center;\n        display: flex;\n        justify-content: center;\n        margin: 0 auto;\n        height: 100%;\n        width: 80vw;\n      }\n      svg[_ngcontent-xbj-3] {\n        fill: #333e48;\n      }\n      .title-wrapper[_ngcontent-xbj-3] {\n        flex: 0 1 40%;\n        height: 21vh;\n        max-width: 500px;\n        min-width: 360px;\n        text-align: right;\n      }\n      p[_ngcontent-xbj-3] {\n        margin: 0;\n      }\n      .title-wrapper[_ngcontent-xbj-3] hr[_ngcontent-xbj-3] {\n        display: none;\n      }\n      .image[_ngcontent-xbj-3] {\n        background-size: contain;\n        background-repeat: no-repeat;\n        background-position: 50%;\n        background-color: #fff;\n        height: 69vh;\n        max-width: 800px;\n        width: 80vw;\n      }\n    ".to_owned()), Element::InlineCss("\n      @font-face {\n        font-family: franklin-gothic-urw;\n        src: url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/l?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff2\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/d?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"woff\"),\n          url(https://use.typekit.net/af/9cb78a/0000000000000000000118ad/27/a?primer=7cdcb44be4a7db8877ffa5c0007b8dd865b3bbc383831fe2ea177f62257a9191&amp;fvd=n4&amp;v=3)\n            format(\"opentype\");\n        font-weight: 400;\n        font-style: normal;\n        font-stretch: normal;\n        font-display: auto;\n      }\n    ".to_owned()), 
                                    Element::InlineCss("\n      a[_ngcontent-xbj-1] {\n        text-decoration: none;\n      }\n      .hover[_ngcontent-xbj-1] a[_ngcontent-xbj-1]:hover {\n        text-decoration: underline;\n      }\n    ".to_owned())];

        assert_eq!(elements, expected_results);

        Ok(())
    }
}
