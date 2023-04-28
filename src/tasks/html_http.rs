use async_channel::{Receiver, Sender};
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
            let url = message.unwrap();

            tracing::info!("Received HTTP FETCHER JOB on task {}. Url: {}", i, &url);

            let content = match crawler.get_page_content(&url).await {
                Ok(content) => {
                    tracing::info!("got content for url: {}", &url);
                    content
                }
                Err(err) => {
                    tracing::error!("Unable to get page content for {}. Err: {}", &url, err);
                    continue;
                }
            };

            let page = Page::new(url.clone(), content);

            if let Err(_) = send_message_to_channel(&verifier_node_tx, page).await {
                tracing::error!("Could not send page to site data tx")
            }
        }
        tracing::info!("HTTP HTML FETCHER TASK DONE");
    })
}
