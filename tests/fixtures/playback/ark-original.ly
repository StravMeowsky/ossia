\version "2.26.0"
% source metadata removed
\pointAndClickOff

%% additional definitions required by the score:
D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc


\header {
  title = "宇宙の方舟"
  subtitle = "ASTLIBRA Revision"
  composer = "Composer: Kokoro Majima"
  arranger = "Arranger"
  "id: software" = "Songscription.ai"
  "id: encoding-date" = ""
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
PartPOneVoiceOne = \relative as'' {
  \clef "treble" \numericTimeSignature \time 4/4 \key as \major \tempo 4 = 54 \oneVoice \D as16 -\mp \D es16
  \D bes'16 \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16
  \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D bes'16 \D
  es,16 | % 1
  \D as16 \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D bes'16
  \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16 \D es16
  \D bes'16 \D es,16 | \break % 2
  \D as16 \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D bes'16
  \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16 \D es16
  \D bes'16 \D es,16 | % 3
  g4 f4 es4 \U <g, c>8 \U bes8 | % 4
  \D as'16 \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D bes'16
  \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16 \D es16
  \D bes'16 \D es,16 | \break % 5
  \D as16 -\mf \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D
  bes'16 \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16
  \D es16 \D bes'16 \D es,16 | % 6
  \D as16 -\mp \D es16 \D bes'16 \D es,16 \D as16 \D es16 \D
  bes'16 \D es,16 \D as16 \D es16 \D bes'16 \D es,16 \D as16
  \D es16 \D bes'16 \D es,16 | % 7
  es'4. -\mf bes8 <es, g>2 | \break % 8
  r8. -\p \U f,16 \U g16 \U as16 \U g16 \U f16 ~ \U f8. \U f16
  \U g16 \U as16 \U g16 \U f16 ~ | % 9

  \barNumberCheck #10
  \U f8. -\mp \U f16 \U g16 \U as16 \U g16 \U f16 ~ \U f4 \U g16
  \U as16 \U g16 \U f16 ~ | % 10
  \U f8. \U f16 \U g16 \U as16 \U g16 \U f16 ~ \U f8. \U f16
  \U g16 \U as16 \U g16 \U f16 ~ | \break % 11
  \U f8. -\mf \U f16 \U g16 \U as16 \U g16 \U f16 \U g8. \U f16
  \U g16 \U as16 \U g16 \U f16 ~ | % 12
  \U f8. \U f16 \U g16 \U as16 \U g16 \U f16 ~ \U f8. \U f16
  \U g16 \U as16 \U g16 \U f16 ~ | % 13
  \U f8. \U f16 \U g16 \U as16 \U g16 \U f16 ~ \U f8. \U f16
  \U g16 \U as16 \U g16 \U f16 | \break % 14
  \U <des f bes>8. -\mp \U <des f bes>16 \U <des f as
   >8. \U as'16 \U <c, es g>8. \U g'16 \U es8. \U es16 | % 15
  <bes c f>4. \arpeggio f'16 as16 \tempo 4 = 76 c,8 bes'8 f8 c'8 | % 16
  as8 f'8 c8 bes'8 ~ bes2 | % 17
  \tempo 4 = 54 <f, bes es>2
  \arpeggio -\f <f' bes es>2 \arpeggio | \pageBreak % 18
  \U <f,, as bes es>2 -\mf \U <f' as bes es>4 \D es''16
  \D bes8 \D as16 | % 19

  \barNumberCheck #20
  g4. f8 es4 \U <es, bes' es>4 | % 20
  \tempo 4 = 60 \voiceOne <c es
    as c>1 | \break % 21
  \oneVoice \U as'16 \U c,16 \U es16 \U c16 \U as'16 \U c,16
  \U es16 \U c16 \U as'16 \U c,16 \U es16 \U c16 \U as'16
  \U c,16 \U es16 \U c16 | % 22
  \U g'16 \U c,16 \U es16 \U c16 \U g'16 \U c,16 \U es16 \U
  c16 \U g'16 \U c,16 \U es16 \U c16 \U g'16 \U c,16 \U es16
  \U c16 | % 23
  \U f16 -\mp \U c16 \U es16 \U c16 \U f16 \U c16 \U es16
  \U c16 \U f16 \U c16 \U d16 \U c16 \U f16 \U c16 \U d16
  \U c16 | \break % 24
  \U es16 \U as,16 \U bes16 \U as16 \U es'16 \U as,16 \U bes16
  \U as16 \U e'16 \U g,16 \U c16 \U g16 \U e'16 \U g,16
  \U c16 \U g16 | % 25
  \U as'16 -\f \U bes,16 \U c16 \U bes16 \U as'16 \U bes,16 \U
  c16 \U bes16 \U as'16 \U bes,16 \U c16 \U bes16 \U as'16
  \U bes,16 \U c16 \U bes16 | \break % 26
  \U <c as'>16 \U bes16 \U c16 \U bes16 \U as'16 \U bes,16
  \U c16 \U bes16 \U as'16 \U bes,16 \U c16 \U bes16 \U as'16
  \U bes,16 \U c16 \U bes16 | % 27
  r16 \U es16 \U f16 \U es16 \U bes'16 \U es,16 \U f16 \U es16
  \U bes'16 \U es,16 \U f16 \U es16 \U es'16 \U es,16 \U f16
  \U es16 | % 28
  \U c'16 \U es,16 \U f16 \U es16 \U c'16 \U es,16 \U f16
  \U es16 \U c'16 \U e,16 \U g16 \U e16 \U c'16 \U e,16 \U
  g16 \U e16 | \break % 29

  \barNumberCheck #30
  \U as16 \U c,16 \U es16 \U c16 \U as'16 \U c,16 \U es16
  \U c16 \U as'16 \U c,16 \U es16 \U c16 \U as'16 \U c,16
  \U es16 \U c16 | % 30
  \U f16 \U c16 \U es16 \U c16 \U f16 \U c16 \U es16 \U c16
  \U f16 \U c16 \U es16 \U f16 \U c16 \U es16 \U f16 \U
  c16 | % 31
  \tempo 4 = 46 \U es16 -\mp \U f8.
  \U es16 \U f16 \U bes16 \U c16 \D es16 \D f16 \D bes16
  \D c16 \ottava #1 <es es'>4 \arpeggio \ottava #0 \bar "|."
}

PartPOneVoiceOneChords = \chordmode {
  f16:m7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 1
  des16:maj9 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 2
  f16:m7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 3
  des4 s4 s4 es8 s8 | % 4
  f16:m7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 5
  des16 s16 s16 s16 s16 s16 s16 s16 es16 s16 s16 s16 s16 s16 s16 s16 | % 6
  f16:m7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 7
  des4. s8 es2 | % 8
  f8.:m7 s16 s16 s16 s16 s16 s8. s16 s16 s16 s16 s16 | % 9

  \barNumberCheck #10
  s8. s16 s16 s16 s16 s16 s4 s16 s16 s16 s16 | % 10
  c8.:m s16 s16 s16 s16 s16 s8. s16 s16 s16 s16 s16 | % 11
  es8.:11 s16 s16 s16 s16 s16 c8./+e s16 s16 s16 s16 s16 | % 12
  f8.:m9 s16 s16 s16 s16 s16 s8. s16 s16 s16 s16 s16 | % 13
  des8. s16 s16 s16 s16 s16 s8. s16 s16 s16 s16 s16 | % 14
  bes8.:m s16 bes8.:m9/+c s16 c8.:m s16 s8. s16 | % 15
  s4. s16 s16 s8 s8 s8 s8 | % 16
  s8 s8 s8 s8 s2 | % 17
  f2:m9 s2 | % 18
  des2:maj9 s4 s16 s8 s16 | % 19

  \barNumberCheck #20
  es4.:maj9 s8 s4 s4 | % 20
  as1 | % 21
  f16:m9 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 22
  c16:m s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 23
  des16:maj9 s16 s16 s16 s16 s16 s16 s16 d16:m7 s16 s16 s16 s16 s16 s16 s16 | % 24
  es16:sus4 s16 s16 s16 s16 s16 s16 s16 c16/+e s16 s16 s16 s16 s16 s16 s16 | % 25
  c16:7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 26
  des16:maj7 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 27
  es16:maj9 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 28
  as16 s16 s16 s16 s16 s16 s16 s16 c16/+g s16 s16 s16 s16 s16 s16 s16 | % 29

  \barNumberCheck #30
  f16:m7 s16 s16 s16 s16 s16 s16 s16 as16/+es s16 s16 s16 s16 s16 s16 s16 | % 30
  bes16:sus2 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 31
  des16:maj7 s8. s16 s16 s16 s16 s16 s16 s16 s16 s4 \bar "|."
}

% Under the tempo map, s1*20 r8 places the first C at 87.459052 s.
% Three sixteenths separate the next C attacks: 88.209052, 88.959052,
% 89.709052 s. Stem-direction aliases consume no musical time.
PartPOneVoiceTwo = \relative c' {
  \clef "treble" \numericTimeSignature \time 4/4 \key as \major s1*20 | % 20
  r8 \D c16 \D es16 \D g16 \D c,16 \D es16 \D g16 \D c,16
  \D es16 \D g16 \D c,16 \D es16 \D g16 \D c,16 \D es16
  s1*11 \bar "|."
}

