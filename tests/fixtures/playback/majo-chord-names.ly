\version "2.26.0"
upper = { \key b \minor \time 4/4 \tempo 4 = 100 R1 | \break R1 | }
lower = \relative b, { \key b \minor \clef bass \time 4/4 b4 <fis' a d>2. | \break b,4 <e a cis>2. | }
harmonies = \chordmode { b1:m7 | a1/e | }
\score { << \new ChordNames \with { \remove "Staff_performer" } \harmonies \new PianoStaff << \new Staff \upper \new Staff \lower >> >> \layout {} \midi {} }
