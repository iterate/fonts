use std::{fs, str::FromStr};

use eyre::{eyre, Result};

use super::woff_parser::parse_woff;

enum FontSignature {
    Woff,
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
            _ => Err(eyre!("Signature variant not found!")),
        }
    }
}

#[derive(Debug)]
pub struct FontData {
    pub family_name: String,
    pub sub_family_name: String,
    pub identifier: String,
    pub full_name: String,
}

impl FontData {
    pub fn from_filepath(filepath: &str) -> Result<FontData> {
        // reads into 1-byte array
        let content = fs::read(filepath)?;

        let signature: FontSignature = content.as_slice().try_into()?;

        match signature {
            FontSignature::Woff => return parse_woff(&content),
        };
    }
}
