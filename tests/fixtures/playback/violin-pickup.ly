\version "2.26.0"
iA = { s16 | <d'' fis' b>8. s16 e''8. s16 <e'' fis' ais>8. d''32 cis''32 d''16. b32 a16. fis''32 | }
iB = { fis''16 | fis''8. b''16 g''8. fis''32 g''32 fis''8. s16*5 | }
\score { \new Staff << \key d \major \time 4/4 \partial 16 \new Voice \iA \\ \new Voice \iB >> \layout {} \midi { \tempo 4 = 32 } }
