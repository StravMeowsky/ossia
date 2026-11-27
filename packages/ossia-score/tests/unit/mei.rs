use super::*;

fn mei(source: &str) -> String {
    source_to_mei(source).expect("mei")
}

#[test]
fn piano_score_has_two_staves_with_measures() {
    let source = r#"\version "2.26.0"
\language "english"
upper = \relative c'' { \clef treble \key g \major \time 3/4 g4 fs8( e) d4 | b'2. \fermata | }
lower = \relative c { \clef bass \key g \major \time 3/4 <g b d>2. | g,2.~ | }
\score { \new PianoStaff << \new Staff \upper \new Staff \lower >> \layout { } }
"#;
    let out = mei(source);
    assert_eq!(out.matches("<measure ").count(), 2, "{out}");
    assert!(out.contains(r#"<staffGrp symbol="brace""#));
    assert!(out.contains(r#"<staffDef n="2" lines="5"><clef shape="F" line="4"/>"#));
    assert!(out.contains(r#"keysig="1s" meter.count="3" meter.unit="4""#));
    assert!(
        out.contains(r#"pname="f" oct="4" accid.ges="s" dur="8""#),
        "{out}"
    );
    assert!(out.contains("<slur "));
    assert!(out.contains("<fermata "));
    assert!(out.contains("<chord "));
    // The first g of the upper staff is selectable by its source offset.
    let g = source.find("g4 fs8").unwrap();
    assert!(out.contains(&format!(r#"xml:id="o{g}""#)));
}

#[test]
fn voices_tuplets_and_pickups() {
    let source = r#"music = \relative c'' {
  \time 2/4 \partial 8 g8 |
  << { c4 d } \\ { e,2 } >> |
  \tuplet 3/2 { c'8 d e } f4 |
  R2*2 |
}
\score { \new Staff \music }
"#;
    let out = mei(source);
    assert!(out.contains(r#"<measure n="1" metcon="false">"#), "{out}");
    assert_eq!(out.matches("<measure ").count(), 5, "{out}");
    assert!(out.contains(r#"<layer n="2">"#));
    assert!(out.contains(r#"<tuplet num="3" numbase="2">"#));
    assert_eq!(out.matches("<mRest").count(), 2);
}

#[test]
fn accidentals_follow_the_key_and_the_measure() {
    let source = r#"m = \relative c'' { \key f \major bes4 b b bes | bes1 }
\score { \m }
"#;
    let out = mei(source);
    // B flat is in the key, the first B natural shows, the second does not, then flat again.
    assert_eq!(out.matches(r#"accid="n""#).count(), 1, "{out}");
    assert_eq!(out.matches(r#"accid="f""#).count(), 1, "{out}");
}

/// Write MEI for every score in `OSSIA_MEI_SCORES` next to it.
#[test]
#[ignore = "needs a folder of scores"]
fn dump_real_scores() {
    let Ok(dir) = std::env::var("OSSIA_MEI_SCORES") else {
        return;
    };
    for path in std::fs::read_dir(dir).unwrap() {
        let path = path.unwrap().path();
        if path.extension().is_none_or(|e| e != "ly") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let started = std::time::Instant::now();
        match source_to_mei(&source) {
            Ok(mei) => {
                eprintln!("MEI {} {:?}", path.display(), started.elapsed());
                std::fs::write(path.with_extension("mei"), mei).unwrap();
            }
            Err(e) => eprintln!("MEI {} failed: {}", path.display(), e.message),
        }
    }
}
