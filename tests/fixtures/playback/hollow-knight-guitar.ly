\version "2.26.0"
\include "predefined-guitar-fretboards.ly"
\storePredefinedDiagram #default-fret-table \chordmode { f/a } #guitar-tuning "x;o;3;2;1;1;"
\storePredefinedDiagram #default-fret-table \chordmode { d:m/a } #guitar-tuning "x;o;o;2;3;1;"
\storePredefinedDiagram #default-fret-table \chordmode { d:m/f } #guitar-tuning "1;x;o;2;3;1;"
\storePredefinedDiagram #default-fret-table \chordmode { g/d } #guitar-tuning "x;x;o;o;o;3;"
\storePredefinedDiagram #default-fret-table \chordmode { d/fis } #guitar-tuning "2;x;o;2;3;2;"
\storePredefinedDiagram #default-fret-table \chordmode { a:m/c } #guitar-tuning "x;3;2;2;1;o;"
\storePredefinedDiagram #default-fret-table \chordmode { g:7/f } #guitar-tuning "1;x;o;o;o;1;"
\header {
  title = "空洞骑士"
  subtitle = "吉他独奏 · 标准调弦"
  composer = "Christopher Larkin"
  arranger = "根据蓝色小草钢琴谱改编"
}
melody = \transpose c a, {
  \key es \major \time 3/4
  c''4( c'' d''8 es'' | d''2.) |
  c''8( g' as'4 g'8 f' | g'2.) \break |
  c''4( c'' d''8 es'' | f''2.) |
  g''8( es'' d'' c'' bes'4 | c''2.) \break |
  <c' f'>2\arpeggio es'4 |
  c''4( c'' d''8 es'' | d''2.) |
  c''8( g' as'4 g'8 f' | g'2.) \break |
  c''4( c'' d''8 es'' | f''2 ~ f''8) g''16( as'' |
  g''4 es''4. bes'8 | c''4 bes'4. f'8 \break |
  g'2. ~ | g'2.) ~ | g'2. ~ | g'2 r8 ces''8( \break |
  ces''2. | c''!2 ~ \tuplet 3/2 { c''8 d'' es'' } |
  f''2. | g''2 bes''4 \break | c'''2. ~ | c'''2.) \bar "||"
}
bass = {
  a,2.\5 | a,2.\5 | a,2.\5 | a,2.\5 |
  a,2.\5 | g,2.\6 | f,2.\6 | d2.\4 |
  fis,2.\6 | a,2.\5 | a,2.\5 | a,2.\5 |
  g,2.\6 | f,2.\6 | d2.\4 | c2.\5 |
  f,2.\6 | a,2.\5 | a,2.\5 | a,2.\5 |
  g,2.\6 | a,2.\5 | a,2.\5 | a,2.\5 |
  g,2.\6 | a,2.\5 | a,2.\5 |
}
harmonies = \chordmode {
 f2./a d2.:m/a d2.:m/a a2.:m7 |
 f2./a g2. d2.:m/f g2./d |
 d2./fis f2./a d2.:m/a d2.:m/a |
 g2. d2.:m/f d2.:m a2.:m/c |
 g2.:7/f f2./a d2.:m/a d2.:m/a |
 g2. f2./a d2.:m/a d2.:m/a |
 g2. a2.:m a2.:m
}
diagrams = \chordmode {
 f2. d2.:m s2. a2.:m7 |
 f2. g2. d2.:m g2. |
 d2. f2. d2.:m s2. |
 g2. d2.:m s2. a2.:m |
 g2.:7 f2. d2.:m s2. |
 g2. f2. d2.:m s2. |
 g2. a2.:m s2.
}
\score {
 <<
  \new ChordNames \with { \remove "Staff_performer" } { \harmonies }
  \new FretBoards \with { \remove "Staff_performer" } { \harmonies }
  \new TabStaff \with { midiInstrument = "acoustic guitar (nylon)" } <<
    \new TabVoice { \tabFullNotation \voiceOne \melody }
    \new TabVoice { \voiceTwo \bass }
  >>
 >>
 \layout { }
 \midi { \tempo 4 = 100 }
}
