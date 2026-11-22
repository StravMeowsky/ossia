/** Moves focus back to the score page after an overlay closes. */
export function focusScorePage() {
  document.querySelector<HTMLElement>('.score-app')?.focus({ preventScroll: true });
}
