use async_channel::{Receiver, Sender};
use eyre::Context;
use tokio::task::JoinHandle;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::{crawler::http_crawler::HttpCrawler, CustomError};

use super::{channel_message::ChannelMessage, Page};

pub fn start_verifier_tasks(
    verifier_node_rx: &Receiver<ChannelMessage<Page>>,
    browser_html_node_tx: &Sender<ChannelMessage<String>>,
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
    browser_html_node_tx: Sender<ChannelMessage<String>>,
    page_node_tx: Sender<Page>,
    i: i32,
) -> JoinHandle<()> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();

    tokio::spawn(async move {
        while let Ok(message) = verifier_node_rx.recv().await {
            let span = tracing::info_span!("receive_verifier_job");
            message.link_to_span(&span);

            let content = message.unwrap();
            if let Err(err) = verify(content, i, &crawler, &page_node_tx, &browser_html_node_tx)
                .instrument(span)
                .await
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
    browser_html_node_tx: &Sender<ChannelMessage<String>>,
) -> eyre::Result<()> {
    tracing::info!(
        "Received VERIFIER NODE JOB on task {}. Url: {}",
        i,
        &page.base_url
    );

    match crawler.get_font_urls_from_page(&page).await {
        Ok(_) => {
            tracing::info!("Verified url {}. Sending to page task.", page.base_url);
            if let Err(err) = page_node_tx.send(page.clone()).await {
                return Err(err).wrap_err("Could not send page to site data tx");
            }
            Ok(())
        }
        Err(err) => match err {
            CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                tracing::info!(
                    "Could not verify url {}. Sending to browser task.",
                    page.base_url
                );

                let mut message = ChannelMessage::new(page.base_url.to_owned());
                message.inject(&tracing::Span::current().context());

                if let Err(err) = browser_html_node_tx.send(message).await {
                    Err(err).wrap_err(format!(
                        "Could not send page to browser html node tx for url: {}",
                        &page.base_url
                    ))
                } else {
                    Ok(())
                }
            }
            err => Err(err).wrap_err(format!("Unable to get site data for {}.", &page.base_url)),
        },
    }
}
