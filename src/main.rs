use std::{fs, vec};

use eyre::Result;
use tokio::sync::mpsc;
use url::Url;

use crate::{crawler::Crawler, font_parser::FontData};

mod crawler;
mod font_parser;

// KNOWN ISSUES
// Cant find font data for https://ense.no
// - bleh. dynamisk lastet inn.

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
        let all_font_data = get_font_data_from_page(&crawler, &base_url).await?;
        println!("Font data for {}", base_url);
        println!("{:#?}", all_font_data);
    } else {
        let urls: Vec<String> = fs::read_to_string("test_files/test_urls.txt")
            .and_then(|file| Ok(file.split("\n").map(|s| s.to_owned()).collect()))
            .expect("could not load file");
        let (tx, mut rx) = mpsc::channel::<Result<Vec<FontData>>>(3);

        let mut all_font_data: Vec<FontData> = vec![];

        for url in urls {
            let task_tx = tx.clone();
            let crawler: Crawler = Crawler::new()?;
            let base_url = url.clone();

            tokio::spawn(async move {
                let font_data = get_font_data_from_page(&crawler, &base_url).await;
                if let Err(_) = task_tx.send(font_data).await {
                    return;
                }
            });
        }

        drop(tx); // ask someone why I have to drop sender explictly

        while let Some(font_data_result) = rx.recv().await {
            match font_data_result {
                Ok(font_data) => {
                    println!("Got result!");
                    all_font_data.extend(font_data);
                }
                Err(err) => eprintln!("Something went wrong: {}", err),
            }

            println!("DONE")
        }

        println!("Font data for everything");
        println!("{:#?}", all_font_data);
    }

    Ok(())
}

async fn get_font_data_from_page(crawler: &Crawler, base_url: &str) -> Result<Vec<FontData>> {
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
            Err(err) => {
                eprintln!("{}", err);
                None
            }
        })
        .collect();

    Ok(all_font_data)
}
