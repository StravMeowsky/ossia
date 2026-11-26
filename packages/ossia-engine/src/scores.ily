% Ossia multi-score timeline: record every \score's music length in order.
% The MIDI pass prints the lengths so the per-score MIDI files can be joined;
% the engraving pass offsets each score's moments by the scores before it.
#(define ossia-score-lengths '())
#(define ossia-toplevel-score-handler toplevel-score-handler)
#(define (toplevel-score-handler score)
   (let ((length (ly:music-length (ly:score-music score))))
     (set! ossia-score-lengths (append ossia-score-lengths (list length)))
     (ly:message "ossia-score-length:~a/~a"
       (ly:moment-main-numerator length) (ly:moment-main-denominator length))
     (ossia-toplevel-score-handler score)))
