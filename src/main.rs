use eyre::Result;
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

    let base_url: String = Url::parse(args.get(1).expect("Send in url"))
        .expect("Could not parse url")
        .as_str()
        .to_owned();

    let crawler: Crawler = Crawler::new()?;
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

    println!("Font data for {}", base_url);
    println!("{:#?}", all_font_data);

    Ok(())
}
