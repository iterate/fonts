use std::{fs, vec};

use eyre::eyre;
use tokio::task::JoinHandle;
use url::Url;

use crate::{
    crawler::{browser_crawler::BrowserCrawler, http_crawler::HttpCrawler},
    font_parser::FontData,
};

mod crawler;
mod font_parser;
use thiserror::Error;

// KNOWN ISSUES
// Cant find font data for https://ense.no
// - bleh. dynamisk lastet inn.
// - https://kerkour.com/rust-crawler-javascript-single-page-application-headless-browser
// - https://github.com/skerkour/black-hat-rust/blob/main/ch_05/crawler/src/spiders/quotes.rs#L51
// - https://webant.online/tutorials/web-scraping-rust-fantoccini/
// Cant find stylesheet for https//www.hjernelaering.no/
// - but its there... this is a bug

pub type Result<T, E = CustomError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("No elements found: {0}")]
    NoElementsFound(String),
    #[error("No font urls found: {0}")]
    NoFontUrlsFound(String),
    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Eyre report: {0}")]
    GenericError(#[from] eyre::Report),
}

#[tokio::main]
async fn main() -> Result<()> {
    // let font_data = FontData::from_filepath("test_font_1.woff")?;
    // println!("family name: {}", &font_data.family_name);
    // println!("sub name: {}", &font_data.sub_family_name);
    // println!("full name: {}", &font_data.full_name);

    let args: Vec<String> = std::env::args().collect();

    let url = args.get(1);

    if let Some(url) = url {
        let base_url: String = Url::parse(url)
            .expect("Could not parse url")
            .as_str()
            .to_owned();

        let crawler: HttpCrawler = HttpCrawler::new()?;

        let content = match crawler.get_page_content(&url).await {
            Ok(content) => content,
            Err(err) => {
                panic!("Unable to get page content for {}. Err: {}", &url, err)
            }
        };

        let page = Page {
            base_url: url.to_owned(),
            page_content: content,
        };

        let all_font_data = match get_site_data_from_page(&crawler, &page).await {
            Ok(data) => data,
            Err(err) => match err {
                CustomError::NoElementsFound(_) => {
                    println!("hallo");
                    return Ok(());
                }
                _ => {
                    eprintln!("Unable to get site data for {}. Err: {}", &url, err);
                    return Err(err);
                }
            },
        };
        println!("Font data for {}", base_url);
        println!("{:#?}", all_font_data);
    } else {
        let urls: Vec<String> = fs::read_to_string("test_files/test_urls.txt")
            .map_err(|err| eyre!(err))
            .map(|file| file.split("\n").map(|s| s.to_owned()).collect())
            .expect("could not load file");

        let (page_node_tx, page_node_rx) = async_channel::bounded::<Page>(5);
        let (verifier_node_tx, verifier_node_rx) = async_channel::bounded::<Page>(3);

        let (http_html_node_tx, http_html_node_rx) = async_channel::bounded::<String>(3);
        let (browser_html_node_tx, browser_html_node_rx) = async_channel::bounded::<String>(3);

        let http_html_handles: Vec<JoinHandle<_>> = (0..3)
            .map(|i| {
                let crawler: HttpCrawler = HttpCrawler::new().unwrap();

                let http_html_node_rx = http_html_node_rx.clone();
                let verifier_node_tx = verifier_node_tx.clone();
                // let page_node_tx = page_node_tx.clone();

                tokio::spawn(async move {
                    while let Ok(url) = http_html_node_rx.recv().await {
                        println!("Received HTTP FETCHER JOB on task {}. Url: {}", i, &url);

                        let content = match crawler.get_page_content(&url).await {
                            Ok(content) => {
                                println!("got content for url: {}", &url);
                                content
                            }
                            Err(err) => {
                                eprintln!("Unable to get page content for {}. Err: {}", &url, err);
                                continue;
                            }
                        };

                        let page = Page {
                            base_url: url.to_owned(),
                            page_content: content,
                        };

                        if let Err(_) = verifier_node_tx.send(page).await {
                            eprintln!("Could not send page to site data tx")
                        }

                        // if let Err(_) = page_node_tx.send(page) {
                        //     eprintln!("Could not send page to site data tx")
                        // }
                    }
                    println!("HTTP HTML FETCHER TASK DONE");
                })
            })
            .collect();

        let verifier_handles: Vec<JoinHandle<_>> = (0..3)
            .map(|i| {
                let crawler: HttpCrawler = HttpCrawler::new().unwrap();

                let verifier_node_rx = verifier_node_rx.clone();
                let browser_html_node_tx = browser_html_node_tx.clone();
                let page_node_tx = page_node_tx.clone();

                tokio::spawn(async move {
                    while let Ok(page) = verifier_node_rx.recv().await {
                        println!(
                            "Receiver VERIFIER NODE JOB on task {}. Url: {}",
                            i, &page.base_url
                        );

                        match crawler.get_font_urls_from_page(&page).await {
                            Ok(_) => {
                                println!("Verified url {}", page.base_url);
                                if let Err(_) = page_node_tx.send(page).await {
                                    eprintln!("Could not send page to site data tx")
                                }
                            }
                            Err(err) => match err {
                                CustomError::NoElementsFound(_)
                                | CustomError::NoFontUrlsFound(_) => {
                                    if let Err(_) = browser_html_node_tx.send(page.base_url).await {
                                        eprintln!("Could not send page to browser html node tx")
                                    }
                                }
                                _ => {
                                    eprintln!(
                                        "Unable to get site data for {}. Err: {}",
                                        &page.base_url, err
                                    )
                                }
                            },
                        }
                    }
                    println!("VERIFIER TASK DONE");
                })
            })
            .collect();

        let browser_html_handles: Vec<JoinHandle<_>> = (0..3)
            .map(|i| {
                let crawler: BrowserCrawler = BrowserCrawler::new().unwrap();

                let browser_html_node_rx = browser_html_node_rx.clone();
                let page_node_tx = page_node_tx.clone();

                tokio::spawn(async move {
                    while let Ok(url) = browser_html_node_rx.recv().await {
                        println!("Received BROWSER JOB on task {}. Url: {}", i, url);

                        let content = match crawler.get_page_content(&url) {
                            Ok(content) => content,
                            Err(err) => {
                                eprintln!("Unable to get page content for {}. Err: {}", &url, err);
                                continue;
                            }
                        };

                        let page = Page {
                            base_url: url.to_owned(),
                            page_content: content,
                        };

                        if let Err(_) = page_node_tx.send(page).await {
                            eprintln!("Could not send page to site data tx")
                        }
                    }
                    println!("BROWSER HTML FETCHER TASK DONE");
                })
            })
            .collect();

        let page_handles: Vec<JoinHandle<_>> = (0..5)
            .map(|i| {
                let crawler: HttpCrawler = HttpCrawler::new().unwrap();

                let page_node_rx = page_node_rx.clone();

                tokio::spawn(async move {
                    let mut thread_site_data: Vec<SiteData> = vec![];
                    while let Ok(page) = page_node_rx.recv().await {
                        println!("Received job on task {}. url: {:#?}", i, &page.base_url);

                        match get_site_data_from_page(&crawler, &page).await {
                            Ok(data) => {
                                println!("Success! url: {}", &page.base_url);
                                thread_site_data.push(data)
                            }
                            Err(err) => {
                                eprintln!(
                                    "Unable to get site data after validation node {}. Err: {}",
                                    &page.base_url, err
                                )
                            }
                        }
                    }
                    println!("PAGE TASK DONE");
                    thread_site_data
                })
            })
            .collect();

        for url in urls {
            if let Err(_) = http_html_node_tx.send(url).await {
                println!("Could not send to channel");
            }
        }

        // drop the transmitters to close the channel
        drop(http_html_node_tx);
        drop(verifier_node_tx);
        drop(browser_html_node_tx);
        drop(page_node_tx);

        for h in http_html_handles {
            h.await.map_err(|err| eyre!(err))?;
            println!("HTTP HTML FERDIG");
        }

        for h in verifier_handles {
            h.await.map_err(|err| eyre!(err))?;
            println!("VERIFIER FERDIG");
        }

        for h in browser_html_handles {
            h.await.map_err(|err| eyre!(err))?;
            println!("BROWSER HTML FERDIG");
        }

        let mut all_site_data: Vec<SiteData> = vec![];
        for h in page_handles {
            let r = h.await.map_err(|err| eyre!(err))?;
            println!("heihei ferdig: {:?}", r.len());

            all_site_data.extend(r);
        }

        println!("Font data for everything");
        println!("Length: {}", all_site_data.len());
        println!("{:#?}", all_site_data);
    }

    Ok(())
}

