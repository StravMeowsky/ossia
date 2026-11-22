const namespace = 'http://www.w3.org/2000/svg';
const tags = new Set([
  'svg',
  'g',
  'a',
  'path',
  'rect',
  'line',
  'polygon',
  'polyline',
  'circle',
  'ellipse',
  'text',
  'tspan',
  'defs',
  'clipPath',
  'symbol',
  'use',
  'title',
  'desc',
]);
const attributes = new Set([
  'id',
  'viewBox',
  'preserveAspectRatio',
  'transform',
  'd',
  'points',
  'x',
  'y',
  'x1',
  'y1',
  'x2',
  'y2',
  'cx',
  'cy',
  'r',
  'rx',
  'ry',
  'dx',
  'dy',
  'width',
  'height',
  'fill',
  'fill-rule',
  'fill-opacity',
  'stroke',
  'stroke-width',
  'stroke-linecap',
  'stroke-linejoin',
  'stroke-miterlimit',
  'stroke-dasharray',
  'stroke-dashoffset',
  'stroke-opacity',
  'opacity',
  'color',
  'font-family',
  'font-size',
  'font-style',
  'font-weight',
  'text-anchor',
  'textLength',
  'lengthAdjust',
  'dominant-baseline',
  'clip-path',
  'clip-rule',
  'clipPathUnits',
  'display',
  'visibility',
]);
const styles = new Set([
  'color',
  'fill',
  'fill-rule',
  'fill-opacity',
  'stroke',
  'stroke-width',
  'stroke-linecap',
  'stroke-linejoin',
  'stroke-dasharray',
  'stroke-dashoffset',
  'stroke-opacity',
  'opacity',
  'font-family',
  'font-size',
  'font-style',
  'font-weight',
  'text-anchor',
  'white-space',
  'display',
  'visibility',
]);

interface BrowserScheduler {
  yield?: () => Promise<void>;
  postTask?: <T>(
    callback: () => T,
    options: { priority: 'background'; signal: AbortSignal },
  ) => Promise<Awaited<T>>;
}

/** Import inert drawing primitives only, with page-instance-local references. */
export function parseScoreSvg(source: string, prefix: string, signal: AbortSignal) {
  const scheduler = (globalThis as typeof globalThis & { scheduler?: BrowserScheduler }).scheduler;
  const parse = () => importDrawing(source, prefix, signal, scheduler);
  // Yield continuations inherit background priority so page preparation does not outrank input.
  return scheduler?.postTask
    ? scheduler.postTask(parse, { priority: 'background', signal })
    : parse();
}

async function importDrawing(
  source: string,
  prefix: string,
  signal: AbortSignal,
  scheduler: BrowserScheduler | undefined,
) {
  signal.throwIfAborted();
  const parsed = new DOMParser().parseFromString(source, 'image/svg+xml');
  if (
    parsed.querySelector('parsererror') ||
    parsed.documentElement.localName !== 'svg' ||
    parsed.doctype
  )
    throw new Error('Invalid score SVG');
  const reference = (value: string) => {
    if (!/url\s*\(/i.test(value)) return value;
    const match = /^url\(\s*['"]?#([^\s'"()]+)['"]?\s*\)$/.exec(value);
    return match ? `url(#${prefix}${match[1]})` : undefined;
  };
  function* copy(original: Element): Generator<void, SVGElement | undefined> {
    yield;
    if (original.namespaceURI !== namespace || !tags.has(original.localName)) return;
    const node = document.createElementNS(
      namespace,
      original.localName === 'a' ? 'g' : original.localName,
    );
    for (const attr of original.attributes) {
      const name = attr.localName;
      if (name === 'href') {
        if (original.localName === 'use' && /^#[\w.-]+$/.test(attr.value))
          node.setAttribute('href', `#${prefix}${attr.value.slice(1)}`);
      } else if (name === 'style') {
        const style = document.createElementNS(namespace, 'g').style;
        style.cssText = attr.value;
        for (const property of styles) {
          const value = reference(style.getPropertyValue(property));
          if (value) node.style.setProperty(property, value);
        }
      } else if (
        attributes.has(name) ||
        /^data-ossia-(graphic|owners|id|kind|source|staff|hit)$/.test(name)
      ) {
        const value = name === 'id' ? prefix + attr.value : reference(attr.value);
        if (value !== undefined) node.setAttribute(name, value);
      }
    }
    if (node.localName === 'tspan') node.style.whiteSpace = 'pre';
    for (const child of original.childNodes) {
      if (child.nodeType === Node.ELEMENT_NODE) {
        const safe = yield* copy(child as Element);
        if (safe) node.append(safe);
      } else if (child.nodeType === Node.TEXT_NODE) {
        node.append(document.createTextNode(child.textContent ?? ''));
      }
    }
    return node;
  }
  const work = copy(parsed.documentElement);
  let deadline = performance.now() + 4;
  let step = work.next();
  while (!step.done) {
    signal.throwIfAborted();
    if (performance.now() >= deadline) {
      if (scheduler?.yield) await scheduler.yield();
      else await new Promise<void>((resolve) => setTimeout(resolve, 0));
      deadline = performance.now() + 4;
    }
    step = work.next();
  }
  signal.throwIfAborted();
  const svg = step.value;
  if (!(svg instanceof SVGSVGElement)) throw new Error('Missing score SVG root');
  svg.classList.add('score-image');
  svg.style.color = 'black';
  svg.setAttribute('width', '100%');
  svg.setAttribute('height', '100%');
  return svg;
}

/** Preserve each leaf's paint and restore it verbatim when selection changes. */
export function scorePainter(svg: SVGSVGElement) {
  const originals = new Map<SVGElement, string | null>();
  const groups = [...svg.querySelectorAll<SVGGElement>('[data-ossia-graphic]')];
  let painted = false;
  return (
    ids: Set<string>,
    annotationColors: ReadonlyMap<string, string>,
    playbackIds: Set<string>,
  ) => {
    if (!painted && !ids.size && !annotationColors.size && !playbackIds.size) return;
    painted = false;
    for (const [leaf, style] of originals) {
      if (style === null) leaf.removeAttribute('style');
      else leaf.setAttribute('style', style);
    }
    originals.clear();
    for (const group of groups) {
      const id = group.dataset.ossiaGraphic!;
      const selected = ids.has(id);
      const color = annotationColors.get(id);
      const playing = playbackIds.has(id);
      group.classList.toggle('element-selection', selected || color !== undefined);
      group.classList.toggle('playback-active', playing);
      if (!selected && color === undefined && !playing) continue;
      painted = true;
      const paintColor = playing ? 'var(--playback)' : (color ?? 'var(--sel)');
      for (const leaf of group.querySelectorAll<SVGElement>(
        'path,rect,line,polygon,polyline,circle,ellipse,text,tspan,use',
      )) {
        if (originals.has(leaf)) continue;
        originals.set(leaf, leaf.getAttribute('style'));
        const paint = getComputedStyle(leaf);
        const fill = paint.fill,
          stroke = paint.stroke;
        leaf.style.setProperty('color', paintColor, 'important');
        if (fill !== 'none' && fill !== 'rgba(0, 0, 0, 0)' && fill !== 'transparent')
          leaf.style.setProperty('fill', paintColor, 'important');
        if (stroke !== 'none' && stroke !== 'rgba(0, 0, 0, 0)' && stroke !== 'transparent')
          leaf.style.setProperty('stroke', paintColor, 'important');
      }
    }
  };
}
