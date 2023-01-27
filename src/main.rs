use eyre::Result;

use crate::font_parser::FontData;

mod font_parser;

fn main() -> Result<()> {
    let font_data = FontData::from_filepath("test_font_1.woff")?;
    println!("family name: {}", &font_data.family_name);
    println!("sub name: {}", &font_data.sub_family_name);
    println!("full name: {}", &font_data.full_name);

    Ok(())
}
