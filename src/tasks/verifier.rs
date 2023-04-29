use async_channel::{Receiver, Sender};
use eyre::{Context, Result};
use tokio::task::JoinHandle;

use crate::{crawler::http_crawler::HttpCrawler, CustomError};

use super::{channel_message::ChannelMessage, Page};

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
            let content = message.unwrap();
            if let Err(err) =
                verify(content, i, &crawler, &page_node_tx, &browser_html_node_tx).await
            {
                tracing::error!(error = ?err, "Failed to verify");
            }
        }
        tracing::info!("VERIFIER TASK DONE");
    })
}

#[tracing::instrument(skip(page, crawler, page_node_tx, browser_html_node_tx))]
async fn verify(
    page: &Page,
    i: i32,
    crawler: &HttpCrawler,
    page_node_tx: &Sender<Page>,
    browser_html_node_tx: &Sender<String>,
) -> Result<()> {
    tracing::info!(
        "Receiver VERIFIER NODE JOB on task {}. Url: {}",
        i,
        &page.base_url
    );

    match crawler.get_font_urls_from_page(&page).await {
        Ok(_) => {
            tracing::info!("Verified url {}", page.base_url);
            if let Err(err) = page_node_tx.send(page.clone()).await {
                return Err(err).wrap_err("Could not send page to site data tx");
            }
            Ok(())
        }
        Err(err) => match err {
            CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                if let Err(_) = browser_html_node_tx.send(page.base_url.to_owned()).await {
                    Err(err).wrap_err("Could not send page to browser html node tx")
                } else {
                    Ok(())
                }
            }
            err => Err(err).wrap_err(format!("Unable to get site data for {}.", &page.base_url)),
        },
    }
}
