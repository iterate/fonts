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
            span.set_parent(message.extract());

            let content = message.unwrap();
            let root_span = message.root_span();
            if let Err(err) = fetch_html_content_with_browser(
                content.to_owned(),
                i,
                &crawler,
                &page_node_tx,
                root_span,
            )
            .instrument(span)
            .await
            {
                tracing::error!(error = ?err, "Failed to perform html browser job");
            }
        }
        tracing::info!("browser html task {} done.", i);
    })
}

#[tracing::instrument(skip(crawler, page_node_tx))]
async fn fetch_html_content_with_browser(
    url: String,
    i: i32,
    crawler: &BrowserCrawler,
    page_node_tx: &Sender<ChannelMessage<Page>>,
    root_span: &tracing::Span,
) -> eyre::Result<()> {
    tracing::info!("Received job on task {}.", i);

    let content = crawler
        .get_page_content(&url)
        .wrap_err(format!("Unable to get page content for {}.", &url))?;

    tracing::info!("gotten page content for url: {}", &url);

    let page = Page::new(url.clone(), content);
    let mut message = ChannelMessage::new(root_span.to_owned(), page);
    message.inject(&tracing::Span::current().context());

    page_node_tx
        .send(message)
        .await
        .wrap_err(format!("Could not send data to page job for url {}", &url))
}
