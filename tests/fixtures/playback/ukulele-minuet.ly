\version "2.26.0"

% Christian Petzold, Menuet in G, BWV Anh. 114 (public domain).
% Melody adapted from the Bach-Gesellschaft edition, typeset and placed in
% the public domain by Allen Garvin for the Mutopia Project:
% https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=75
% This ukulele arrangement omits the keyboard bass and ornaments, and adds
% selected double stops. Transposed down a whole tone; repeated sections are written out.

\header {
  title = "Minuet in G"
  subtitle = "Ukulele solo in F · high G tuning (G4 C4 E4 A4)"
  composer = "Christian Petzold (1677–1733)"
  opus = "BWV Anh. 114 · from the Notebook for Anna Magdalena Bach"
  tagline = ##f
}

music = \transpose g f {
  \key g \major
  \time 3/4
  \tempo 4 = 108
  \relative c'' {
d4 g,8 a b c |
    d4 g, g |
    e'4 c8 d e fis |
    g4 g, g |
    c4 d8 c b a |
    b4 c8 b a g |
    fis4 g8 a b g |
    a2. |
    d4 g,8 a b c |
    d4 g, g |
    e'4 c8 d e fis |
    g4 g, g |
    c4 d8 c b a |
    b4 c8 b a g |
    a4 b8 a g fis |
    <g b>2. |
  }
  \relative c'' {
d4 g,8 a b c |
    d4 g, g |
    e'4 c8 d e fis |
    g4 g, g |
    c4 d8 c b a |
    b4 c8 b a g |
    fis4 g8 a b g |
    a2. |
    d4 g,8 a b c |
    d4 g, g |
    e'4 c8 d e fis |
    g4 g, g |
    c4 d8 c b a |
    b4 c8 b a g |
    a4 b8 a g fis |
    <g b>2. |
  }
  \relative c'' {
b'4 g8 a b g |
    a4 d,8 e fis d |
    g4 e8 fis g d |
    cis4 b8 cis a4 |
    a8 b cis d e fis |
    g4 fis e |
    fis4 a, cis |
    d2. |
    d4 g,8 fis g4 |
    e'4 g,8 fis g4 |
    d'4 c b |
    a8 g fis g a4 |
    d,8 e fis g a b |
    c4 b a |
    b8 d g,4 fis |
    <g b>2. |
  }
  \relative c'' {
b'4 g8 a b g |
    a4 d,8 e fis d |
    g4 e8 fis g d |
    cis4 b8 cis a4 |
    a8 b cis d e fis |
    g4 fis e |
    fis4 a, cis |
    d2. |
    d4 g,8 fis g4 |
    e'4 g,8 fis g4 |
    d'4 c b |
    a8 g fis g a4 |
    d,8 e fis g a b |
    c4 b a |
    b8 d g,4 fis |
    <g b>2. |
  }
}

\score {
  \new StaffGroup <<
    \new Staff { \clef treble \music }
    \new TabStaff \with { stringTunings = #ukulele-tuning } { \music }
  >>
  \layout { }
  \midi { \context { \TabVoice \remove "Note_performer" } }
}
