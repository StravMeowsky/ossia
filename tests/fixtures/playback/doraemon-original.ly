\version "2.26.0"

\header {
  title = "哆啦A梦（机器猫）"
  subtitle = "主题曲"
  tagline = ##f
}

\paper {
  #(set-paper-size "a4")
  ragged-bottom = ##f
  markup-system-spacing.basic-distance = #12
  system-system-spacing.basic-distance = #14
}

intro = {
  \time 4/4
  \key c \major
  \tempo 4 = 130
  \tuplet 3/2 { <d''\1 fis'\2 d'\3 d\4>8\arpeggio(^\markup { \circle { "P" } } <cis''\1>8) <b'\1>8 } \tuplet 3/2 { <a'\2>8 <b'\1>8(^\markup { \circle { "H" } } <cis''\1>8) } \tuplet 3/2 { <d''\1>8(^\markup { \circle { "P" } } <cis''\1>8) <b'\1>8 } \tuplet 3/2 { <a'\2 d'\3 d\4>8 <b'\1>8(^\markup { \circle { "H" } } <cis''\1>8) } |
  \tuplet 3/2 { <d''\1>8(^\markup { \circle { "P" } } <cis''\1>8) <b'\1>8 } \tuplet 3/2 { <a'\2>8 <b'\1>8(^\markup { \circle { "H" } } <cis''\1>8) } \tuplet 3/2 { <d''\1 fis'\2 d'\3 d\4>8\arpeggio(^\markup { \circle { "P" } } <cis''\1>8) <b'\1>8 } <a'\2>4 |
  \tuplet 3/2 { <e''\1 gis'\2 e'\3 e,\6>8(^\markup { \circle { "P" } } <d''\1>8) <cis''\1>8 } \tuplet 3/2 { <b'\2>8 <cis''\1>8(^\markup { \circle { "H" } } <d''\1>8) } \tuplet 3/2 { <e''\1>8(^\markup { \circle { "P" } } <d''\1>8) <cis''\1>8 } \tuplet 3/2 { <b'\2 e'\3 e,\6>8 <cis''\1>8(^\markup { \circle { "H" } } <d''\1>8) } | \break
  \tuplet 3/2 { <e''\1>8(^\markup { \circle { "P" } } <d''\1>8) <cis''\1>8 } \tuplet 3/2 { <b'\2>8 <cis''\1>8(^\markup { \circle { "H" } } <d''\1>8) } \tuplet 3/2 { <cis''\1 a'\2 e'\3 a,\5>8(^\markup { \circle { "H" } } <d''\1>8) <dis''\1>8 } <e''\1>4 |
  \tuplet 3/2 { <b'\1 g\3>8(^\markup { \circle { "H" } } <cis''\1>8) <d''\1>8 } \tuplet 3/2 { <g'\2 e\5>8(^\markup { \circle { "H" } } <a'\2>8) <b'\1>8 } \tuplet 3/2 { <a'\2 fis\5>8(^\markup { \circle { "H" } } <b'\2>8) <cis''\1>8 } \tuplet 3/2 { <fis'\2 d\4>8(^\markup { \circle { "H" } } <g'\2>8) <a'\2>8 } |
  \tuplet 3/2 { <g'\2 e\5>8(^\markup { \circle { "H" } } <a'\2>8) <b'\1>8 } \tuplet 3/2 { <e'\1 cis\5>8(^\markup { \circle { "H" } } <fis'\1>8) <g'\1>8 } \tuplet 3/2 { <fis'\1 d\4>8(^\markup { \circle { "H" } } <g'\1>8) <a'\1>8 } \tuplet 3/2 { <d'\2 b,\5>8(^\markup { \circle { "H" } } <e'\2>8) <fis'\1>8 } | \break
  \tuplet 3/2 { <e''\1 gis'\2 e'\3 e,\6>8(^\markup { \circle { "P" } } <dis''\1>8) <d''\1>8 } \tuplet 3/2 { <cis''\1>8 <c''\2>8 <b'\2>8 } \tuplet 3/2 { <ais'\2>8 <a'\2>8 <gis'\2>8 } \tuplet 3/2 { <g'\3>8 <fis'\3>8 <f'\3>8 } |
  <e'\3>4 <a,\5>4 <a,\5>8.(^\markup { \circle { "H" } } <b,\5>16) <cis\5>4 | \break
}

main = {
  <a\3 d\4>8. <d'\2>16 <d'\2 cis\5>8. <fis'\1>16 <b'\1 fis'\2 b,\6>8. <fis'\2>16 <a'\1 e'\2 a,\5>4 |
  <a'\1 fis'\2 d\4>8. <b'\1>16 <a'\1 fis'\2 d\4>8. <fis'\2>16 <g'\2 e\5>8. <fis'\2>16 <e'\2 cis'\3 a,\5>4 |
  <b\3 g\4 e,\6>8. <e'\2 b\3>16 <e'\2 b\3 e,\6>8. <g'\1 e'\2>16 <cis''\1 e'\3 a,\5>8. <cis''\1 e'\3>16 <b'\1 d'\3 e,\6>8. <a'\1 cis'\3>16 |
  <g'\1 b\2 g\3>4 <g'\1 b\2 b,\5>8. <fis'\1>16 <b\2 g\3>8. <cis'\2>16 <fis\4>8 <d'\2>8 |
  <e'\1 cis'\2 a,\5>4 <e\4>8 <a\3>8 <a,\5>8. <a,\5>16(^\markup { \circle { "H" } } <b,\5>8) <cis\5>8 | \break
  <a\3 d\4>8. <d'\2>16 <d'\2 cis\5>8. <fis'\1>16 <b'\1 fis'\2 b,\6>8. <fis'\2>16 <a'\1 e'\2 a,\5>4 |
  <a'\1 fis'\2 d\4>8. <b'\1>16 <a'\1 fis'\2 d\4>8. <fis'\2>16 <g'\2 e\5>8. <fis'\2>16 <e'\2 cis'\3 a,\5>4 |
  <b\3 g\4 e,\6>8. <e'\2 b\3>16 <e'\2 b\3 e,\6>8. <g'\1 e'\2>16 <cis''\1 e'\3 a,\5>8. <cis''\1 e'\3>16 <b'\1 d'\3 e,\6>8. <a'\1 cis'\3>16 |
  <g'\1 b\2 g\3>4 <g'\1 b\2 b,\5>8. <fis'\1>16 <b\2 g\3>8. <cis'\2>16 <fis\4>8 <d'\2>8 |
  <d'\2 a\3 d\4>4 <d\4>8. <d\4>16 <e\4>8. <fis\4>16 <cis'\2>4 | \break
  <b'\1 d'\3 g\4>4 <b'\1 d'\3 g\4>8. <a'\1>16 \tuplet 3/2 { <g'\1 d'\2 d\4>8(^\markup { \circle { "H" } } <a'\1>8)(^\markup { \circle { "S" } } <b'\1>8) } <a'\1 d\4>4 |
  <e'\1 gis\3 e,\6>8. <fis'\1 a\3>16 <gis'\1 b\3 e,\6>8. <e'\1>16 <a'\1 e'\2 cis'\3 a,\5>4 <a,\5>4 |
  <b\3 g\4 e,\6>4 <a\3 e\4 a,\5>4 <e\4 e,\6>8. <fis\4>16 <gis\3 e,\6>8. <e\4>16 |
  <a\3 e\4 a,\5>4 <a,\5>8 <e\4>8 <e\4 a,\5>8 <a\3>8 <a\3 a,\5>8 <g\3>8 | \break
  <b'\1 d'\3 g\4>4 <a'\1 d'\3>4 <g'\2 d'\3 g\4>4 r4 |
  <e'\2 cis'\3 a,\5>4 <cis''\1 e'\3 a,\5>8. <b'\1 d'\3>16 <a'\1 cis'\3 a,\5>8. <b'\1 d'\3>16 <a'\1 cis'\3 a,\5>8. <g'\1 b\3>16 |
  <a,\5>4 <a'\1 e'\2 cis'\3>8\arpeggio <b'\1>8 <fis'\1>4 <a,\5>8 <e'\1>8 |
  <d'\2 a\3 d\4>4 <a,\5>4 <d\4>4 <a,\5>4 | \pageBreak
}

endingOne = {
  <d'\2 a\3 d\4>8. <d'\2>16 <cis'\2 a,\5>8. <cis'\2>16 <b\3 d\4>8. <b\3>16 <a\3 a,\5>8. <a\3>16 |
  <b\3 d\4>8. <b\3>16 <cis'\2 a,\5>8. <cis'\2>16 <d'\2 d\4>4 \tuplet 3/2 { <d'\2 a,\5>8(^\markup { \circle { "P" } } <cis'\2>8 <d'\2>8) } |
  <e'\1 e,\6>8 <b\2 gis\3 e\4>8 <b\2 gis\3 e\4 b,\5>4 <e'\1 e,\6>8 <b\2 gis\3 e\4>8 <b\2 gis\3 e\4 b,\5>4 |
  <b\2 e,\6>8. <cis'\2>16 <d'\2 gis\3 e\4 b,\5>8. <b\2>16 <e'\1 e,\6>4 <a\3 e\4 a,\5>4 | \break
  <d'\2 a\3 d\4>8. <d'\2>16 <cis'\2 a,\5>8. <cis'\2>16 <b\3 d\4>8. <b\3>16 <a\3 a,\5>8. <d'\2>16 |
  <fis'\1 d'\2 d\4>8 <fis'\1 d'\2>8 \tuplet 3/2 { <fis'\1 d'\2 fis\4>8(^\markup { \circle { "P" } } <e'\1>8 <fis'\1>8) } <g'\1 d'\2 g\3>4 <b,\5>4 |
  <b\3 g\4 e,\6>8 <ais\3 fis\4>8 <b\3 g\4 e,\6>8 <d'\2 b\3>8 <cis'\2 a\3 a,\5>8 <d'\2 b\3>8 <e'\1 cis'\2 a,\5>8 <cis'\2 a\3>8 |
  <d'\2 a\3 d\4>4 <a,\5>4 <d\4>8. <a,\5>16(^\markup { \circle { "H" } } <b,\5>8) <cis\5>8 |
}

endingTwo = {
  \tuplet 3/2 { <fis'\1 a\3 d\4>8(^\markup { \circle { "S" } } <gis\3>8 <fis'\1 a\3>8) } <d'\2 a,\5>8 <a\3>8 \tuplet 3/2 { <g'\1 a\3 d\4>8(^\markup { \circle { "S" } } <fis'\1 gis\3>8 <g'\1 a\3>8) } <d'\2 a,\5>8 <a\3>8 |
  \tuplet 3/2 { <g'\1 e,\6>8(^\markup { \circle { "P" } } <fis'\1>8 <g'\1>8) } <e'\1 a,\5>8 <a\3 e\4>8 <a\3 e\4 a,\5>8 <a,\5>8 <a,\5>16 r8. |
  \tuplet 3/2 { <g'\1 e,\6>8(^\markup { \circle { "P" } } <fis'\1>8 <g'\1>8) } <e'\1 a,\5>8 <a\3 e\4>8 \tuplet 3/2 { <g'\1 e,\6>8(^\markup { \circle { "P" } } <fis'\1>8 <g'\1>8) } <e'\1 a,\5>8 <a\3 e\4>8 |
  \tuplet 3/2 { <fis'\1 d'\2 a\3 d\4>8 <g'\1>8 <gis'\1>8 } <a'\1 a,\5>4 <d'\2 a\3 d\4>8 <d\4>8 <d\4>16 r8. | \break
  \tuplet 3/2 { <b'\1 d'\3 g\4>8(^\markup { \circle { "P" } } <ais'\1>8 <b'\1>8) } \tuplet 3/2 { <ais'\1 d'\3 d\5>8(^\markup { \circle { "P" } } <b'\1>8 <ais'\1>8) } <b'\1 d'\3 g\4>8 <cis''\1>8 <d''\1 d\4>8 <b'\1>8 |
  \tuplet 3/2 { <a'\2 fis'\3 d\4>8(^\markup { \circle { "P" } } <gis'\2 e'\3>8 <a'\2 fis'\3>8) } \tuplet 3/2 { <gis'\2 e'\3 a,\5>8(^\markup { \circle { "P" } } <a'\2 fis'\3>8 <gis'\2 e'\3>8) } <a'\2 fis'\3 d\4>8 <cis''\1>8 <d''\1 a,\5>8 <a'\2>8 |
  \tuplet 3/2 { <cis''\1 e'\3 a,\5>8(^\markup { \circle { "S" } } <dis'\3>8 <cis''\1 e'\3>8) } \tuplet 3/2 { <c''\1 dis'\3 e,\6>8(^\markup { \circle { "S" } } <e'\3>8 <c''\1 dis'\3>8) } <cis''\1 e'\3 a,\5>8 <e''\1>8 \tuplet 3/2 { <e''\1 a,\5>8(^\markup { \circle { "P" } } <d''\1>8 <cis''\1>8) } |
  <d''\1 a'\2 d\4>4 <a,\5>4 <d\4>8. <a,\5>16(^\markup { \circle { "H" } } <b,\5>8) <cis\5>8 |
}

endingThree = {
  <d'\2 a\3 d\4>4 <a,\5>4 <d\4>8 <d'\2>8 <e'\1 fis,\6>8 <fis'\1>8 |
  \tuplet 3/2 { <g'\1 d'\2 g,\6>8(^\markup { \circle { "P" } } <fis'\1>8 <g'\1>8) } <fis'\1 d'\2 d\4>8 <g'\1>8 <fis'\1 d'\2 g,\6>8 <e'\1>8 <d\4>8 <f'\1>8 |
  \tuplet 3/2 { <fis'\1 a\3 a,\5>8(^\markup { \circle { "S" } } <gis\3>8 <fis'\1 a\3>8) } <f'\1 gis\3 d\4>8 <fis'\1 a\3>8 <e'\1 g\3 a,\5>8 <d'\2 fis\4>8 <d\5>4 |
  <b\2 g\3 g,\6>4 <b\2 g\3 g,\6>4 <cis'\2 a\3 a,\5>8. <d'\2>16 <e'\1 cis'\2 a,\5>4 | \break
  <b\2 g\3 g,\6>4 <b\2 g\3 g,\6>4 <cis'\2 a\3 a,\5>8. <d'\2>16 <e'\1 cis'\2 a,\5>8 <d'\2 a\3 d\4>8 |
  r8 \deadNotesOn <e'\1 b\2 g\3 d\4 a,\5 e,\6>16 \deadNotesOff \deadNotesOn <e'\1 b\2 g\3 d\4 a,\5 e,\6>16 \deadNotesOff \deadNotesOn <e'\1 b\2 g\3 d\4 a,\5 e,\6>4 \deadNotesOff <d''\1 fis'\2 d'\3 d\4>4~\arpeggio <d''\1 fis'\2 d'\3 d\4>4~\arpeggio |
  <d''\1 fis'\2 d'\3 d\4>1\arpeggio |
  \bar "|."
}

\score {
  \new TabStaff {
    \tabFullNotation
    \arpeggioArrowUp
    \set doubleSlurs = ##f
    \set Staff.midiInstrument = "acoustic guitar (nylon)"
    \intro
    \repeat volta 3 { \main }
    \alternative {
      \endingOne
      \endingTwo
      \endingThree
    }
  }
  \layout { }
  \midi { }
}
