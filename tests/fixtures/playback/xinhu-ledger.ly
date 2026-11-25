\version "2.26.0"
upper = \relative a'' { \tempo 4 = 96 \time 4/4 \key a \minor a2.. d8 | a1 | }
lower = \relative c' { a4 e a e | a1 | }
\score { \new PianoStaff << \new Staff \upper \new Staff \lower >> \layout {} \midi {} }
