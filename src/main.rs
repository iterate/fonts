use std::{fs, time::Duration, vec};

use eyre::eyre;
use flume;
use tokio::task::JoinHandle;
use url::Url;

use crate::{crawler::http_crawler::HttpCrawler, font_parser::FontData};

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

const HTTP_N_WORKERS: i32 = 5;

pub type Result<T, E = CustomError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum CustomError {
    #[error("No elements found: {0}")]
    NoElementsFound(String),
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
        let all_font_data = match get_site_data_from_page(&crawler, &base_url).await {
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

        let (html_crawler_tx, html_crawler_rx) = flume::bounded::<String>(5);

        let (browser_crawler_tx, browser_crawler_rx) = flume::bounded::<String>(2);

        let browser_crawler_handles: Vec<JoinHandle<_>> = (0..1)
            .map(|i| {
                let worker_rx = browser_crawler_rx.clone();
                tokio::spawn(async move {
                    let mut thread_site_data: Vec<SiteData> = vec![];
                    while let Ok(url) = worker_rx.recv() {
                        println!("Received BROWSER JOB on task {}. Url: {}", i, url);

                        // tokio::time::sleep(Duration::from_secs(5)).await;

                        // match get_site_data_from_page(&crawler, &url).await {
                        //     Ok(data) => thread_site_data.push(data),
                        //     Err(err) => match err {
                        //         CustomError::NoLinksFound(url) => {
                        //             eprintln!("Could not send to browser crawler")
                        //         }
                        //         _ => {
                        //             eprintln!("Unable to get site data for {}. Err: {}", &url, err)
                        //         }
                        //     },
                        // }
                    }
                    thread_site_data
                })
            })
            .collect();

        let html_crawler_handles: Vec<JoinHandle<_>> = (0..HTTP_N_WORKERS)
            .map(|i| {
                let worker_rx = html_crawler_rx.clone();
                // let worker_browser_tx = browser_crawler_tx.clone();
                tokio::spawn(async move {
                    let mut thread_site_data: Vec<SiteData> = vec![];
                    while let Ok(url) = worker_rx.recv() {
                        println!("Received job on task {}. Url: {}", i, url);
                        let crawler: HttpCrawler = HttpCrawler::new().unwrap();
                        // if let Err(_) = worker_browser_tx.send(url.clone()) {
                        //     eprintln!("Could not send to browser crawler")
                        // }

                        match get_site_data_from_page(&crawler, &url).await {
                            Ok(data) => thread_site_data.push(data),
                            Err(err) => match err {
                                CustomError::NoElementsFound(_) => {
                                    println!("SENDER TIL BROWSER");
                                }
                                _ => {
                                    eprintln!("Unable to get site data for {}. Err: {}", &url, err)
                                }
                            },
                        }
                    }
                    thread_site_data
                })
            })
            .collect();

        for url in urls {
            if let Err(_) = html_crawler_tx.send(url) {
                println!("Could not send to channel");
            }
        }

        // drop the transmitter to close the channel
        drop(html_crawler_tx);
        drop(browser_crawler_tx);

        let mut all_site_data: Vec<SiteData> = vec![];
        for h in html_crawler_handles {
            let r = h.await.map_err(|err| eyre!(err))?;
            println!("heihei ferdig: {:?}", r.len());

            all_site_data.extend(r);
        }

        for h in browser_crawler_handles {
            let r = h.await.map_err(|err| eyre!(err))?;
            println!("BROWSER FERDIG: {:?}", r.len());
        }

        println!("Font data for everything");
        println!("Length: {}", all_site_data.len());
        println!("{:#?}", all_site_data);
    }

    Ok(())
}

#[derive(Debug)]
struct SiteData {
    url: String,
    fonts: Vec<FontData>,
}

async fn get_site_data_from_page(crawler: &HttpCrawler, base_url: &str) -> Result<SiteData> {
    let font_urls = crawler.get_font_urls_from_page(&base_url).await?;

    let mut font_contents: Vec<Vec<u8>> = vec![];

    for font_url in &font_urls {
        let font_content = match crawler.get_font_file_as_bytes(font_url.as_str()).await {
            Ok(font_content) => font_content,
            Err(err) => {
                eprintln!("{}", err);
                continue;
            }
        };
        font_contents.push(font_content);
    }

    println!("Found {} font urls", font_urls.len());

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
        url: base_url.to_owned(),
        fonts: all_font_data,
    })
}