PartPOneVoiceThree = \relative f {
  \clef "treble" \numericTimeSignature \time 4/4 \key as \major \U f16 \U c'16
  \U es16 \U g16 \U bes16 \U g16 \U bes16 \U g16 \U f,16
  \U c'16 \U f16 \U g16 \U as4 | % 1
  \clef "bass" \D des,,16 \D as'16 \D des16 \D es16 f4 \D des,16
  \D as'16 \D des16 \D es16 f4 | % 2
  \clef "treble" \U f,16 \U c'16 \U f16 \U g16 \U as4 \U f,16
  \U c'16 \U f16 \U g16 \U as4 | % 3
  \clef "bass" \D des,,16 \D as'16 \D des16 \D es16 \D f16 \D es16
  \D f16 \D es16 \D f16 \D es16 \D f16 \D es16 <es, bes'
    c>4 | % 4
  \clef "treble" f16 c'16 es16 f16 g16 f16 g16 f16 f,16
  c'16 f16 g16 as16 g16 as16 g16 | % 5
  \clef "bass" des,16 as'16 des16 es16 f16 es16 f16 es16
  es,16 bes'16 es16 f16 g4 | % 6
  \clef "treble" f,16 c'16 f16 g16 as16 g16 as16 g16 f,16
  c'16 f16 g16 as16 g16 as16 g16 | % 7
  \clef "bass" des,16 as'16 des16 es16 f4 es,2 | % 8
  f16 c'16 es4. f,16 c'16 es4. | % 9

  \barNumberCheck #10
  f,16 c'16 es4. f,16 c'16 es4 es,8 | % 10
  des16 as'16 des4. des,16 as'16 des4. | % 11
  es,16 bes'16 des4. e,16 c'16 e!4. | % 12
  f,16 c'16 es4. f,16 c'16 es4. | % 13
  des,16 as'16 des4. des,16 as'16 des4. | % 14
  bes,16 f'16 bes8 c4 c,16 g'16 c4. | % 15
  f,,16 c'16 f16 g16 as16 c16 bes16 s16*25 | % 17
  f,16 c'16 f16 g16 as16 c16 f16 as16 c2 | % 18
  des,,,16 as'16 des16 f16 as16 bes16 c8 ~ c2 | % 19

  \barNumberCheck #20
  es,,16 bes'16 es16 f16 g16 bes8. es2 | % 20
  as,,16 es'16 as16 bes16 c16 bes16 c16 bes16 r16 bes16
  c16 bes16 c16 bes16 g,8 \change Staff="1" \change Staff="2"
  | % 21
  f16 c'16 f16 g16 as16 g16 as16 g16 as16 g16 as16
  g16 as16 g8. | % 22
  c,,16 g'16 c16 es16 g16 es16 g16 es16 g8 g8 g16
  es8. | % 23
  des,16 as'16 des16 es16 f4 d,4 d'!4 | % 24
  es,16 bes'16 es4. e,16 c'16 e!4. | % 25
  f,16 c'16 f16 g16 ~ g4 ~ g4 g4 | % 26
  des,16 as'16 des16 es16 f2. | % 27
  es,16 bes'16 es16 f16 ~ f4 g4 bes4 | % 28
  as,16 es'16 as16 bes16 c4 g,16 e'16 g4. | % 29

  \barNumberCheck #30
  f,16 c'16 f16 as16 ~ as4 es,16 c'16 es4. | % 30
  d,16 bes'8 f'16 ~ f8 bes8 ~ bes2 | % 31
  r8 des,16 as'16 bes16 c8. r2 \bar "|."
}

