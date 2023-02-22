use std::{fs, vec};

use eyre::{eyre, Result};
use flume;
use tokio::task::JoinHandle;
use url::Url;

use crate::{crawler::Crawler, font_parser::FontData};

mod crawler;
mod font_parser;

// KNOWN ISSUES
// Cant find font data for https://ense.no
// - bleh. dynamisk lastet inn.
// Cant find stylesheet for https//www.hjernelaering.no/
// - but its there... this is a bug

const N_WORKERS: i32 = 5;

#[tokio::main]
async fn main() -> Result<()> {
    // let font_data = FontData::from_filepath("test_font_1.woff")?;
    // println!("family name: {}", &font_data.family_name);
    // println!("sub name: {}", &font_data.sub_family_name);
    // println!("full name: {}", &font_data.full_name);

    let args: Vec<String> = std::env::args().collect();

    let url = args.get(1);

    if let Some(url) = url {
        let base_url: String = Url::parse(url)
            .expect("Could not parse url")
            .as_str()
            .to_owned();

        let crawler: Crawler = Crawler::new()?;
        let all_font_data = get_site_data_from_page(&crawler, &base_url).await?;
        println!("Font data for {}", base_url);
        println!("{:#?}", all_font_data);
    } else {
        let urls: Vec<String> = fs::read_to_string("test_files/test_urls.txt")
            .map_err(|err| eyre!(err))
            .map(|file| file.split("\n").map(|s| s.to_owned()).collect())
            .expect("could not load file");

        let (tx, rx) = flume::bounded::<String>(5);

        let handles: Vec<JoinHandle<_>> = (0..N_WORKERS)
            .map(|i| {
                let worker_rx = rx.clone();
                tokio::spawn(async move {
                    let mut thread_site_data: Vec<SiteData> = vec![];
                    while let Ok(url) = worker_rx.recv() {
                        println!("Received job on task {}. Url: {}", i, url);
                        let crawler: Crawler = Crawler::new().unwrap();

                        match get_site_data_from_page(&crawler, &url).await {
                            Ok(data) => thread_site_data.push(data),
                            Err(err) => {
                                eprintln!("Unable to get site data for {}. Err: {}", &url, err)
                            }
                        }
                    }
                    thread_site_data
                })
            })
            .collect();

        for url in urls {
            if let Err(_) = tx.send(url) {
                println!("Could not send to channel");
            }
        }

        drop(tx);

        let mut all_site_data: Vec<SiteData> = vec![];
        for h in handles {
            let r = h.await?;
            println!("heihei ferdig: {:?}", r.len());

            all_site_data.extend(r);
        }

        println!("Font data for everything");
        println!("Length: {}", all_site_data.len());
        println!("{:#?}", all_site_data);
    }

    Ok(())
}

#[derive(Debug)]
struct SiteData {
    url: String,
    fonts: Vec<FontData>,
}

async fn get_site_data_from_page(crawler: &Crawler, base_url: &str) -> Result<SiteData> {
    let font_urls = crawler.get_font_urls_from_page(&base_url).await?;

    let mut font_contents: Vec<Vec<u8>> = vec![];

    for font_url in &font_urls {
        let font_content = match crawler.get_font_file_as_bytes(font_url.as_str()).await {
            Ok(font_content) => font_content,
            Err(err) => {
                eprintln!("{}", err);
                continue;
            }
        };
        font_contents.push(font_content);
    }

    println!("Found {} font urls", font_urls.len());

    let all_font_data: Vec<FontData> = font_contents
        .iter()
        .filter_map(|font_content| match FontData::from_bytes(font_content) {
            Ok(font_content) => Some(font_content),
            Err(_) => {
                //eprintln!("{}", err);
                None
            }
        })
        .collect();

    Ok(SiteData {
        url: base_url.to_owned(),
        fonts: all_font_data,
    })
}
