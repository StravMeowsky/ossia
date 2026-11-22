\version "2.26.0"
\header { title = "Guitar tab scaffold" tagline = ##f }
\paper { markup-system-spacing.basic-distance = #12 }

music = {
  \time 4/4
  \key c \major
  c4\5 d4\5 e4\4 f4\4 |
  g4\3 a4\3 b4\2 c'4\2 |
}

\score {
  \new TabStaff {
    \set Staff.midiInstrument = "acoustic guitar (nylon)"
    \music
  }
  \layout { }
  \midi { }
}
