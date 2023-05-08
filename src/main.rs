use std::{fs, vec};

use crate::{
    crawler::{browser_crawler::BrowserCrawler, http_crawler::HttpCrawler},
    tasks::{
        channel_message::ChannelMessage, html_browser::start_html_browser_tasks,
        html_http::start_html_http_tasks, page::start_page_tasks, verifier::start_verifier_tasks,
        Page, SiteData,
    },
};
use eyre::{eyre, Context};
use opentelemetry::global;
use tracing::{error, Instrument};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use url::Url;

mod crawler;
mod font_parser;
mod parsers;
mod tasks;
mod tracer;
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
    tracer::init_tracing()?;

    let args: Vec<String> = std::env::args().collect();

    let url = args.get(1);

    if let Some(url) = url {
        let base_url: String = Url::parse(url)
            .expect("Could not parse url")
            .as_str()
            .to_owned();

        let crawler: HttpCrawler = HttpCrawler::new()?;

        let content = get_content_from_url(&url).await?;

        let page = Page::new(url.to_owned(), content.clone());

        let all_font_data = match SiteData::from_page(&crawler, &page).await {
            Ok(data) => {
                tracing::info!("Got data!");
                data
            }
            Err(err) => match err {
                CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                    tracing::error!("{}", err);
                    return Ok(());
                }
                _ => {
                    tracing::error!("Unable to get site data for {}", &url);
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

        let (page_node_tx, page_node_rx) = async_channel::bounded::<ChannelMessage<Page>>(5);
        let (verifier_node_tx, verifier_node_rx) =
            async_channel::bounded::<ChannelMessage<Page>>(3);

        let (html_http_node_tx, html_http_node_rx) =
            async_channel::bounded::<ChannelMessage<String>>(3);
        let (html_browser_node_tx, html_browser_node_rx) =
            async_channel::bounded::<ChannelMessage<String>>(3);

        let html_http_handles = start_html_http_tasks(&html_http_node_rx, &verifier_node_tx, 3);

        let verifier_handles =
            start_verifier_tasks(&verifier_node_rx, &html_browser_node_tx, &page_node_tx, 3);

        let html_browser_handles =
            start_html_browser_tasks(&html_browser_node_rx, &page_node_tx, 3);

        let page_handles = start_page_tasks(&page_node_rx, 5);

        start_jobs(urls, &html_http_node_tx).await;

        // drop the transmitters to close the channel
        drop(html_http_node_tx);
        drop(verifier_node_tx);
        drop(html_browser_node_tx);
        drop(page_node_tx);

        for h in html_http_handles {
            h.await.map_err(|err| eyre!(err))?;
            println!("HTTP HTML FERDIG");
        }

        for h in verifier_handles {
            h.await.map_err(|err| eyre!(err))?;
            println!("VERIFIER FERDIG");
        }

        for h in html_browser_handles {
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

    global::shutdown_tracer_provider();
    Ok(())
}

async fn start_jobs(
    urls: Vec<String>,
    html_http_node_tx: &async_channel::Sender<ChannelMessage<String>>,
) {
    for url in urls {
        start_job(url, html_http_node_tx).await;
    }
}

#[tracing::instrument(skip(html_http_node_tx))]
async fn start_job(url: String, html_http_node_tx: &async_channel::Sender<ChannelMessage<String>>) {
    tracing::info!("Starting job");

    let span = tracing::Span::current();

    let mut message = ChannelMessage::new(span.to_owned(), url);
    message.inject(&span.context());

    if let Err(_) = html_http_node_tx.send(message).instrument(span).await {
        tracing::error!("Could not send to html_http channel");
    }
}

// Fetches with http, verifies, and fetches with browser if necessary
async fn get_content_from_url(url: &str) -> eyre::Result<String> {
    let crawler: HttpCrawler = HttpCrawler::new()?;

    let content = match crawler.get_page_content(&url).await {
        Ok(content) => {
            tracing::info!("Got content with http!");
            content
        }
        Err(err) => {
            tracing::error!("{}", err);
            tracing::info!("Could not get fetch with http. Trying with browser");
            let browser_crawler: BrowserCrawler = BrowserCrawler::new()?;
            browser_crawler
                .get_page_content(&url)
                .wrap_err(format!("Unable to get page content for {}.", &url))?
        }
    };

    let page = Page::new(url.to_owned(), content.clone());

    if let Err(err) = crawler.get_font_urls_from_page(&page).await {
        match err {
            CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                tracing::info!(
                    "Could not verify content for url {}. Sending to browser.",
                    page.base_url
                );

                let browser_crawler: BrowserCrawler = BrowserCrawler::new()?;
                return browser_crawler
                    .get_page_content(&url)
                    .wrap_err(format!("Unable to get page content for {}.", &url));
            }
            err => {
                tracing::error!(
                    "Unable to get site data for {}. Err: {}",
                    &page.base_url,
                    err
                );

                return Err(err)
                    .wrap_err(format!("Unable to get site data for {}.", &page.base_url));
            }
        }
    }

    return Ok(content);
}
