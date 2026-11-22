import { useCallback, useEffect, useRef, useState } from 'react';

/** Owns native fullscreen synchronization with an immersive CSS fallback. */
export function useScoreFullscreen() {
  const [active, setActive] = useState(false);
  const nativeRequested = useRef(false);

  useEffect(() => {
    const sync = () => {
      if (document.fullscreenElement) {
        setActive(true);
        return;
      }
      if (nativeRequested.current) {
        nativeRequested.current = false;
        setActive(false);
      }
    };
    sync();
    document.addEventListener('fullscreenchange', sync);
    return () => {
      document.removeEventListener('fullscreenchange', sync);
      if (document.fullscreenElement) void document.exitFullscreen().catch(() => {});
    };
  }, []);

  const enter = useCallback(() => {
    setActive(true);
    const root = document.documentElement;
    // Mobile fullscreen gestures can interfere with score panning.
    const isMobile =
      /Android|iPhone|iPad|iPod|Mobile/u.test(navigator.userAgent) ||
      (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1);
    if (isMobile) return;
    if (typeof root.requestFullscreen !== 'function' || document.fullscreenEnabled === false)
      return;
    nativeRequested.current = true;
    void root.requestFullscreen().catch(() => {
      nativeRequested.current = false;
    });
  }, []);

  const exit = useCallback(() => {
    setActive(false);
    nativeRequested.current = false;
    if (document.fullscreenElement) void document.exitFullscreen().catch(() => {});
  }, []);

  const toggle = useCallback(() => {
    if (active) exit();
    else enter();
  }, [active, enter, exit]);

  return { active, exit, toggle };
}
