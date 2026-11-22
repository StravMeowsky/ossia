\version "2.26.0"
\header { title = "Single-staff scaffold" tagline = ##f }

music = \relative c' {
  \clef treble
  \key c \major
  \time 4/4
  c4 d e f |
  g4 a b c |
  \bar "|."
}

\score {
  \new Staff \music
  \layout { }
  \midi { }
}
