use eyre::Result;
use url::Url;

use crate::{crawler::Crawler, font_parser::FontData};

mod crawler;
mod font_parser;

#[tokio::main]
async fn main() -> Result<()> {
    // let font_data = FontData::from_filepath("test_font_1.woff")?;
    // println!("family name: {}", &font_data.family_name);
    // println!("sub name: {}", &font_data.sub_family_name);
    // println!("full name: {}", &font_data.full_name);

    let args: Vec<String> = std::env::args().collect();

    let crawler: Crawler = Crawler::new()?;
    let font_urls = crawler
        .get_font_urls_from_page(
            Url::parse(args.get(1).expect("Send in url"))
                .expect("Could not parse url")
                .as_str(),
        )
        .await?;

    println!("{:#?}", font_urls);

    Ok(())
}
