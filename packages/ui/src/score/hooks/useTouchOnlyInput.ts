import { useEffect, useState } from 'react';

const TOUCH_ONLY_QUERY = '(hover: none) and (pointer: coarse)';

/** Tracks whether the primary input is a coarse pointer without hover. */
export function useTouchOnlyInput() {
  const [touchOnly, setTouchOnly] = useState(() => matchMedia(TOUCH_ONLY_QUERY).matches);

  useEffect(() => {
    const query = matchMedia(TOUCH_ONLY_QUERY);
    const update = () => setTouchOnly(query.matches);
    update();
    query.addEventListener('change', update);
    return () => query.removeEventListener('change', update);
  }, []);

  return touchOnly;
}
