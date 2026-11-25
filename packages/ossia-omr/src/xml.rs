use crate::Result;
use ossia_types::{AppError, ContentHash, ErrorCode};
use quick_xml::{events::Event, Reader};
use std::{fs, path::Path};

#[derive(Debug)]
pub(crate) struct ValidatedOutput {
    pub output_hash: ContentHash,
}

pub(crate) fn collect(output: &Path) -> Result<ValidatedOutput> {
    let mut candidates = fs::read_dir(output)
        .map_err(storage)?
        .flatten()
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let name = entry.file_name().to_string_lossy().into_owned();
            (file_type.is_file() && name.to_ascii_lowercase().ends_with(".xml"))
                .then_some((name, entry.path()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0));
    let (name, input) = match candidates.as_slice() {
        [] => {
            return Err(failure(
                "omr-no-output",
                "Audiveris produced no MusicXML file",
            ))
        }
        [single] => single.clone(),
        many => {
            return Err(AppError {
                code: ErrorCode::ImportRefused,
                message: "Audiveris produced multiple MusicXML movements".into(),
                detail: Some(serde_json::json!({
                    "reason": "omr-multiple-movements",
                    "files": many.iter().map(|(name, _)| name).collect::<Vec<_>>(),
                })),
            })
        }
    };
    let bytes = fs::read(&input).map_err(storage)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| failure("omr-invalid-xml", "Audiveris output is not UTF-8 XML"))?;
    validate_xml(text)?;
    let result = output.join("result.xml");
    if name != "result.xml" {
        let temporary = output.join(".result.tmp");
        fs::write(&temporary, &bytes).map_err(storage)?;
        fs::rename(&temporary, &result).map_err(storage)?;
    }
    Ok(ValidatedOutput {
        output_hash: ContentHash::of(&bytes),
    })
}

fn validate_xml(text: &str) -> Result<()> {
    let text = text.trim_start_matches('\u{feff}');
    let mut reader = Reader::from_str(text);
    let mut saw_root = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(_)) | Ok(Event::Empty(_)) => saw_root = true,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(failure(
                    "omr-invalid-xml",
                    &format!("Audiveris output is not well-formed XML: {error}"),
                ))
            }
        }
    }
    if !saw_root {
        return Err(failure(
            "omr-invalid-xml",
            "Audiveris output has no XML root",
        ));
    }
    Ok(())
}

fn failure(reason: &str, message: &str) -> AppError {
    AppError {
        code: ErrorCode::CompileFailed,
        message: message.into(),
        detail: Some(serde_json::json!({"reason": reason})),
    }
}

fn storage(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::StorageError, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_multiple_movements() {
        let root = tempfile::tempdir().unwrap();
        assert!(collect(root.path()).unwrap_err().detail.unwrap()["reason"] == "omr-no-output");
        fs::write(root.path().join("a.xml"), "<score-partwise/>").unwrap();
        fs::write(root.path().join("b.xml"), "<score-partwise/>").unwrap();
        let error = collect(root.path()).unwrap_err();
        assert_eq!(error.detail.unwrap()["reason"], "omr-multiple-movements");
    }

    #[test]
    fn validates_one_xml_and_records_its_hash() {
        let root = tempfile::tempdir().unwrap();
        let bytes = b"<?xml version=\"1.0\"?><score-partwise/>";
        fs::write(root.path().join("score.xml"), bytes).unwrap();
        let output = collect(root.path()).unwrap();
        assert_eq!(output.output_hash, ContentHash::of(bytes));
        assert_eq!(fs::read(root.path().join("result.xml")).unwrap(), bytes);
    }
}
