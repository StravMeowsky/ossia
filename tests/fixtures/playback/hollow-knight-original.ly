\version "2.26.0"
\header {
  title = "Playback fixture"
  subtitle = ""
  composer = ""
  arranger = ""
}
\paper { #(set-paper-size "a4") }
global = { \key es \major \time 3/4 }
right = {
  \global \clef treble \transpose c c' { \ottava 1
  c''4-1( c''4 d''8 es'' | d''2. ) |
  c''8-5( g'8 as'4 g'8 f'8 | g'2. ) |
  c''4-1( c''4 d''8 es'' | f''2. ) \break |
  g''8-5( es'' d'' c''-1 bes'4-2 | c''2. ) \ottava 0 } |
  <c' f'>2\arpeggio-4 es'4-3 |
  c''4-1( c'' d''8 es'' | d''2. ) |
  c''8-5( g' as'4 g'8 f' | g'2. ) \break |
  c''4-1( c'' d''8 es'' | f''2 ~ f''8) g''16-4( as''-5 |
  g''4-4 es''4.-2 bes'8-1 | c''4-3 bes'4.-2 f'8-1 |
  g'2.-2 ~ | g'2.) ~ \break |
  g'2. ~ | g'2 r8 ces''8-1( |
  ces''2.-2 | c''!2-1 ~ \tuplet 3/2 { c''8 d''-2 es''-3 } \break |
  f''2.-1 | g''2-2 bes''4-4 | c'''2.-5 ~ | c'''2.) \bar "||"
}
left = {
  \global \clef treble
  c'8-5 g'8-2 as'2-1 | c'8-5 f'8-2 g'2-1 |
  c'8-5 es'8-3 f'2 | c'8-4 d' es' d' bes4 |
  c'8-5 g' as'2 | bes8-5 g' f'2 |
  as8-5 es' f'2 | \clef bass f8-5 bes-3 c' f' c' bes |
  a2.-4 |
  <c c'>8-5 g8-2 as2-1 | c8-5 f g2 | c8-5 es f2 |
  bes,8-5 es d4 bes, | as,8-5 es f2 |
  f,8-5 c-2 f-1 g-3 as4-2 |
  es,8-5 c-1 es-3 g-2 c'4-1 |
  as,8-5 es-2 bes,-4 f-1 d4-2 |
  c8-5 g as g as g | c8 f g f g f |
  c8-5 es-2 f-1 es f es |
  <bes, d>4-2-1 <f, bes,>4-5-2 <bes, d>4 |
  c8-5 g as g as g | c8 f g f g f |
  c8-5 es-3 f-2 es f es |
  bes,8-5 d-3 f-2^\markup { \italic "rit." } bes-1 d'4-2 |
  c'2.-3 ~ | c'2. |
}
\score {
  \new PianoStaff << \new Staff { \transpose c a, { \right } } \new Staff { \transpose c a, { \left } } >>
  \layout { }
  \midi { \tempo 4 = 100 }
}
