% Mutopia Project public-domain edition; typeset by Jakob Bagterp.
% Source: https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=64
% The typesetter placed this edition in the public domain.

\header {
	subtitle = "Guitar · standard tuning"
	title =		"Etude"
	composer =	"Fernando Sor"
	opus =		"Opus 35, No. 14"
	% instrument =	"Guitar"

	% Mutopia file info:

	mutopiatitle =	"No. 14: Etude"
	mutopiacomposer =	"SorF"
	mutopiaopus =	"Opus 35, No. 14"
	mutopiainstrument =	"Guitar"
	style =		"Classical"
	source =	"Unknown"
	filename =	"Sor_Etude_Opus35_14.ly"
	enteredby =	"Jakob Bagterp"
	maintainer =	"Jakob Bagterp"
	maintainerEmail =	"jakob_bagterp@hotmail.com"

	tagline = ##f
}

% This score has been optimized for A4 paper layout. /Jakob Bagterp

\version "2.26.0"
\include "deutsch.ly"

global =  {
	\key a \minor
	\time 4/4
}

melody =  \relative c'' {
	\tempo "Andante"
	\global
	\stemUp

	\repeat volta 2 {

		\partial 4
		c8.^1 h16^0 |

		% 1
		a8.^2 h16^0 c8.^1 d16^4 e4^0 f^2^1 |
		h,^0 e2^0 d4^1^4 |
		c^2^1 h^0^0 a8.^3^2 c16^1 \slurDown \tuplet 3/2 { c8(  h) a^3 } \break |
		gis4^1 h r \slashedGrace d8 c8. h16 |

		% 5
		a8. h16 c8. d16 e4 fis^1^2 |
		g4.^4 f!8^1 e4^0 c |
		a^2 d^4 h8. c16^1 d8.^4 h16 |
		c2^1 r4
	}

	\repeat volta 2 {

		g'8.^4 f16^1 |
		e4^0 e8. d16^4 c4^1 c |

		% 10
		a'4..^4 g16^4 f4^1 f8. e16^0 |
		d4^1 d8. c16^1 h4^0 h |
		g'4..^4 f16^1 e4^0 e8. e16 |
		d4^1^4 e8. e16 c4^2^1 e8. e16 |
		h4^1^0 e8. e16 a,4^2 e'8. e16 |

		% 15
		d4 e8. e16 c4 e8. c16^1 |
		<gis h^1^0>4 e' r \slashedGrace d8 c8. h16 |
		a8. h16 c8. d16 e4 f |
		h, e2 d4^4 |
		c h a8. c16 \tuplet 3/2 { c8(  h) a } |

		% 20
		gis4 h r c8. h16 |
		a8. h16 c8. d16 e4 a,^1 |
		b8.^3 d16^4 f2^1 d8. b16 |
		a8.^3 c16^1 e2 gis,8.^1 h16 |
		a4^3 <e c'^2^1> a
	}
}

bass =  \relative c' {
	\global
	\stemDown
	\repeat volta 2 {

		\partial 4
		r4 |

		% 1
		a2 r4 <d a'> |
		<e_1_2 gis>2 e,4 gis' |
		a g! f d |
		e2_2 e,4 r |

		% 5
		a2 r4 <d c'> |
		<g h>2 c4_1 e,_2 |
		f2_3 g |
		c,8._3 g'16 e8._2 g16 c,4
	}

	\repeat volta 2 {

		r4 |
		r2 r4 a'8._2 g16_0 |

		% 10
		f4_1 f8. e16_2 d4_0 r |
		h'4..\3_2 a16_2 g4_0 g8. f16_3 |
		e4_2 e8. d16_0 c4_3 r |
		<h_2 gis'>2 <a a'> |
		<d gis> <c_3 a'> |

		% 15
		<h gis'> <a a'> |
		e4 e'_2 e, r |
		a4 r r <d a'> |
		<< { e2_3_2 ~ e_1 } { a gis } >> |
		a4 g! f d |

		% 20
		e2 e,4 r |
		a4 r r c_2 |
		d2_0 d |
		e_2 e, |
		a2 a4
	}
}

% Finger numbers in the source describe the notation, not TAB string choices.
stripFingerings = #(define-music-function (music) (ly:music?)
  (music-map
    (lambda (m)
      (if (music-is-of-type? m 'event-chord)
          (set! (ly:music-property m 'elements)
            (filter (lambda (e) (not (music-is-of-type? e 'fingering-event)))
              (ly:music-property m 'elements))))
      (if (music-is-of-type? m 'note-event)
          (set! (ly:music-property m 'articulations)
            (filter (lambda (e) (not (music-is-of-type? e 'fingering-event)))
              (ly:music-property m 'articulations))))
      m)
    music))
% Guitar notation sounds one octave below its written pitch.
\score {
  \new StaffGroup <<
    \new Staff <<
      \new Voice = "melody" { \melody }
      \new Voice = "bass" { \bass }
    >>
    \new TabStaff <<
      \new TabVoice = "melody" { \transpose c c, \stripFingerings \melody }
      \new TabVoice = "bass" { \transpose c c, \stripFingerings \bass }
    >>
  >>
  \layout { }
}
% A separate MIDI-only score keeps playback to one copy of each voice.
\score {
  \new Staff <<
    \set Staff.midiInstrument = "acoustic guitar (nylon)"
    \transposition c
    \new Voice = "melody" { \melody }
    \new Voice = "bass" { \bass }
  >>
  \midi { \tempo 4 = 80 }
}
