use async_channel::{Receiver, Sender};
use eyre::Context;
use tokio::task::JoinHandle;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::crawler::browser_crawler::BrowserCrawler;

use super::{channel_message::ChannelMessage, Page};

pub fn start_html_browser_tasks(
    html_browser_node_rx: &Receiver<ChannelMessage<String>>,
    page_node_tx: &Sender<ChannelMessage<Page>>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<()>> {
    (0..no_of_tasks)
        .map(|i| start_html_browser_task(html_browser_node_rx.clone(), page_node_tx.clone(), i))
        .collect()
}

fn start_html_browser_task(
    html_browser_node_rx: Receiver<ChannelMessage<String>>,
    page_node_tx: Sender<ChannelMessage<Page>>,
    i: i32,
) -> JoinHandle<()> {
    let crawler: BrowserCrawler = BrowserCrawler::new().unwrap();

    tokio::spawn(async move {
        while let Ok(message) = html_browser_node_rx.recv().await {
            let span = tracing::info_span!("html_browser_job");
            message.link_to_span(&span);

            let content = message.unwrap();

            if let Err(err) =
                fetch_html_content_with_browser(content.to_owned(), i, &crawler, &page_node_tx)
                    .instrument(span)
                    .await
            {
                tracing::error!(error = ?err, "Failed to fetch content with browser");
            }
        }
        tracing::info!("BROWSER HTML FETCHER TASK DONE");
    })
}

#[tracing::instrument(skip(crawler, page_node_tx))]
async fn fetch_html_content_with_browser(
    url: String,
    i: i32,
    crawler: &BrowserCrawler,
    page_node_tx: &Sender<ChannelMessage<Page>>,
) -> eyre::Result<()> {
    tracing::info!("Received BROWSER JOB on task {}. Url: {}", i, url);

    let content = match crawler.get_page_content(&url) {
        Ok(content) => {
            tracing::info!("got content for url: {}", &url);
            content
        }
        Err(err) => {
            return Err(err).wrap_err(format!("Unable to get page content for {}.", &url));
        }
    };

    let page = Page::new(url, content);
    let mut message = ChannelMessage::new(page);
    message.inject(&tracing::Span::current().context());

    if let Err(_) = page_node_tx.send(message).await {
        tracing::error!("Could not send page to site data tx")
    }
    Ok(())
}
