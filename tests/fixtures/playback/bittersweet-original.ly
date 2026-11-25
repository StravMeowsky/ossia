\version "2.26.0"
% automatically converted by musicxml2ly from input.musicxml
\pointAndClickOff

%% additional definitions required by the score:
D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc


\header {
  title = \markup { \fontsize #2.5 \bold "Bittersweet" }
  subtitle = \markup { \fontsize #1 "ASTLIBRA" }
  composer = "Composer: G-MIYA"
  arranger = "Arranger"
  tagline = ##f
}
\paper {
  #(set-paper-size "a4")
  page-count = #2
  print-first-page-number = ##f
  ragged-last-bottom = ##f
  markup-system-spacing.basic-distance = #5
  indent = 10\mm
}
\layout {
  \context {
    \Staff
    printKeyCancellation = ##f
  }
  \context {
    \Score
    autoBeaming = ##t
  }
}
PartPOneVoiceOne = \relative gis'' {
  \clef "treble" \numericTimeSignature \time 4/4 \key e \major \tempo 4 = 81
  R1 | % 1
  R1 | % 2
  \D gis2. \D fis8 [ \D e8 ] | % 3
  \break
  \D dis2 \D b2 | % 4
  cis1 ^~ | % 5
  cis1 | % 6
  \D gis'2. ^~ \D gis8 [ \D fis16 \D e16 ] | % 7
  \break
  \D dis2 \D b2 | % 8
  <a cis>1 ~ | % 9

  \barNumberCheck #10
  <a cis>1 | % 10
  \D <cis gis'>2. \D fis8 [ \D e8 ] | % 11
  \break
  \D dis2 \D b2 | % 12
  <gis _~ cis ^~>1 | % 13
  <gis cis>1 | % 14
  \D <cis _~ gis' ^~>2. \D <cis gis'>8 [ \D fis16 \D e16 ] | % 15
  \break
  \D dis2 \D <e b'>2 | % 16
  <e _~ gis ^~>1 | % 17
  \D <e gis>2. \D gis4 | % 18
  \D <fis _~ cis' ^~>2. \D <fis cis'>8 [ \D dis'16 \D e16 ] | % 19
  \pageBreak

  \barNumberCheck #20
  \D <gis, dis'>4. \D b8 ^~ \D b4 \D gis4 | % 20
  \D <cis, fis>4. \D e8 ^~ \D e4 \D <cis fis>4 | % 21
  \D <dis gis>2 \D fis4. \D gis8 | % 22
  \D <fis cis'>2. ~ \D <fis cis'>8 [ \D dis'16 \D e16 ] | % 23
  \break
  \D <gis, dis'>4. \D cis8 ^~ \D cis2 | % 24
  \tempo 4 = 82
  \D <gis cis>4. \D <gis _~ bis ^~>8 \D <gis bis>4 \D <gis cis>4 | % 25
  \D <gis bis dis>4 \U <gis, bis>4 \D <ais cis>4 \D <bis dis>4 | % 26
  \D e8. [ \D dis16 ^~ ] \D dis8 [ \D e8 ] \U gis,4 _~ \D gis16 [ \D gis16 \D
  cis16 \D dis16 ] | % 27
  \break
  \D fis8. [ \D e16 ^~ ] \D e8 [ \D fis8 ] \D cis4. \D cis16 [ \D fis16 ] | % 28
  \D gis4. \D gis8 ^~ \D gis8 [ \D gis8 ] \tweak TupletBracket.direction #DOWN
  \tuplet 3/2 {
    \D b8 [ \D a8 \D gis8 ] }
  | % 29

  \barNumberCheck #30
  \D gis2 \D fis2 | % 30
  \D e8. [ \D dis16 ^~ ] \D dis8 [ \D e8 ] \U gis,4 _~ \D gis16 [ \D gis16 \D
  cis16 \D e16 ] | % 31
  \break
  \D gis8. [ \D fis16 ^~ ] \D fis8 [ \D gis8 ] \D cis,4. \D cis16 [ \D dis16
  ] | % 32
  \D e2 ^~ \D e8 [ \D fis8 ] \D e8 [ \D dis16 \D
  cis16 ^~ ] | % 33
  cis1 | % 34
  \break
  \D <e gis>8. [ \D <dis _~ fis ^~>16 ] \D <dis fis>8 [ \D <e gis>8 ] \U
  <gis, _~ cis ^~>4 \D <gis cis>16 [ \D <gis cis>16 \D <cis e>16 \D <dis gis>16
  ] | % 35
  \D <fis b>8. [ \D <e _~ a ^~>16 ] \D <e a>8 [ \D <fis b>8 ] \D <cis
    e>4. \D cis16 [ \D fis16 ] | % 36
  \D gis4. \D gis8 ^~ \D gis8 [ \D gis8 ] \tweak TupletBracket.direction #DOWN
  \tuplet 3/2 {
    \D b8 [ \D a8 \D gis8 ] }
  | % 37
  \D gis2 \D fis2 | % 38
  \break
  \D <e gis>8. [ \D <dis _~ fis ^~>16 ] \D <dis fis>8 [ \D <e gis>8 ] \U
  <gis, _~ cis ^~>4 \D <gis cis>16 [ \D <gis cis>16 \D <cis e>16 \D <e gis>16
  ] | % 39

  \barNumberCheck #40
  \D <gis b>8. [ \D <fis _~ a ^~>16 ] \D <fis a>8 [ \D <gis b>8 ] \D
  <cis, e>4. \D <cis e>16 [ \D <dis fis>16 ] | % 40
  \D <e _~ gis ^~>2 \D <e gis>8 ^( \D fis8 ) \D e8 [ \D dis16 \D cis16 ^~ ] | % 41
  cis1 \bar "|."
}


PartPOneVoiceFive = \relative cis {
  \clef "bass" \numericTimeSignature \time 4/4 \key e \major \D cis8 [ ^\mf \D
  gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 1
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 2
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 3
  \break
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 4
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 5
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 6
  \D cis8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 7
  \break
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 8
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 9

  \barNumberCheck #10
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 10
  \D cis8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 11
  \break
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 12
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 13
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 14
  \D cis8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 15
  \break
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D dis'8 [ \D gis,8 \D b8 \D gis8 ] | % 16
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 17
  \D a,8 [ \D e'8 \D a8 \D e8 ] \D b'8 [ \D e,8 \D a8 \D e8 ] | % 18
  \U fis,8 [ \U cis'8 \U fis8 \U cis8 ] \D gis'8 [ \D cis,8 \D fis8 \D cis8 ] | % 19
  \pageBreak

  \barNumberCheck #20
  \U gis8 [ \U dis'8 \U gis8 \U dis8 ] \D b'8 [ \D dis,8 \D gis8
  \D dis8 ] | % 20
  \D a8 [ \D e'8 \D a8 \D e8 ] \D cis'8 [ \D e,8 \D a8 \D e8 ] | % 21
  \U gis,8 [ \U dis'8 \U gis8 \U dis8 ] \D b'8 [ \D dis,8 \D gis8 \D dis8 ] | % 22
  \U fis,8 [ \U cis'8 \U fis8 \U cis8 ] \D gis'8 [ \D cis,8 \D fis8 \D cis8 ] | % 23
  \break
  \U e,8 [ \U cis'8 \U e8 \U cis8 ] \D fis8 [ \D cis8 \D e8 \D cis8 ] | % 24
  \U dis,8 [ \U ais'8 \U dis8 \U ais8 ] \U cis8 [ \U ais8 \U dis8 \U cis8 ] | % 25
  \U gis8 [ \U dis'8 \U gis8 \U dis8 ] \D ais'8 [ \D dis,8 \D bis'8 \D dis,8 ] | % 26
  \D cis8 [ \D gis'8 \D dis'8 \D gis,8 ] \D e'8 [ \D gis,8 \D dis'8 \D gis,8 ] | % 27
  \break
  \D a,8 [ \D e'8 \D b'8 \D e,8 ] \D cis'8 [ \D e,8 \D a8 \D e8 ] | % 28
  \U fis,8 [ \U cis'8 \U gis'8 \U cis,8 ] \D a'8 [ \D cis,8 \D b'8 \D cis,8 ] | % 29

  \barNumberCheck #30
  \D gis8 [ \D dis'8 \D gis8 \D b8 ] \D <b, cis'>8 [ \D fis'8 \D <b dis>8 \D
  fis8 ] | % 30
  \D cis8 [ \D gis'8 \D dis'8 \D gis,8 ] \D e'8 [ \D gis,8 \D dis'8 \D gis,8 ] | % 31
  \break
  \D a,8 [ \D e'8 \D b'8 \D e,8 ] \D cis'8 [ \D e,8 \D a8 \D e8 ] | % 32
  \U fis,8 [ \U cis'8 \U fis8 \U a8 ] \D gis,8 [ \D dis'8 \D gis8 \D b8 ] | % 33
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \D gis,8 [ \D dis'8 \D gis8 \D b8 ] | % 34
  \break
  \D cis,8 [ \D gis'8 \D dis'8 \D gis,8 ] \D e'8 [ \D gis,8 \D dis'8 \D gis,8 ]
  | % 35
  \D a,8 [ \D e'8 \D b'8 \D e,8 ] \D cis'8 [ \D e,8 \D a8 \D e8 ] | % 36
  \U fis,8 [ \U cis'8 \U gis'8 \U cis,8 ] \D a'8 [ \D cis,8 \D b'8 \D cis,8 ] | % 37
  \D gis8 [ \D dis'8 \D gis8 \D b8 ] \D <b, cis'>8 [ \D fis'8 \D <b dis>8 \D
  fis8 ] | % 38
  \break
  \D cis8 [ \D gis'8 \D dis'8 \D gis,8 ] \D e'8 [ \D gis,8 \D dis'8 \D gis,8 ] | % 39

  \barNumberCheck #40
  \D a,8 [ \D e'8 \D b'8 \D e,8 ] \D cis'8 [ \D e,8 \D a8 \D e8 ] | % 40
  \U fis,8 [ \U cis'8 \U fis8 \U a8 ] \D gis,8 [ \D dis'8 \D gis8 \D b8 ] | % 41
  \D cis,8 [ \D gis'8 \D cis8 \D gis8 ] \U cis,2 \bar "|."
}


% The score definition
\score {
  <<
    \new PianoStaff <<
      \context Staff = "1" <<
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPOneVoiceOne" {
          \PartPOneVoiceOne
        }
      >>
      \context Staff = "2" <<
        \override Staff.BarLine.allow-span-bar = ##f
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPOneVoiceFive" {
          \PartPOneVoiceFive
        }
      >>
    >>
  >>
  \layout {}
  \midi { \tempo 4 = 81 }
}
