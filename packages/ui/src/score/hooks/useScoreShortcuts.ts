import { useEffect } from 'react';
import type { Dispatch, SetStateAction } from 'react';

import { isEditableKeyboardTarget, isInteractiveKeyboardTarget } from '../utils/annotationInput.ts';
import { focusScorePage } from '../utils/scoreFocus.ts';
import type { DrawingMode } from './useDrawingInput.ts';

interface ScoreShortcutOptions {
  annotationAvailable: boolean;
  annotationMode: boolean;
  drawingMode: DrawingMode;
  playbackEnabled: boolean;
  playbackPhase: string;
  setAnnotationMode: Dispatch<SetStateAction<boolean>>;
  setDrawingMode: Dispatch<SetStateAction<DrawingMode>>;
  togglePlayback: () => Promise<void>;
  clearPlaybackHighlight: () => void;
  seekMeasure: (direction: 'previous' | 'next') => boolean;
  auditionAdjacent: (
    direction: 'previous' | 'next',
    options?: { simultaneous?: boolean },
  ) => boolean;
}

/** Registers the score page's annotation, playback and measure-navigation shortcuts. */
export function useScoreShortcuts({
  annotationAvailable,
  annotationMode,
  drawingMode,
  playbackEnabled,
  playbackPhase,
  setAnnotationMode,
  setDrawingMode,
  togglePlayback,
  clearPlaybackHighlight,
  seekMeasure,
  auditionAdjacent,
}: ScoreShortcutOptions) {
  useEffect(() => {
    if (!annotationAvailable) {
      setAnnotationMode(false);
      return;
    }
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        document.querySelector('dialog[open]') ||
        event.defaultPrevented ||
        event.isComposing ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey ||
        event.key.toLowerCase() !== 'e' ||
        isEditableKeyboardTarget(event.target)
      ) {
        return;
      }
      event.preventDefault();
      setAnnotationMode((current) => !current);
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [annotationAvailable, setAnnotationMode]);

  useEffect(() => {
    if (!annotationMode && !drawingMode) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        document.querySelector('dialog[open], [data-radix-popper-content-wrapper]') ||
        event.defaultPrevented ||
        event.isComposing ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey ||
        event.key !== 'Escape'
      ) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      setAnnotationMode(false);
      setDrawingMode(null);
      focusScorePage();
    };
    document.addEventListener('keydown', onKeyDown, true);
    return () => document.removeEventListener('keydown', onKeyDown, true);
  }, [annotationMode, drawingMode, setAnnotationMode, setDrawingMode]);

  useEffect(() => {
    if (!playbackEnabled || annotationMode || playbackPhase === 'playing') return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        document.querySelector('dialog[open]') ||
        event.defaultPrevented ||
        event.isComposing ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey ||
        event.key !== 'Escape' ||
        isInteractiveKeyboardTarget(event.target)
      ) {
        return;
      }
      clearPlaybackHighlight();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [annotationMode, clearPlaybackHighlight, playbackEnabled, playbackPhase]);

  useEffect(() => {
    if (!playbackEnabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        document.querySelector('dialog[open]') ||
        event.defaultPrevented ||
        event.isComposing ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.altKey ||
        event.shiftKey ||
        event.key !== ' ' ||
        isInteractiveKeyboardTarget(event.target)
      ) {
        return;
      }
      event.preventDefault();
      void togglePlayback();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [playbackEnabled, togglePlayback]);

  useEffect(() => {
    if (!playbackEnabled) return;
    const onKeyDown = (event: KeyboardEvent) => {
      const featureToolbarTarget =
        event.target instanceof HTMLElement &&
        Boolean(event.target.closest('.score-feature-toolbar'));
      if (
        document.querySelector('dialog[open]') ||
        event.defaultPrevented ||
        event.isComposing ||
        event.repeat ||
        event.metaKey ||
        event.ctrlKey ||
        event.shiftKey ||
        (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') ||
        (isInteractiveKeyboardTarget(event.target) && !featureToolbarTarget)
      ) {
        return;
      }
      const direction = event.key === 'ArrowRight' ? 'next' : 'previous';
      const handled =
        playbackPhase === 'playing'
          ? seekMeasure(direction)
          : auditionAdjacent(direction, { simultaneous: event.altKey });
      if (handled) event.preventDefault();
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, [auditionAdjacent, playbackEnabled, playbackPhase, seekMeasure]);
}