#[derive(Debug)]
pub struct Page {
    base_url: String,
    page_content: String,
}

#[derive(Debug)]
struct SiteData {
    url: String,
    fonts: Vec<FontData>,
}

async fn get_site_data_from_page(crawler: &HttpCrawler, page: &Page) -> Result<SiteData> {
    // Get page content to find links to follow
    // let page_content = crawler.get_page_content(base_url).await?;

    println!("yesyes");

    let font_urls = crawler.get_font_urls_from_page(page).await?;

    let mut font_contents: Vec<Vec<u8>> = vec![];

    for font_url in &font_urls {
        let font_content = match crawler.get_font_file_as_bytes(font_url.as_str()).await {
            Ok(font_content) => font_content,
            Err(err) => {
                //eprintln!("{}", err);
                continue;
            }
        };
        font_contents.push(font_content);
    }

    //println!("Found {} font urls", font_urls.len());

    let all_font_data: Vec<FontData> = font_contents
        .iter()
        .filter_map(|font_content| match FontData::from_bytes(font_content) {
            Ok(font_content) => Some(font_content),
            Err(_) => {
                //eprintln!("{}", err);
                None
            }
        })
        .collect();

    Ok(SiteData {
        url: page.base_url.to_owned(),
        fonts: all_font_data,
    })
}
