use async_channel::{Receiver, Sender};
use eyre::Context;
use tokio::task::JoinHandle;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::crawler::http_crawler::HttpCrawler;

use super::{channel_message::ChannelMessage, Page};

pub fn start_html_http_tasks(
    html_http_node_rx: &Receiver<ChannelMessage<String>>,
    verifier_node_tx: &Sender<ChannelMessage<Page>>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<()>> {
    (0..no_of_tasks)
        .map(|i| start_html_http_task(html_http_node_rx.clone(), verifier_node_tx.clone(), i))
        .collect()
}

fn start_html_http_task(
    html_http_node_rx: Receiver<ChannelMessage<String>>,
    verifier_node_tx: Sender<ChannelMessage<Page>>,
    i: i32,
) -> JoinHandle<()> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();

    tokio::spawn(async move {
        while let Ok(message) = html_http_node_rx.recv().await {
            let span = tracing::info_span!("html_http_job");
            span.set_parent(message.extract());

            let content = message.unwrap();
            let root_span = message.root_span();
            if let Err(err) = html_http_job(
                content.to_owned(),
                i,
                &crawler,
                &verifier_node_tx,
                root_span,
            )
            .await
            {
                tracing::error!(error = ?err, "Failed to fetch content");
            }
        }
        tracing::info!("HTTP HTML FETCHER TASK DONE");
    })
}

#[tracing::instrument(skip(crawler, verifier_node_tx, root_span))]
async fn html_http_job(
    url: String,
    i: i32,
    crawler: &HttpCrawler,
    verifier_node_tx: &Sender<ChannelMessage<Page>>,
    root_span: &tracing::Span,
) -> eyre::Result<()> {
    tracing::info!("Received HTTP FETCHER JOB on task {}. Url: {}", i, &url);

    let content = match crawler.get_page_content(&url).await {
        Ok(content) => {
            tracing::info!("got content for url: {}", &url);
            content
        }
        Err(err) => {
            return Err(err).wrap_err(format!("Unable to get page content for {}.", &url));
        }
    };

    let page = Page::new(url.clone(), content);

    let mut message = ChannelMessage::new(root_span.to_owned(), page);
    message.inject(&root_span.context());

    if let Err(err) = verifier_node_tx.send(message).await {
        return Err(err).wrap_err(format!(
            "Could not send content to verifier job for url {}",
            &url
        ));
    }

    Ok(())
}
