\version "2.26.0"

%% Johann Sebastian Bach (1685-1750)
%% Praeludium I in C major, BWV 846, from Das Wohltemperierte Klavier I (1722).
%%
%% Music after the public-domain Mutopia Project edition (Mutopia-2011/09/12-5).
%% Pitch content cross-checked against two independent public-domain engravings.
%% All pitches are written at sounding pitch.

\header {
  title     = "Praeludium I"
  subtitle  = "Das Wohltemperierte Klavier, Erster Teil"
  composer  = "Johann Sebastian Bach (1685–1750)"
  opus      = "BWV 846"
  style     = "Baroque"
  tagline   = ##f
}

#(set-global-staff-size 18)

\paper {
  #(set-paper-size "a4")
  indent                               = 10\mm
  print-first-page-number              = ##f
  top-system-spacing.basic-distance    = #10
  system-system-spacing.basic-distance = #11
  ragged-last-bottom                   = ##f
  bottom-edge                          = 14\mm
}

\pointAndClickOff

%% ---------------------------------------------------------------------------
%% Right hand: upper five notes of each broken-chord group, six sixteenths per
%% half bar after an eighth rest.  Bars 33-34 drop into the bass register.
%% ---------------------------------------------------------------------------
right = {
  \key c \major
  \time 4/4
  \clef "treble"
  \tempo 4 = 72
  \set Score.tempoHideNote = ##t

  r8 g'16[ c''] e''[ g' c'' e''] r8 g'16[ c''] e''[ g' c'' e''] |
  r8 a'16[ d''] f''[ a' d'' f''] r8 a'16[ d''] f''[ a' d'' f''] |
  r8 g'16[ d''] f''[ g' d'' f''] r8 g'16[ d''] f''[ g' d'' f''] |
  r8 g'16[ c''] e''[ g' c'' e''] r8 g'16[ c''] e''[ g' c'' e''] |
  r8 a'16[ e''] a''[ a' e'' a''] r8 a'16[ e''] a''[ a' e'' a''] |
  r8 fis'16[ a'] d''[ fis' a' d''] r8 fis'16[ a'] d''[ fis' a' d''] |
  r8 g'16[ d''] g''[ g' d'' g''] r8 g'16[ d''] g''[ g' d'' g''] |
  r8 e'16[ g'] c''[ e' g' c''] r8 e'16[ g'] c''[ e' g' c''] |
  r8 e'16[ g'] c''[ e' g' c''] r8 e'16[ g'] c''[ e' g' c''] |

  r8 d'16[ fis'] c''[ d' fis' c''] r8 d'16[ fis'] c''[ d' fis' c''] |
  r8 d'16[ g'] b'[ d' g' b'] r8 d'16[ g'] b'[ d' g' b'] |
  r8 e'16[ g'] cis''[ e' g' cis''] r8 e'16[ g'] cis''[ e' g' cis''] |
  r8 d'16[ a'] d''[ d' a' d''] r8 d'16[ a'] d''[ d' a' d''] |
  r8 d'16[ f'] b'[ d' f' b'] r8 d'16[ f'] b'[ d' f' b'] |
  r8 c'16[ g'] c''[ c' g' c''] r8 c'16[ g'] c''[ c' g' c''] |
  r8 a16[ c'] f'[ a c' f'] r8 a16[ c'] f'[ a c' f'] |
  r8 a16[ c'] f'[ a c' f'] r8 a16[ c'] f'[ a c' f'] |
  r8 g16[ b] f'[ g b f'] r8 g16[ b] f'[ g b f'] |
  r8 g16[ c'] e'[ g c' e'] r8 g16[ c'] e'[ g c' e'] |

  r8 bes16[ c'] e'[ bes c' e'] r8 bes16[ c'] e'[ bes c' e'] |
  r8 a16[ c'] e'[ a c' e'] r8 a16[ c'] e'[ a c' e'] |
  r8 a16[ c'] ees'[ a c' ees'] r8 a16[ c'] ees'[ a c' ees'] |
  r8 b16[ c'] d'[ b c' d'] r8 b16[ c'] d'[ b c' d'] |
  r8 g16[ b] d'[ g b d'] r8 g16[ b] d'[ g b d'] |
  r8 g16[ c'] e'[ g c' e'] r8 g16[ c'] e'[ g c' e'] |
  r8 g16[ c'] f'[ g c' f'] r8 g16[ c'] f'[ g c' f'] |
  r8 g16[ b] f'[ g b f'] r8 g16[ b] f'[ g b f'] |
  r8 a16[ c'] fis'[ a c' fis'] r8 a16[ c'] fis'[ a c' fis'] |
  r8 g16[ c'] g'[ g c' g'] r8 g16[ c'] g'[ g c' g'] |

  r8 g16[ c'] f'[ g c' f'] r8 g16[ c'] f'[ g c' f'] |
  r8 g16[ b] f'[ g b f'] r8 g16[ b] f'[ g b f'] |
  r8 g16[ bes] e'[ g bes e'] r8 g16[ bes] e'[ g bes e'] |

  \clef "bass"
  r8 f16[ a] c'[ f' c' a] c'[ a f a] f[ d f d] |
  \clef "treble"
  r8 g'16[ b'] d''[ f'' d'' b'] d''[ b' g' b'] d'[ f' e' d'] |
  <e' g' c''>1\fermata\arpeggio \bar "|."
}

%% ---------------------------------------------------------------------------
%% Left hand: sustained bass note under a sounding third/sixth.
%% ---------------------------------------------------------------------------
left = {
  \key c \major
  \time 4/4
  \clef "bass"
  <<
    {
      r16 e'8. ~ e'4 r16 e'8. ~ e'4 |
      r16 d'8. ~ d'4 r16 d'8. ~ d'4 |
      r16 d'8. ~ d'4 r16 d'8. ~ d'4 |
      r16 e'8. ~ e'4 r16 e'8. ~ e'4 |
      r16 e'8. ~ e'4 r16 e'8. ~ e'4 |
      r16 d'8. ~ d'4 r16 d'8. ~ d'4 |
      r16 d'8. ~ d'4 r16 d'8. ~ d'4 |
      r16 c'8. ~ c'4 r16 c'8. ~ c'4 |
      r16 c'8. ~ c'4 r16 c'8. ~ c'4 |

      r16 a8. ~ a4 r16 a8. ~ a4 |
      r16 b8. ~ b4 r16 b8. ~ b4 |
      r16 bes8. ~ bes4 r16 bes8. ~ bes4 |
      r16 a8. ~ a4 r16 a8. ~ a4 |
      r16 aes8. ~ aes4 r16 aes8. ~ aes4 |
      r16 g8. ~ g4 r16 g8. ~ g4 |
      r16 f8. ~ f4 r16 f8. ~ f4 |
      r16 f8. ~ f4 r16 f8. ~ f4 |
      r16 d8. ~ d4 r16 d8. ~ d4 |

      r16 e8. ~ e4 r16 e8. ~ e4 |
      r16 g8. ~ g4 r16 g8. ~ g4 |
      r16 f8. ~ f4 r16 f8. ~ f4 |
      r16 c8. ~ c4 r16 c8. ~ c4 |
      r16 f8. ~ f4 r16 f8. ~ f4 |
      r16 f8. ~ f4 r16 f8. ~ f4 |
      r16 e8. ~ e4 r16 e8. ~ e4 |
      r16 d8. ~ d4 r16 d8. ~ d4 |
      r16 d8. ~ d4 r16 d8. ~ d4 |

      r16 ees8. ~ ees4 r16 ees8. ~ ees4 |
      r16 e8. ~ e4 r16 e8. ~ e4 |
      r16 d8. ~ d4 r16 d8. ~ d4 |
      r16 d8. ~ d4 r16 d8. ~ d4 |
      r16 c8. ~ c4 r16 c8. ~ c4 |

      r16 c8. ~ c4 ~ c2 |
      r16 b,8. ~ b,4 ~ b,2 |
      c1\arpeggio \bar "|."
    }
    \\
    {
      c'2 c'2 |
      c'2 c'2 |
      b2 b2 |
      c'2 c'2 |
      c'2 c'2 |
      c'2 c'2 |
      b2 b2 |
      b2 b2 |
      a2 a2 |

      d2 d2 |
      g2 g2 |
      g2 g2 |
      f2 f2 |
      f2 f2 |
      e2 e2 |
      e2 e2 |
      d2 d2 |
      g,2 g,2 |

      c2 c2 |
      c2 c2 |
      f,2 f,2 |
      fis,2 fis,2 |
      aes,2 aes,2 |
      g,2 g,2 |
      g,2 g,2 |
      g,2 g,2 |
      g,2 g,2 |

      g,2 g,2 |
      g,2 g,2 |
      g,2 g,2 |
      g,2 g,2 |
      c,2 c,2 |

      c,1 |
      c,1 |
      c,1\arpeggio_\fermata
    }
  >>
}

\score {
  \new PianoStaff <<
    \set PianoStaff.connectArpeggios = ##t
    \new Staff = "upper" \right
    \new Staff = "lower" \left
  >>
  \layout { }
  \midi { }
}
