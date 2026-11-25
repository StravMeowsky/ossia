\version "2.26.0"
fluteOne = \relative c''' { \time 4/4 \key bes \major \tempo 4 = 120 r8 bes32( c d16) bes( d) c( f,) bes8 r8 r4 | }
fluteTwo = \relative c''' { \time 4/4 \key bes \major r8 d,32( es f16) d( f) es( c) d8 r8 r4 | }
\score { \new Staff << \new Voice { \voiceOne \fluteOne } \\ \new Voice { \voiceTwo \fluteTwo } >> \layout {} \midi {} }
