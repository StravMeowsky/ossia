use crate::AppError;
use quick_xml::{
    events::{BytesStart, Event},
    Reader, XmlVersion,
};
use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};
use zip::ZipArchive;

const CONTAINER_PATH: &str = "META-INF/container.xml";
const MAX_CONTAINER_BYTES: u64 = 1024 * 1024;
const MAX_MUSICXML_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 4096;

pub(crate) fn extract(input: &[u8]) -> Result<Vec<u8>, AppError> {
    let mut archive = ZipArchive::new(Cursor::new(input))
        .map_err(|_| AppError::invalid("compressed MusicXML is not a valid MXL ZIP archive"))?;
    if archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(AppError::invalid(
            "compressed MusicXML contains too many archive entries",
        ));
    }

    let container = read_entry(&mut archive, CONTAINER_PATH, MAX_CONTAINER_BYTES)?;
    let rootfile = rootfile_path(&container)?;
    read_entry(&mut archive, &rootfile, MAX_MUSICXML_BYTES)
}

fn read_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
    limit: u64,
) -> Result<Vec<u8>, AppError> {
    let entry = archive
        .by_name(name)
        .map_err(|_| AppError::invalid(format!("compressed MusicXML is missing {name}")))?;
    if entry.size() > limit {
        return Err(AppError::invalid(format!(
            "compressed MusicXML entry exceeds {} MiB",
            limit / (1024 * 1024)
        )));
    }
    let mut data = Vec::with_capacity(entry.size().min(limit) as usize);
    entry
        .take(limit + 1)
        .read_to_end(&mut data)
        .map_err(|_| AppError::invalid(format!("compressed MusicXML entry {name} is damaged")))?;
    if data.len() as u64 > limit {
        return Err(AppError::invalid(format!(
            "compressed MusicXML entry exceeds {} MiB",
            limit / (1024 * 1024)
        )));
    }
    Ok(data)
}

fn rootfile_path(container: &[u8]) -> Result<String, AppError> {
    let mut reader = Reader::from_reader(container);
    let mut rootfile = None;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) | Ok(Event::Empty(element)) => {
                if element.local_name().as_ref() == "rootfile" {
                    let path = rootfile_attribute(&element)?;
                    if rootfile.replace(path).is_some() {
                        return Err(AppError::invalid(
                            "compressed MusicXML contains multiple root files",
                        ));
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(_) => {
                return Err(AppError::invalid(
                    "compressed MusicXML has a malformed container",
                ))
            }
        }
    }
    rootfile.ok_or_else(|| AppError::invalid("compressed MusicXML has no root file"))
}

fn rootfile_attribute(element: &BytesStart<'_>) -> Result<String, AppError> {
    let mut path = None;
    for attribute in element.attributes() {
        let attribute = attribute
            .map_err(|_| AppError::invalid("compressed MusicXML has a malformed container"))?;
        if attribute.key.local_name().as_ref() == "full-path" {
            path = Some(
                attribute
                    .normalized_value(XmlVersion::Implicit1_0)
                    .map_err(|_| {
                        AppError::invalid("compressed MusicXML has a malformed container")
                    })?
                    .into_owned(),
            );
        }
    }
    match path {
        Some(path) if !path.is_empty() && !Path::new(&path).is_absolute() => Ok(path),
        _ => Err(AppError::invalid(
            "compressed MusicXML has no valid root file path",
        )),
    }
}

#[cfg(test)]
#[path = "../tests/unit/mxl.rs"]
mod tests;
