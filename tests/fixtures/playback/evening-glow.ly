\version "2.26.0"
\header { title = "晚风里的微光" subtitle = "温柔地，歌唱般" composer = "原创旋律" tagline = ##f }

music = \relative c' {
  \clef treble
  \key g \major
  \time 6/8
  \tempo 4. = 60
  d8^\markup { "G" }\p( g b) a( g fis) |
  e8^\markup { "Em" }( g b) d4( b8) |
  c8^\markup { "C" }( b a) g( fis e) |
  fis4.^\markup { "D" } d4 r8 \break |
  d8^\markup { "G" }( g b) a( g fis) |
  e8^\markup { "Em" }( g b) d4( e8) |
  d8^\markup { "C" }( b a) b( g fis) |
  g4.^\markup { "G" } g4 r8 \break |
  b8^\markup { "Em" }\mp( d e) d( b a) |
  c8^\markup { "C" }( e fis) e4( d8) |
  b8^\markup { "Gmaj7" }( d g) fis( e d) |
  e4.^\markup { "Em7" } d4 r8 \break |
  c8^\markup { "C" }\p( b a) b( g e) |
  a8^\markup { "Am7" }( g fis) g( e d) |
  e8^\markup { "D7" }( fis g) a( fis d) |
  g4.^\markup { "G" }~ g4\fermata r8 \bar "|."
}
accompaniment = {
  \clef bass
  \key g \major
  \time 6/8
  g,8\p d g b g d |
  e,8 b, e g e b, |
  c8 g c' e' c' g |
  d8 a d' fis' d' a |
  g,8 d g b g d |
  e,8 b, e g e b, |
  c8 g c' e' c' g |
  g,8 d g b g d |
  e,8\mp b, e g b g |
  c8 g c' e' c' g |
  g,8 d g b fis' d' |
  e,8 b, e g d' b |
  c8\p g c' e' c' g |
  a,8 e a c' g e |
  d8 a c' fis' a fis' |
  g,4. <g b d'>4.\fermata
}
\score {
  <<
    \new Staff \with { instrumentName = "Fl." midiInstrument = "flute" } \music
    \new Staff \with { instrumentName = "Pno." midiInstrument = "acoustic grand" } \accompaniment
  >>
  \layout { }
  \midi { }
}
