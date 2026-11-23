use ossia_score::{Mode, TokenKind};

fn parse_version(value: &str) -> Option<[u32; 3]> {
    let mut parts = value.split('.');
    let version = [
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ];
    parts.next().is_none().then_some(version)
}

fn declared_version(source: &str) -> Result<([u32; 3], String), String> {
    let tokens: Vec<_> = ossia_score::tokenize(source)
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Ws | TokenKind::Comment))
        .collect();
    let mut found: Option<([u32; 3], String)> = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.mode != Mode::Top || token.kind != TokenKind::Cmd || token.raw != "\\version" {
            continue;
        }
        let value = tokens
            .get(index + 1)
            .filter(|value| value.kind == TokenKind::String)
            .and_then(|value| value.raw.strip_prefix('"')?.strip_suffix('"'))
            .ok_or_else(|| {
                "Invalid LilyPond version declaration; use \\version \"2.24.0\" or newer."
                    .to_string()
            })?;
        let version = parse_version(value).ok_or_else(|| {
            format!("Invalid LilyPond version {value}; use a three-part version such as 2.24.0.")
        })?;
        if found.as_ref().is_some_and(|(prior, _)| *prior != version) {
            return Err(
                "Conflicting LilyPond version declarations; provide one consistent version.".into(),
            );
        }
        found = Some((version, value.into()));
    }
    found.ok_or_else(|| "Missing LilyPond version declaration; add a valid \\version declaration after checking the source version.".to_string())
}

/// Check explicit top-level LilyPond version declarations in imported files.
pub(crate) fn check_version(source: &str, engine_version: &str) -> Result<String, String> {
    let (version, value) = declared_version(source)?;
    if version < [2, 24, 0] {
        return Err(format!("LilyPond {value} is older than the minimum supported version 2.24.0. Migrate a copy using convert-ly and check the result before importing; changing only the version declaration is not migration."));
    }
    let engine = parse_version(engine_version)
        .ok_or_else(|| format!("Cannot determine bundled LilyPond version: {engine_version}"))?;
    if version > engine {
        return Err(format!(
            "LilyPond {value} is newer than the bundled engine {engine_version}."
        ));
    }
    Ok(value)
}

pub(crate) fn check_migration_input(source: &str, engine_version: &str) -> Result<(), String> {
    let (version, value) = declared_version(source)?;
    let engine = parse_version(engine_version)
        .ok_or_else(|| format!("Cannot determine bundled LilyPond version: {engine_version}"))?;
    if version > engine {
        return Err(format!(
            "LilyPond {value} is newer than the bundled engine {engine_version}."
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_version;

    #[test]
    fn checks_only_real_top_level_declarations() {
        for source in [
            "\\version \"2.14.2\"",
            "\\version \"2.23.99\"",
            "{ c'1 }",
            "\\version 2.24",
            "\\version \"2.24.0\" \\version \"2.25.0\"",
            "\\version \"2.27.0\"",
        ] {
            assert!(check_version(source, "2.26.0").is_err(), "{source}");
        }
        for source in [
            "\\version \"2.24.0\"",
            "\\version \"2.26.0\"",
            "%{ \\version \"2.14.0\" %}\n\\version \"2.24.1\"",
            "title = \"\\\\version 2.14.0\"\n\\version \"2.24.0\"",
        ] {
            assert!(check_version(source, "2.26.0").is_ok(), "{source}");
        }
    }
}
