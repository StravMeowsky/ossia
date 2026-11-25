\version "2.26.0"
\pointAndClickOff

D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc

\header {
  title = \markup {
    \center-column {
      \abs-fontsize #21 "魔女の館"
      \vspace #0.8
      \abs-fontsize #14 "ASTLIBRA"
    }
  }
  composer = \markup { \abs-fontsize #12.5 "Composer: Azell" }
  arranger = \markup { \abs-fontsize #12.5 \italic "Arranger" }
}

#(set-global-staff-size 16)

\paper {
  #(set-paper-size "a4")
  top-margin = 10\mm
  bottom-margin = 14\mm
  left-margin = 20\mm
  right-margin = 20\mm
  indent = 0\mm
  ragged-bottom = ##t
  ragged-last-bottom = ##t
  system-system-spacing.basic-distance = #20
  system-system-spacing.minimum-distance = #14
  system-system-spacing.padding = #2
}

\layout {
  \context {
    \Score
    autoBeaming = ##f
    \override DynamicText.font-size = #2
    \override MetronomeMark.font-size = #1
  }
  \context {
    \Voice
    \override TupletBracket.direction = #DOWN
    \override TupletBracket.bracket-visibility = ##t
    \override TupletNumber.direction = #DOWN
  }
}

upper = \relative fis'' {
  \key b \minor
  \clef "treble"
  \numericTimeSignature
  \time 4/4
  \tempo 4 = 100
  R1\mp | % 1
  R1 | % 2
  R1 | % 3
  R1 | % 4
  R1 | % 5
  R1 | % 6
  R1\mf | % 7

  \break
  R1 | % 8
  fis4. e8 \D fis8. [ \D a8. ] d,8 ] | % 9
  cis2 ~ cis8 b8 \D d8 [ \D g,8 ] | % 10
  fis4. e8 \U fis8. [ \U a8. ] d8 ] | % 11
  \D cis8. [ \D d8. ] e4 d8 \D e8 [ \D g8 ] | % 12

  \break
  fis4. e8 \D fis8. [ \D a8. ] d,8 ] | % 13
  cis2 ~ cis8 b8 \D d8 [ \D g,8 ] | % 14
  fis4. e8 \U fis8. [ \U a8. ] d8 ] | % 15
  \D e8. [ \D d8. ] g4
  \tuplet 3/2 { \D fis16 [ \D g16 \D fis16 ] }
  \D d8 [ \D e8 ] | % 16
  fis4. e8 \D fis8. [ \D a8. ] d,8 ] | % 17

  \break
  cis2 ~ cis8 b8 \D d8 [ \D g,8 ] | % 18
  fis4. e8 \U fis8. [ \U a8. ] d8 ] | % 19
  \D cis8. [ \D d8. ] e4 d8 \D e8 [ \D g8 ] | % 20
  fis4. e8 \D fis8. [ \D a8. ] d,8 ] | % 21
  cis2 ~ cis8 b8 \D d8 [ \D g,8 ] | % 22

  \break
  fis4. e8 \U fis8. [ \U a8. ] d8 ] | % 23
  \D e8. [ \D d8. ] cis4 a4 b8 | % 24
  d4. cis8 \D d8. [ \D fis8. ] d8 ] | % 25
  e4. e8 \D a8
  \tuplet 3/2 { \D g16 [ \D a16 \D g16 ] }
  \D fis8 [ \D g8 ] | % 26
  fis4. e8 \D fis8. [ \D a8. ] d,8\f ] | % 27

  \break
  cis8.\mf [ \D d8. ] e4 d8 \D cis8 [ \D a8 ] | % 28
  b4. cis8 \D d8. [ \D fis8. ] d8 ] | % 29
  e4. e8 \D a8
  \tuplet 3/2 { \D g16 [ \D a16 \D g16 ] }
  \D fis8 [ \D g8 ] | % 30
  fis4. e8 \D fis8 [ \D g8 ] \D a8 [ \D d8 ] | % 31
  \mark \markup { \bold "Fine" }
  e2 a,2 \bar "|." | % 32
}

lower = \relative b, {
  \key b \minor
  \clef "bass"
  \numericTimeSignature
  \time 4/4
  b4 <fis' a d>2. | % 1
  b,4 <e a cis>2. | % 2
  b4 <fis' a d>2. | % 3
  b,4 <e a cis>2. | % 4
  g,4 <fis' a d>2. | % 5
  g,4 <e' a cis>2. | % 6
  g,4 <fis' a d>2. | % 7
  g,4 <e' a cis>2. | % 8
  b4 <fis' a d>2. | % 9
  b,4 <e a cis>2. | % 10
  b4 <fis' a d>2. | % 11
  b,4 <e a cis>2. | % 12
  g,4 <fis' a d>2. | % 13
  g,4 <e' a cis>2. | % 14
  g,4 <fis' a d>2. | % 15
  g,4 <e' a cis>2. | % 16
  b4 <fis' a d>2. | % 17
  b,4 <e a cis>2. | % 18
  b4 <fis' a d>2. | % 19
  b,4 <e a cis>2. | % 20
  g,4 <fis' a d>2. | % 21
  g,4 <e' a cis>2. | % 22
  g,4 <fis' a d>2. | % 23
  g,4 <e' a cis>2. | % 24
  e,4 <g' b d>2. | % 25
  fis,4 <e' a cis>2. | % 26
  g,4 <g' b d>2. | % 27
  a,4 <e' a cis>2. | % 28
  e,4 <g' b d>2. | % 29
  fis,4 <e' a cis>2. | % 30
  g,4 <g' b d>2. | % 31
  a,4 <a' cis e>2. \bar "|." | % 32
}

chExceptionMusic = {
  <c e g b d'>-\markup { \super "maj9" }
  <c e g b>-\markup { \super "maj7" }
}

chExceptions = #(append
  (sequential-music-to-chord-exceptions chExceptionMusic #t)
  ignatzekExceptions)

harmonies = \chordmode {
  \set chordNameExceptions = #chExceptions
  b1:m7 |
  a1/e |
  b1:m7 |
  a1/e |
  g1:maj9 |
  a1:7/g |
  g1:maj9 |
  a1:7/g |
  b1:m7 |
  a1/e |
  b1:m7 |
  a1/e |
  g1:maj9 |
  a1:7/g |
  g1:maj9 |
  a1:7/g |
  b1:m7 |
  a1/e |
  b1:m7 |
  a1/e |
  g1:maj9 |
  a1:7/g |
  g1:maj9 |
  a1:7/g |
  e1:m7 |
  fis1:m7 |
  g1:maj7 |
  a1 |
  e1:m7 |
  fis1:m7 |
  g1:maj7 |
  a1 |
}

\score {
  <<
    \new ChordNames \with {
      \remove "Staff_performer"
      \override ChordName.font-size = #1
    } \harmonies
    \new PianoStaff <<
      \new Staff = "upper" << \upper >>
      \new Staff = "lower" << \lower >>
    >>
  >>
  \layout {}
  \midi {}
}
