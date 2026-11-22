/** Platform-specific modifier label shown by the annotation toolbar. */
export type AnnotationModifier = 'Command' | 'Ctrl';

/** Resolves the annotation grouping key from the browser platform identifier. */
export function resolveAnnotationModifier(
  platform = globalThis.navigator?.platform ?? '',
): AnnotationModifier {
  return /Mac|iPhone|iPad|iPod/u.test(platform) ? 'Command' : 'Ctrl';
}

/** Checks the same platform-specific modifier used by the toolbar hint. */
export function isAnnotationModifierPressed(event: Pick<MouseEvent, 'metaKey' | 'ctrlKey'>) {
  return resolveAnnotationModifier() === 'Command' ? event.metaKey : event.ctrlKey;
}

/** Reports whether keyboard shortcuts should leave the event to form controls. */
export function isEditableKeyboardTarget(target: EventTarget | null) {
  return (
    target instanceof HTMLElement &&
    Boolean(target.closest('input, textarea, select, [contenteditable="true"]'))
  );
}

/** Keeps playback shortcuts from stealing focused control or menu activation. */
export function isInteractiveKeyboardTarget(target: EventTarget | null) {
  return (
    isEditableKeyboardTarget(target) ||
    (target instanceof HTMLElement &&
      Boolean(
        target.closest(
          'button, a, summary, [role="button"], [role="menuitem"], [role="menuitemradio"]',
        ),
      ))
  );
}
