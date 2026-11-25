\version "2.26.0"
% automatically converted by musicxml2ly from input.musicxml

%% additional definitions required by the score:
#(define-markup-command (square layout props arg)
   (markup?)
   (let* ((stil (interpret-markup layout props arg))
          (x-size (interval-length (ly:stencil-extent stil X)))
          (y-size (interval-length (ly:stencil-extent stil Y)))
          (axis (if (> x-size y-size) Y X)))
     (interpret-markup
      layout props
      (markup #:box #:with-dimension-from axis #:rotate 90 arg arg))))

D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc


\header {
  title = "心湖"
  subtitle = "《影之刃零》沐小葵主题曲"
  composer = \markup \column { "Original by 薄彩生" "Arranged by Anonymous" }
  "id: software" = "music21 v.10.5.2"
  "id: encoding-date" = "2026-11-07"
}
#(set-global-staff-size 17.5)
\paper {
  #(set-paper-size "a4")
  top-margin = 9\mm
  bottom-margin = 9\mm
  left-margin = 12\mm
  right-margin = 12\mm
  top-system-spacing.basic-distance = 6
  top-system-spacing.minimum-distance = 5
  top-system-spacing.padding = 0.5
  markup-system-spacing.basic-distance = 8
  markup-system-spacing.minimum-distance = 6
  markup-system-spacing.padding = 0.5
  system-system-spacing.basic-distance = 7.0
  system-system-spacing.minimum-distance = 6.0
  system-system-spacing.padding = 0.5
  last-bottom-spacing.basic-distance = 4
  last-bottom-spacing.minimum-distance = 3
  last-bottom-spacing.padding = 1
  system-count = 8
  ragged-last-bottom = ##f
}
\layout {
  indent = 0
  \context {
    \Staff
    printKeyCancellation = ##f
  }
  \context {
    \Score
    autoBeaming = ##f
  }
  \context {
    \PianoStaff
    \override StaffGrouper.staff-staff-spacing.basic-distance = 7.5
    \override StaffGrouper.staff-staff-spacing.minimum-distance = 6.0
    \override StaffGrouper.staff-staff-spacing.padding = 0.5
  }
}
PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceOne =
\relative a'' {
  \once \override Score.MetronomeMark.stencil = ##f
  \tempo 4 = 96 \clef "treble" \numericTimeSignature \time 4/4 \key a \minor \break \once \override Score.RehearsalMark.padding = #4.0 \mark
  \markup { \square A \hspace #1.5 \fontsize #-2 { \note-by-number #2 #0 #1 " = 96" } } a2.. d8 | % 1
  a2.. e'8 | % 2
  d4 a2 \D f8 [ \D g8 ] | % 3
  e1 \break | % 4
  \U a,8 [ \U e8 ] f8 a4. c8 [ d8 ] | % 5
  \U <e, a d>8 [ \U c'8 ] \U c8 [ \U g8 ] a4. \D c16 [ \D d16 ] | % 6
  <d, a'>2 f4 \U <d g>8 [ \U f8 ] | % 7
  <g, d' e>1 \break | % 8
  \mark \markup \square B \D e''8 [ \D f8 ] \D e8 [ \D f8 ] \D e8 [ \D f8 ]
  e4 ~ | % 9

  \barNumberCheck #10
  e1 | % 10
  \D d8 [ \D e8 ] \D d8 [ \D e8 ] d4. c8 | % 11
  c2.. a8 \break | % 12
  \D e'8 [ \D f8 ] \D e8 [ \D f8 ] \D e8 [ \D f8 ] \D e8 [ \D g8 ] | % 13
  e2.. a8 | % 14
  d,2.. g8 | % 15
  \D <a, f'>8 [ \D e'8 ] d4 e4. d8 \break | % 16
  \mark \markup \square "B′" \D e8 [ \D f8 ] \D e8 [ \D f8 ] \D e8 [ \D f8
  ] e4 ~ | % 17
  e2 <a, a'>2 | % 18
  \D <f a d>8 [ \D e'8 ] \D d8 [ \D e8 ] d4. c8 | % 19

  \barNumberCheck #20
  <g c>1 \break | % 20
  \D <a e'>8 [ \D f'8 ] \D e8 [ \D f8 ] \D e8 [ \D f8 ] \D e8 [ \D g8 ] | % 21
  <a, e' g>2 a'4. a8 | % 22
  <a, d>2.. g'8 | % 23
  \D <a, d f>8 [ \D e'8 ] d4 <a cis e>2 \break | % 24
  \mark \markup \square C <f a e'>2 <f a e'>4. <a a'>8 | % 25
  <e e'>2. <a a'>4 | % 26
  <g g'>4 <f f'>4 <b b'>4 <a a'>4 | % 27
  <e e'>1 | % 28
  <f a e'>2 <f a e'>4. <a a'>8 | % 29

  \barNumberCheck #30
  <b b'>2 <b b'>4. <a a'>8 | % 30
  <c c'>2 <c c'>4. <d d'>8 | % 31
  b'2 a4( gis4) | % 32
  <c, e a>1 \fermata \bar "|."
}

PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceTwo =
\relative d' {
  \clef "treble" \numericTimeSignature \time 4/4 \key a \minor \break \oneVoice
  \U d8 [ \U a'8 ] d2 a4 | % 1
  \U c,8 [ \U a'8 ] e'2. | % 2
  \U bes,8 [ \U f'8 ] a2. | % 3
  \U c,8 [ \U g'8 ] c2 \clef "bass" a,,4 \break | % 4
  \D d8 [ \D a'8 ] d2. | % 5
  \D c,8 [ \D g'8 ] \D a8 [ \D c8 ] g2 | % 6
  \D bes,8 [ \D f'8 ] bes!8 c4. bes,!4 | % 7
  a2 a,2 \break | % 8
  \D d'8 [ \D a'8 ] e'2. | % 9

  \barNumberCheck #10
  \D c,8 [ \D g'8 ] d'8 e2 ~ e8 | % 10
  \D bes,8 [ \D g'8 ] \D c8 [ \D d8 ] e4 g,4 | % 11
  \U a,8 [ \U e'8 ] \D bes'8 [ \D c8 ] bes!4 a4 \break | % 12
  \D d,8 [ \D a'8 ] e'8 f4 g8 a4 | % 13
  \D c,,8 [ \D g'8 ] d'8 e2 ~ e8 | % 14
  \D bes,8 [ \D f'8 ] bes!8 c4. f4 | % 15
  \D bes,,8 [ \D f'8 ] \D gis8 [ \D a8 ] \D b8 [ \D f'8 ] \D e8 [ \D d8 ] \break
  | % 16
  \D d,8 [ \D a'8 ] e'2 g!4 | % 17
  \D c,,8 [ \D g'8 ] d'8 e2 ~ e8 | % 18
  <bes, f' a>4. c'4 bes4. | % 19

  \barNumberCheck #20
  <a, e' g>4 \D gis''8 [ \D g8 ] \D g8 [ \D f8 ] \D g8 [ \D a8 ] \break | % 20
  bes,,8 f'4 e'8 f2 | % 21
  \D c,8 [ \D g'8 ] d'8 c2 ~ c8 | % 22
  \D bes,8 [ \D f'8 ] c'8 d2 ~ d8 | % 23
  <a, e'>2 <a e' a>2 \break | % 24
  \D d8 [ \D a'8 ] \D e'8 [ \D a,8 ] \D d,8 [ \D a'8 ] e'4 | % 25
  \D c,8 [ \D g'8 ] \D c8 [ \D g8 ] \D c,8 [ \D g'8 ] c4 | % 26
  \D bes,8 [ \D f'8 ] \D c'8 [ \D f,8 ] \D bes,!8 [ \D f'8 ] \D e'8 [ \D d8 ]
  | % 27
  \U a,8 [ \U e'8 ] \D b'!8 [ \D e,8 ] \D b'8 [ \D e,8 ] \D b'8 [ \D e,8 ] | % 28
  \D d8 [ \D a'8 ] \D d8 [ \D a8 ] \D d,8 [ \D a'8 ] \D e'8 [ \D a,8 ] | % 29

  \barNumberCheck #30
  \D c,8 [ \D g'8 ] \D e'8 [ \D g,8 ] \D c,8 [ \D g'8 ] \D e'8 [ \D g,8 ]
  | % 30
  \D bes,8 [ \D f'8 ] \D d'8 [ \D f,8 ] \D bes,!8 [ \D f'8 ] d'4 | % 31
  \voiceOne \change Staff="1" \stemDown b'2 e4 b4 \change Staff="2" | % 32
  \oneVoice <a,, e' a>1 \fermata \bar "|."
}

PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceThree =
\relative e, {
  \clef "treble" \numericTimeSignature \time 4/4 \key a \minor s4*15
  \clef "bass" s4 s1*4 s1*4 s1*4 s1*4
  s1*4 s1*3 s1*3 s1 | % 31
  \oneVoice \U e8 [ \U a'8 ] \D b!8 [ \D a8 ] <e e'>4 e4 s1 \bar "|."
}


% The score definition
\score {
  <<
    \new PianoStaff <<
      \context Staff = "1" <<
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceOne"
        {
          \PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceOne
        }
      >>
      \context Staff = "2" <<
        \override Staff.BarLine.allow-span-bar = ##f
        \mergeDifferentlyDottedOn
        \mergeDifferentlyHeadedOn
        \context Voice = "PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceTwo"
        {
          \voiceOne \PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceTwo
        }
        \context Voice = "PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceThree"
        {
          \voiceTwo \PartPThreeZeroZeroNineadedaFourdFoureFivedNineaZerobfFiveThreeeNineFiveFourTwoNineFourOneZerodVoiceThree
        }
      >>
    >>
  >>
  \layout {}
  \midi { \tempo 4 = 96 }
  % To create MIDI output, uncomment the following line:
  % \midi { \tempo 4 = 96 }
}
