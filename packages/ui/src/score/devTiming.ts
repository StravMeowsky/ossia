// TEMP(timing): measure click → sound → paint in the running score window.
import { scoreAudio } from './scoreAudio.ts';

const log: unknown[][] = [];
const T = () => Math.round(performance.now() * 10) / 10;
const ua = navigator.userAgent.includes('Chrome') ? 'chromium' : 'webkit';
log.push(['start', T(), ua, location.search]);
document.addEventListener('pointerdown', (e) => log.push(['down', T(), +e.timeStamp.toFixed(1)]), true);
document.addEventListener('click', () => log.push(['click', T()]), true);
// TEMP(timing): A/B the native mouse-press default action. Odd presses keep it, even presses cancel it.
let presses = 0;
document.addEventListener(
  'mousedown',
  (e) => {
    presses += 1;
    const cancel = presses % 2 === 0;
    if (cancel) e.preventDefault();
    const target = e.target as Element;
    log.push(['mdown', T(), +e.timeStamp.toFixed(1), cancel, document.activeElement?.tagName ?? null, getSelection()?.type ?? null, target.tagName, target.getAttribute('class')]);
  },
  true,
);
// Bubble listeners on window run after every page handler, so each '-end' marks the end of a dispatch.
for (const type of ['pointerdown', 'mousedown', 'pointerup', 'mouseup'] as const)
  window.addEventListener(type, (e) => {
    log.push([`${type}-end`, T(), +e.timeStamp.toFixed(1)]);
    setTimeout(() => log.push([`${type}-next`, T()]), 0);
  });
document.addEventListener('focusin', (e) => log.push(['focus', T(), (e.target as Element).tagName]), true);
document.addEventListener('selectionchange', () => log.push(['selection', T(), getSelection()?.type ?? null]));
const start = Reflect.get(AudioBufferSourceNode.prototype, 'start') as AudioBufferSourceNode['start'];
AudioBufferSourceNode.prototype.start = function (this: AudioBufferSourceNode, when = 0, ...rest) {
  const context = this.context as AudioContext;
  log.push(['sound', T(), context.state, +(when - context.currentTime).toFixed(3), context.baseLatency, context.outputLatency ?? null]);
  start.call(this, when, ...rest);
};
for (const name of ['ready', 'load', 'resume', 'playNow', 'play', 'stopAll'] as const) {
  const original = Reflect.get(scoreAudio, name) as (...args: unknown[]) => unknown;
  Reflect.set(scoreAudio, name, function (this: unknown, ...args: unknown[]) {
    const t = T();
    const result = original.apply(scoreAudio, args);
    if (result instanceof Promise)
      void result.then(() => log.push([name, t, 'done', +(T() - t).toFixed(1)]));
    else log.push([name, t, typeof result === 'boolean' ? result : 'sync', +(T() - t).toFixed(1)]);
    return result;
  });
}
new MutationObserver((records) => {
  if (records.some((r) => (r.target as Element).classList?.contains('score-playback-layer'))) log.push(['paint', T()]);
}).observe(document.documentElement, { childList: true, subtree: true });
try {
  new PerformanceObserver((list) => {
    for (const entry of list.getEntries()) log.push(['long', Math.round(entry.startTime), Math.round(entry.duration)]);
  }).observe({ type: 'longtask' });
} catch {
  log.push(['no-longtask']);
}
let frame = T();
const tick = () => {
  const now = T();
  if (now - frame > 50) log.push(['frame-gap', frame, +(now - frame).toFixed(1)]);
  frame = now;
  requestAnimationFrame(tick);
};
requestAnimationFrame(tick);
setInterval(() => {
  if (!log.length) return;
  const body = JSON.stringify(log.splice(0));
  void fetch('/__ossia-timing', { method: 'POST', body }).catch(() => {});
}, 1000);
