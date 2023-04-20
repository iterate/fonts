use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::crawler::http_crawler::HttpCrawler;

use super::Page;

pub fn start_html_http_tasks(
    html_http_node_rx: &Receiver<String>,
    verifier_node_tx: &Sender<Page>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<()>> {
    (0..no_of_tasks)
        .map(|i| start_html_http_task(html_http_node_rx.clone(), verifier_node_tx.clone()))
        .collect()
}

fn start_html_http_task(
    html_http_node_rx: Receiver<String>,
    verifier_node_tx: Sender<Page>,
) -> JoinHandle<()> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();
    // let page_node_tx = page_node_tx.clone();

    // let parent_span = span!(Level::INFO, "http_html_worker", i);

    tokio::spawn(async move {
        // let _enter = parent_span.enter();
        while let Ok(url) = html_http_node_rx.recv().await {
            // let child_span = span!(Level::INFO, "url", url);

            // let _enter = child_span.enter();

            // info!("Received HTTP FETCHER JOB on task {}. Url: {}", i, &url);

            let content = match crawler.get_page_content(&url).await {
                Ok(content) => {
                    // info!("got content for url: {}", &url);
                    content
                }
                Err(err) => {
                    tracing::error!("Unable to get page content for {}. Err: {}", &url, err);
                    continue;
                }
            };

            let page = Page::new(url, content);

            if let Err(_) = verifier_node_tx.send(page).await {
                tracing::error!("Could not send page to site data tx")
            }

            // if let Err(_) = page_node_tx.send(page).await {
            //     tracing:error!("Could not send page to site data tx")
            // }
        }
        // info!("HTTP HTML FETCHER TASK DONE");
    })
}