PartPOneVoiceThreeChords = \chordmode {
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s4 | % 1
  s16 s16 s16 s16 s4 s16 s16 s16 s16 s4 | % 2
  s16 s16 s16 s16 s4 s16 s16 s16 s16 s4 | % 3
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s4 | % 4
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 5
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s4 | % 6
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 | % 7
  s16 s16 s16 s16 s4 s2 | % 8
  s16 s16 s4. s16 s16 s4. | % 9

  \barNumberCheck #10
  s16 s16 s4. s16 s16 s4 s8 | % 10
  s16 s16 s4. s16 s16 s4. | % 11
  s16 s16 s4. s16 s16 s4. | % 12
  s16 s16 s4. s16 s16 s4. | % 13
  s16 s16 s4. s16 s16 s4. | % 14
  s16 s16 s8 s4 s16 s16 s4. | % 15
  s16 s16 s16 s16 s16 s16 s8*13 | % 17
  s16 s16 s16 s16 s16 s16 s16 s16 s2 | % 18
  s16 s16 s16 s16 s16 s16 s8 s2 | % 19

  \barNumberCheck #20
  s16 s16 s16 s16 s16 s8. s2 | % 20
  s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s8 | % 21
  as16:maj7/+c s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s16 s8. | % 22
  s16 s16 s16 s16 s16 s16 s16 s16 s8 s8 s16 s8. | % 23
  s16 s16 s16 s16 s4 s4 s4 | % 24
  s16 s16 s4. s16 s16 s4. | % 25
  s16 s16 s16 s16 s4 s4 s4 | % 26
  s16 s16 s16 s16 s2. | % 27
  s16 s16 s16 s16 s4 s4 s4 | % 28
  s16 s16 s16 s16 s4 s16 s16 s4. | % 29

  \barNumberCheck #30
  s16 s16 s16 s16 s4 s16 s16 s4. | % 30
  s16 s8 s16 s8 s8 s2 | % 31
  s8 s16 s16 s16 s8. s2 \bar "|."
}


% The score definition
\score {
  <<
    \context ChordNames = "PartPOneVoiceOneChords" {
      \omit ChordName
      \PartPOneVoiceOneChords
    }
    \context ChordNames = "PartPOneVoiceThreeChords" {
      \omit ChordName
      \PartPOneVoiceThreeChords
    }
    \new PianoStaff <<
      \context Staff = "1" <<
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPOneVoiceOne" {
          \voiceOne \PartPOneVoiceOne
        }
        \context Voice = "PartPOneVoiceTwo" {
          \voiceTwo \PartPOneVoiceTwo
        }
      >>
      \context Staff = "2" <<
        \override Staff.BarLine.allow-span-bar = ##f
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPOneVoiceThree" {
          \PartPOneVoiceThree
        }
      >>
    >>
  >>
  \layout {}
  % To create MIDI output, uncomment the following line:
  % \midi { \tempo 4 = 53.53145370767824 }
}

