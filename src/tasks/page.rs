use async_channel::Receiver;
use eyre::Context;
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::crawler::http_crawler::HttpCrawler;

use super::{channel_message::ChannelMessage, Page, SiteData};

pub fn start_page_tasks(
    page_node_rx: &Receiver<ChannelMessage<Page>>,
    no_of_tasks: i32,
) -> Vec<JoinHandle<Vec<SiteData>>> {
    (0..no_of_tasks)
        .map(|i| start_page_task(page_node_rx.clone(), i))
        .collect()
}

fn start_page_task(
    page_node_rx: Receiver<ChannelMessage<Page>>,
    i: i32,
) -> JoinHandle<Vec<SiteData>> {
    let crawler: HttpCrawler = HttpCrawler::new().unwrap();

    tokio::spawn(async move {
        let mut thread_site_data: Vec<SiteData> = vec![];
        while let Ok(message) = page_node_rx.recv().await {
            let span = tracing::info_span!("page_job");
            message.link_to_span(&span);

            let content = message.unwrap();

            match get_site_data(content, i, &crawler).instrument(span).await {
                Ok(site_data) => {
                    tracing::info!("Pushing site data to list");
                    thread_site_data.push(site_data);
                }
                Err(err) => tracing::error!(error = ?err, "Failed to get site data"),
            }
        }
        tracing::info!("PAGE TASK DONE");
        thread_site_data
    })
}

#[tracing::instrument(skip(crawler, page), fields(url=page.base_url))]
async fn get_site_data(page: &Page, i: i32, crawler: &HttpCrawler) -> eyre::Result<SiteData> {
    tracing::info!("Received job on task {}. url: {:#?}", i, &page.base_url);

    match SiteData::from_page(&crawler, &page).await {
        Ok(data) => {
            tracing::info!("Success! url: {}", &page.base_url);
            Ok(data)
        }
        Err(err) => Err(err).wrap_err(format!(
            "Unable to get site data for url {}.",
            &page.base_url,
        )),
    }
}
