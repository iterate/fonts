use async_channel::{Receiver, Sender};
use eyre::Context;
use tokio::task::JoinHandle;

use crate::crawler::http_crawler::HttpCrawler;

use super::{
    sender::{send_message_to_channel, ChannelMessage},
    Page,
};

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
            let content = message.unwrap();
            if let Err(err) =
                fetch_content(content.to_owned(), i, &crawler, &verifier_node_tx).await
            {
                tracing::error!(error = ?err, "Failed to fetch content");
            }
        }
        tracing::info!("HTTP HTML FETCHER TASK DONE");
    })
}

#[tracing::instrument(skip(crawler, verifier_node_tx))]
async fn fetch_content(
    url: String,
    i: i32,
    crawler: &HttpCrawler,
    verifier_node_tx: &Sender<ChannelMessage<Page>>,
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

    if let Err(err) =
        send_message_to_channel(tracing::Span::current().id(), verifier_node_tx, page).await
    {
        return Err(err).wrap_err("Could not send page to site data tx");
    }

    Ok(())
}
