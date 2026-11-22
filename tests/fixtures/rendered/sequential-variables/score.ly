\version "2.26.0"
intro = { \time 4/4 c'1 | d'1 | }
main = { e'1 | f'1 | g'1 | }
one = { a'1 | b'1 | }
two = { c''1 | d''1 | }
\score {
  \new Staff {
    \intro
    \repeat volta 2 { \main }
    \alternative { \one \two }
  }
  \layout { }
  \midi { }
}
