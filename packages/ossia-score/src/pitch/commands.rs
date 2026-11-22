fn property_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "set"
            | "unset"
            | "override"
            | "revert"
            | "omit"
            | "hide"
            | "once"
            | "temporary"
            | "permanent"
            | "default"
            | "undo"
    )
}

pub(crate) fn known_command(cmd: &str) -> bool {
    property_command(cmd)
        || matches!(
            cmd,
            "relative"
                | "absolute"
                | "transpose"
                | "key"
                | "major"
                | "minor"
                | "time"
                | "numericTimeSignature"
                | "clef"
                | "tempo"
                | "ottava"
                | "partial"
                | "bar"
                | "barNumberCheck"
                | "change"
                | "break"
                | "noBreak"
                | "pageBreak"
                | "noPageBreak"
                | "voiceOne"
                | "voiceTwo"
                | "voiceThree"
                | "voiceFour"
                | "oneVoice"
                | "stemUp"
                | "stemDown"
                | "stemNeutral"
                | "tieUp"
                | "tieDown"
                | "slurUp"
                | "slurDown"
                | "slurNeutral"
                | "autoBeamOff"
                | "autoBeamOn"
                | "tuplet"
                | "tupletSpan"
                | "times"
                | "fermata"
                | "trill"
                | "arpeggio"
                | "deadNotesOn"
                | "deadNotesOff"
                | "glissando"
                | "harmonic"
                | "laissezVibrer"
                | "breathe"
                | "bold"
                | "mp"
                | "mf"
                | "ppp"
                | "pp"
                | "p"
                | "f"
                | "ff"
                | "fff"
                | "sfz"
                | "fp"
                | "cresc"
                | "decresc"
                | "dim"
                | "<"
                | ">"
                | "!"
                | "("
                | ")"
                | "["
                | "]"
                | "\\"
                | "compressMMRests"
                | "mergeDifferentlyDottedOn"
                | "mergeDifferentlyHeadedOn"
                | "tweak"
                | "mark"
                | "square"
        )
}

pub(super) fn safe_stem_direction_alias(source: &str, command: &str) -> bool {
    matches!(command, "U" | "D")
        && source.lines().any(|line| {
            let line = line.trim();
            line.starts_with(&format!("{command} ="))
                && line.contains("\\tweak Stem.direction")
                && line.contains("\\etc")
        })
}
