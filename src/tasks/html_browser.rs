use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::crawler::browser_crawler::BrowserCrawler;

use super::Page;

pub fn start_html_browser_tasks(
    html_browser_node_rx: &Receiver<String>,
    page_node_tx: &Sender<Page>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<()>> {
    (0..no_of_tasks)
        .map(|i| start_html_browser_task(html_browser_node_rx.clone(), page_node_tx.clone(), i))
        .collect()
}

fn start_html_browser_task(
    html_browser_node_rx: Receiver<String>,
    page_node_tx: Sender<Page>,
    i: i32,
) -> JoinHandle<()> {
    let crawler: BrowserCrawler = BrowserCrawler::new().unwrap();

    tokio::spawn(async move {
        while let Ok(url) = html_browser_node_rx.recv().await {
            tracing::info!("Received BROWSER JOB on task {}. Url: {}", i, url);

            let content = match crawler.get_page_content(&url) {
                Ok(content) => content,
                Err(err) => {
                    tracing::error!("Unable to get page content for {}. Err: {}", &url, err);
                    continue;
                }
            };

            let page = Page::new(url, content);

            if let Err(_) = page_node_tx.send(page).await {
                tracing::error!("Could not send page to site data tx")
            }
        }
        tracing::info!("BROWSER HTML FETCHER TASK DONE");
    })
}
