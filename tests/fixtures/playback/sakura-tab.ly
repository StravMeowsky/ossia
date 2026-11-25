\version "2.26.0"

\header {
  title = "樱花草"
  subtitle = "Capo. 2 fret"
  composer = "东尼指弹"
  tagline = ""
}

\paper {
  top-margin = 13\mm
  bottom-margin = 12\mm
  left-margin = 12\mm
  right-margin = 12\mm
  ragged-last-bottom = ##f
  system-system-spacing.basic-distance = #13
  system-system-spacing.minimum-distance = #10
  markup-system-spacing.basic-distance = #8
}

music = {
  \numericTimeSignature
  \time 4/4
  \tempo "Moderate" 4 = 90
  \omit Staff.Clef

  <g, c'>8^\markup { \italic \small "let ring" } e'8 <e d'>8 e'8 <g, c'>8 e'8 <d d'>4 |
  <a, c'>8 e'8 <e d'>8 e'8 <g, c'>8 d8 g8 d8 |
  <f, c'>8 e'8 <f d'>8 e'8 <g, e'>8 d8 g8 e'16 d'16 |
  <c c'>8 e8 f8 g8 e4 e'8 e16 d'16 |

  \break
  <g, c'>8^\markup { \italic \small "let ring" } g8 <e c'>8 e'8 <g, d'>8 d16 g16 e'8 d16 d'16 |
  <a, c'>8 e8 a8 c'8 <g, b>8 d16 g16 b8 c'16 <b a>16 |
  f,8 f8 b8 a16 g16 e,8 b,8 e8 c'16 c'16 |

  \break
  <f' d>8^\markup { \italic \small "let ring" } e'8 <a d'>8 c'16 e'16 <g, d'>8 d16 g16 e'8 d16 d'16 |
  <c c'>8 g8 <e c'>8 e'8 <g, d'>8 d16 g16 e'8 d16 d'16 |
  <a, c'>8 e8 a8 c'8 <g, b>8 d16 g16 b8 c'16 <b a>16 |

  \break
  f,8^\markup { \italic \small "let ring" } f8 b8 a16 g16 e,8 b,8 e8 c'16 c'16 |
  <f' d>8 e'8 <a d'>8 c'16 e'16 <g, d'>8 d8 g8 d16 e'16 |
  <a, c'>8 e8 a8 c'16 d'16 <f, c'>8 f8 a8 c'16 a16 |

  \break
  <c g>8^\markup { \italic \small "let ring" } c'8 <e e'>8 e'16 f'16 <g, d'>8 d8 g8 e'8 |
  <a, c'>8 e8 a8 c'16 d'16 <f, c'>8 f8 a8 c'16 a16 |
  <c g>8 c'8 <e e'>8 e'16 f'16 <g, d'>8 d16 g16 g'8 a'16 g'16 |

  \break
  <e' g,>8^\markup { \italic \small "let ring" } g'8 <g' e>8 e'8 <g, d'>8 d16 g16 g'8 a'16 g'16 |
  <e' e,>8 g'8 <g' e>8 c'16 d'16 <c' f,>8 f8 <b, c'>8 d'16 e'16 |
  c8 e16 g16 g'8 e'16 d'16 g,8 d16 g16 g'8 e'8 |
}

\score {
  \new TabStaff \with {
    \tabFullNotation
    \override TabNoteHead.whiteout = ##t
    \override TabNoteHead.whiteout-color = #white
  } {
    \set Staff.midiInstrument = "acoustic guitar (nylon)"
    \music
  }
  \layout { }
  \midi { }
}
