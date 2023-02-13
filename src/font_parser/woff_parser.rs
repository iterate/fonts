use eyre::{eyre, Result};
use flate2::read::ZlibDecoder;
use std::io::Read;

use super::FontData;

pub fn parse_woff(content: &[u8]) -> Result<FontData> {
    let num_tables: u16 = u16::from_be_bytes(content[12..14].try_into().unwrap());

    let mut name_table_entry: Option<TableDirectoryEntry> = None;

    let table_directory_start: usize = 44;

    for i in 0..num_tables {
        // A TableDirectoryEntry is 20 bytes long
        let index: usize = (i * 20).into();
        let tag = std::str::from_utf8(
            &content[table_directory_start + index..table_directory_start + 4 + index],
        )?;

        if tag == "name" {
            name_table_entry = Some(
                content[table_directory_start + index..table_directory_start + 16 + index]
                    .to_owned()
                    .try_into()?,
            );
            break;
        }
    }

    let name_table_entry = match name_table_entry {
        Some(e) => e,
        None => return Err(eyre!("Could not find name table entry")),
    };

    let name_table: NameTable = convert_to_name_table(&content, &name_table_entry)?;

    let font_data: FontData = get_font_data(&name_table)?;

    Ok(font_data)
}

//https://github.com/pcwalton/rust-woff/blob/master/lib.rs
//https://github.com/hanikesn/woff2otf/blob/master/woff2otf.py

// https://www.w3.org/TR/WOFF/#OverallStructure
// https://www.w3.org/TR/WOFF2/#FileStructure
// https://learn.microsoft.com/nb-no/typography/opentype/spec/name
// https://learn.microsoft.com/en-us/typography/opentype/spec/otff
// https://docs.fileformat.com/font/ttf/

// Big-Endian: The most significant byte (the "big" end) of the data is places at the byte with the lowest address.

// Data types
// UInt32	32-bit (4-byte) unsigned integer in big-endian format
// UInt16	16-bit (2-byte) unsigned integer in big-endian format

// WOFFHeader
// 0-4      UInt32  signature	0x774F4646 'wOFF'
// 4-8      UInt32 	flavor	The "sfnt version" of the input font.
// 8-12     UInt32	length	Total size of the WOFF file.
// 12-14    UInt16	numTables	Number of entries in directory of font tables.
// 14-16    UInt16	reserved	Reserved; set to zero.
// 16-20    UInt32	totalSfntSize	Total size needed for the uncompressed font data, including the sfnt header, directory, and font tables (including padding).
// 20-22    UInt16	majorVersion	Major version of the WOFF file.
// 22-24    UInt16	minorVersion	Minor version of the WOFF file.
// 24-28    UInt32	metaOffset	Offset to metadata block, from beginning of WOFF file.
// 28-32    UInt32	metaLength	Length of compressed metadata block.
// 32-36    UInt32	metaOrigLength	Uncompressed size of metadata block.
// 36-40    UInt32	privOffset	Offset to private data block, from beginning of WOFF file.
// 40-44    UInt32	privLength	Length of private data block.

// WOFF TableDirectoryEntry
// 44-48    UInt32	tag	4-byte sfnt table identifier.
// 48-52    UInt32	offset	Offset to the data, from beginning of WOFF file.
// 52-56    UInt32	compLength	Length of the compressed data, excluding padding.
// 56-60    UInt32	origLength	Length of the uncompressed table, excluding padding.
// 60-64    UInt32	origChecksum	Checksum of the uncompressed table.

struct TableDirectoryEntry {
    // tag: String,
    offset: usize,
    comp_length: usize,
    orig_length: usize,
}

impl TryFrom<Vec<u8>> for TableDirectoryEntry {
    type Error = eyre::Report;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        // Assumes value has correct length
        let tag = std::str::from_utf8(&value[0..4])?;

        if tag != "name" {
            return Err(eyre!("Not woff name table entry"));
        }

        let offset: u32 = u32::from_be_bytes(value[4..8].try_into()?);
        let comp_length: u32 = u32::from_be_bytes(value[8..12].try_into()?);
        let orig_length: u32 = u32::from_be_bytes(value[12..16].try_into()?);

        Ok(TableDirectoryEntry {
            // tag: tag.to_owned(),
            offset: offset as usize,
            comp_length: comp_length as usize,
            orig_length: orig_length as usize,
        })
    }
}

