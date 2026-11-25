\version "2.26.0"
upper = {
  \time 4/4
  \tempo 4 = 60
  \compressMMRests R1*2 |
  r4. r8 r2 |
  \tuplet 3/2 { r8 d''8 e''8 } r2. |
  \tempo 4 = 120 r1 |
  c''1 |
  r4 |
}
lower = {
  \time 4/4
  c'4 d' e' f' | g' a' b' c'' |
  c'4 d' e' f' | c' d' e' f' | c'1 | c'1 |
  r4 |
}
\score {
  \new PianoStaff << \new Staff \upper \new Staff \lower >>
  \layout { }
  \midi { }
}
