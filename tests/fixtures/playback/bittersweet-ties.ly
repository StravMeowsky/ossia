\version "2.26.0"
upper = \relative b' { \time 4/4 \key e \major \tempo 4 = 81
  <a cis>1 ~ | <a cis>1 | <gis _~ cis ^~>1 | <gis cis>1 |
}
lower = \relative a { a8 e' a e b' e, a e | a,1 | a8 e' a e b' e, a e | a,1 | }
\score { \new PianoStaff << \new Staff \upper \new Staff \lower >> \layout {} \midi {} }
