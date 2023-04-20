use async_channel::{Receiver, Sender};
use tokio::task::JoinHandle;

use crate::{crawler::http_crawler::HttpCrawler, CustomError};

use super::Page;

pub fn start_verifier_tasks(
    verifier_node_rx: &Receiver<Page>,
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
            )
        })
        .collect()
}

fn start_verifier_task(
    verifier_node_rx: Receiver<Page>,
    browser_html_node_tx: Sender<String>,
    page_node_tx: Sender<Page>,
) -> JoinHandle<()> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();
    // let span = span!(Level::INFO, "verifier_worker", i);

    tokio::spawn(async move {
        // let _enter = span.enter();
        while let Ok(page) = verifier_node_rx.recv().await {
            // info!(
            //     "Receiver VERIFIER NODE JOB on task {}. Url: {}",
            //     i, &page.base_url
            // );

            match crawler.get_font_urls_from_page(&page).await {
                Ok(_) => {
                    // info!("Verified url {}", page.base_url);
                    if let Err(_) = page_node_tx.send(page).await {
                        tracing::error!("Could not send page to site data tx")
                    }
                }
                Err(err) => match err {
                    CustomError::NoElementsFound(_) | CustomError::NoFontUrlsFound(_) => {
                        if let Err(_) = browser_html_node_tx.send(page.base_url).await {
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
        // info!("VERIFIER TASK DONE");
    })
}
