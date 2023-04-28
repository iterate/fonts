use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::{crawler::http_crawler::HttpCrawler, CustomError};

use super::{sender::ChannelMessage, Page};

pub fn start_verifier_tasks(
    verifier_node_rx: &Receiver<ChannelMessage<Page>>,
    browser_html_node_tx: &Sender<String>,
    page_node_tx: &Sender<Page>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<()>> {
    (0..no_of_tasks)
        .map(|i| {
            start_verifier_task(
                verifier_node_rx.clone(),
                browser_html_node_tx.clone(),
                page_node_tx.clone(),
                i,
            )
        })
        .collect()
}

fn start_verifier_task(
    verifier_node_rx: Receiver<ChannelMessage<Page>>,
    browser_html_node_tx: Sender<String>,
    page_node_tx: Sender<Page>,
    i: i32,
) -> JoinHandle<()> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();

    tokio::spawn(async move {
        while let Ok(message) = verifier_node_rx.recv().await {
            let page = message.unwrap();

            tracing::info!(
                "Receiver VERIFIER NODE JOB on task {}. Url: {}",
                i,
                &page.base_url
            );

            let page = message.unwrap();

            match crawler.get_font_urls_from_page(&page).await {
                Ok(_) => {
                    tracing::info!("Verified url {}", page.base_url);
                    if let Err(_) = page_node_tx.send(page.clone()).await {
                        tracing::error!("Could not send page to site data tx")
                    }
                }
                Err(err) => match err {
                    CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                        if let Err(_) = browser_html_node_tx.send(page.base_url.to_owned()).await {
                            tracing::error!("Could not send page to browser html node tx")
                        }
                    }
                    _ => {
                        tracing::error!(
                            "Unable to get site data for {}. Err: {}",
                            &page.base_url,
                            err
                        )
                    }
                },
            }
        }
        tracing::info!("VERIFIER TASK DONE");
    })
}
