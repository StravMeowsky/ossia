\version "2.26.0"
\header { title = "Piano scaffold" tagline = ##f }
\score {
  \new PianoStaff <<
    \new Staff {
      \clef treble
      \key c \major
      \time 4/4
      \relative c' { c4 d e f | g a b c | }
    }
    \new Staff {
      \clef bass
      \key c \major
      \time 4/4
      R1 | R1 |
    }
  >>
  \layout { }
  \midi { }
}