//      Type 	    Name 	            Description
// 0-2  uint16 	    version 	        Table version number (=0).
// 2-4  uint16 	    count 	            Number of name records.
// 4-6  Offset16 	storageOffset 	    Offset to start of string storage (from start of table).
//      NameRecord 	nameRecord[count] 	The name records where count is the number of records.
//                  (Variable)          Storage for the actual string data.

#[derive(Debug)]
struct NameTable {
    // version: u16,
    // count: u16,
    offset: usize,
    records: Vec<NameRecord>,
    data: Vec<u8>,
}

// Type 	Name 	Description
// uint16 	platformID 	Platform ID.
// uint16 	encodingID 	Platform-specific encoding ID.
// uint16 	languageID 	Language ID.
// uint16 	nameID 	Name ID.
// uint16 	length 	String length (in bytes).
// Offset16 	stringOffset 	String offset from start of storage area (in bytes).

#[derive(Debug)]
struct NameRecord {
    // platform_id: u16,
    // encoding_id: u16,
    // language_id: u16,
    name_id: u16,
    length: usize,
    offset: usize,
}

fn convert_to_name_table(data: &[u8], entry: &TableDirectoryEntry) -> Result<NameTable> {
    let mut name_data = Vec::new();

    // decompress data with zlib decoder if comp_length != orig_length
    if entry.comp_length != entry.orig_length {
        let mut d = ZlibDecoder::new(&data[entry.offset..entry.offset + entry.comp_length])
            .take(entry.orig_length as u64);

        d.read_to_end(&mut name_data).unwrap();
    } else {
        name_data = data[entry.offset..entry.offset + entry.comp_length].to_owned();
    }

    // let version: u16 = u16::from_be_bytes(name_data[0..2].try_into().unwrap());

    let count: u16 = u16::from_be_bytes(name_data[2..4].try_into().unwrap());

    let offset: usize = u16::from_be_bytes(name_data[4..6].try_into().unwrap()).into();

    let name_records: Vec<NameRecord> = get_name_records(&name_data, count)?;

    Ok(NameTable {
        // version,
        // count,
        offset: offset as usize,
        records: name_records,
        data: name_data,
    })
}

fn get_name_records(data: &[u8], count: u16) -> Result<Vec<NameRecord>> {
    let mut records: Vec<NameRecord> = Vec::new();

    for i in 0..count {
        let index: usize = (i * 12).into();
        // let platform_id = u16::from_be_bytes(data[6 + index..8 + index].try_into()?);
        // let encoding_id = u16::from_be_bytes(data[8 + index..10 + index].try_into()?);
        // let language_id = u16::from_be_bytes(data[10 + index..12 + index].try_into()?);
        let name_id = u16::from_be_bytes(data[12 + index..14 + index].try_into()?);
        let length = u16::from_be_bytes(data[14 + index..16 + index].try_into()?);
        let offset = u16::from_be_bytes(data[16 + index..18 + index].try_into()?);

        records.push(NameRecord {
            // platform_id,
            // encoding_id,
            // language_id,
            name_id,
            length: length.into(),
            offset: offset.into(),
        })
    }

    Ok(records)
}

fn get_font_data(table: &NameTable) -> Result<FontData> {
    let offset: &usize = &table.offset;
    let records: &[NameRecord] = &table.records;
    let data = &table.data;

    let family: String = get_name_id_from_record(data, records, offset, 1)?;

    let subfamily = get_name_id_from_record(data, records, offset, 2)?;

    let identifier_record = get_name_id_from_record(data, records, offset, 3)?;

    let full_name_record = get_name_id_from_record(data, records, offset, 4)?;

    Ok(FontData {
        family_name: family,
        sub_family_name: subfamily,
        identifier: identifier_record,
        full_name: full_name_record,
    })
}

fn get_name_id_from_record(
    data: &[u8],
    records: &[NameRecord],
    table_offset: &usize,
    find_id: u16,
) -> Result<String> {
    records
        .into_iter()
        .find(|&record| record.name_id == find_id)
        .ok_or_else(|| eyre!("Unable to find font family record"))
        .map(|record| -> Result<String> {
            Ok(String::from_utf8(
                data[table_offset + record.offset..table_offset + record.offset + record.length]
                    .to_vec(),
            )
            .map_err(|_| eyre!("Unable to parse bytearray as utf-8"))?
            .replace("\0", ""))
        })?
}
