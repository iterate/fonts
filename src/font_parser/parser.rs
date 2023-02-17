use std::str::FromStr;

use eyre::{eyre, Result};

use super::woff_parser::parse_woff;

#[derive(Debug)]
enum FontSignature {
    Woff,
    Woff2,
}

impl TryInto<FontSignature> for &[u8] {
    type Error = eyre::ErrReport;

    fn try_into(self) -> Result<FontSignature> {
        std::str::from_utf8(&self[0..4])?.parse()
    }
}

impl FromStr for FontSignature {
    type Err = eyre::ErrReport;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "wOFF" => Ok(FontSignature::Woff),
            "wOF2" => Ok(FontSignature::Woff2),
            _ => Err(eyre!("Signature variant for {} not found!", s)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FontData {
    pub family_name: String,
    pub sub_family_name: String,
    pub identifier: String,
    pub full_name: String,
}

impl FontData {
    #[cfg(test)] // only used in testing for now
    fn from_filepath(filepath: &str) -> Result<FontData> {
        // reads into 1-byte array

        use std::fs;
        let content = fs::read(filepath)?;

        return FontData::from_bytes(&content);
    }

    pub fn from_bytes(content: &Vec<u8>) -> Result<FontData> {
        let signature: FontSignature = content.as_slice().try_into()?;

        match signature {
            FontSignature::Woff => return parse_woff(&content),
            FontSignature::Woff2 => return Err(eyre!("woff2 parsing not implemented!")),
        };
    }
}

#[cfg(test)]
mod tests {

    use eyre::Result;

    use super::FontData;

    #[test]
    fn get_font_data_from_woff() -> Result<()> {
        let font_data = FontData::from_filepath("test_files/test_font_1.woff")?;

        let expected_results = FontData {
            family_name: "Univers Else".to_owned(),
            sub_family_name: "Regular".to_owned(),
            identifier: "webfont".to_owned(),
            full_name: "Univers Else Regular".to_owned(),
        };

        assert_eq!(font_data, expected_results);

        let font_data = FontData::from_filepath("test_files/test_font_2.woff")?;

        let expected_results = FontData {
            family_name: "Adieu".to_owned(),
            sub_family_name: "Regular".to_owned(),
            identifier: "3.100;UKWN;Adieu-Regular".to_owned(),
            full_name: "Adieu Regular".to_owned(),
        };

        assert_eq!(font_data, expected_results);

        Ok(())
    }
}
