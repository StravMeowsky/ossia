const TERMS: &[(&str, &str)] = &[
    ("相对音高", "relative"),
    ("绝对音高", "absolute"),
    ("八度", "octave"),
    ("时值", "duration"),
    ("音阶", "scale"),
    ("谱号", "clef"),
    ("连音", "tie"),
    ("排版", "layout"),
    ("六线谱", "tablature"),
    ("吉他", "guitar"),
    ("指法", "fingering"),
    ("变调夹", "capo"),
];

pub(super) fn expand(query: &str) -> String {
    let mut expanded = query.to_owned();
    for (from, to) in TERMS {
        if expanded.contains(from) {
            expanded = expanded.replace(from, &format!(" {to} "));
        }
    }
    expanded.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
#[path = "../../tests/unit/documentation/terms.rs"]
mod tests;
