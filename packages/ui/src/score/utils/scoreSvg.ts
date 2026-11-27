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

const PAINTED_LEAVES = 'path,rect,line,polygon,polyline,circle,ellipse,text,tspan,use';

/** Whether a computed fill or stroke actually draws something. */
function paints(value: string) {
  return value !== 'none' && value !== 'rgba(0, 0, 0, 0)' && value !== 'transparent';
}

/** Preserve each leaf's paint and restore it verbatim when selection changes. */
export function scorePainter(svg: SVGSVGElement) {
  const originals = new Map<SVGElement, string | null>();
  const groups = [...svg.querySelectorAll<SVGGElement>('[data-ossia-graphic]')];
  let painted = false;
  return (ids: Set<string>, annotationColors: ReadonlyMap<string, string>) => {
    if (!painted && !ids.size && !annotationColors.size) return;
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
      group.classList.toggle('element-selection', selected || color !== undefined);
      if (!selected && color === undefined) continue;
      painted = true;
      const paintColor = color ?? 'var(--sel)';
      for (const leaf of group.querySelectorAll<SVGElement>(PAINTED_LEAVES)) {
        if (originals.has(leaf)) continue;
        originals.set(leaf, leaf.getAttribute('style'));
        const paint = getComputedStyle(leaf);
        const fill = paint.fill,
          stroke = paint.stroke;
        leaf.style.setProperty('color', paintColor, 'important');
        if (paints(fill)) leaf.style.setProperty('fill', paintColor, 'important');
        if (paints(stroke)) leaf.style.setProperty('stroke', paintColor, 'important');
      }
    }
  };
}

/**
 * Draw playback highlights as recoloured copies in a separate small SVG above the page.
 *
 * Restyling leaves inside a large page drawing makes WebKit restyle and repaint the whole page
 * for hundreds of milliseconds, which delays the next click. The page drawing is only read here.
 */
export function playbackLayer(svg: SVGSVGElement) {
  const layer = document.createElementNS(namespace, 'svg');
  layer.classList.add('score-playback-layer');
  layer.setAttribute('aria-hidden', 'true');
  layer.setAttribute('width', '100%');
  layer.setAttribute('height', '100%');
  for (const name of ['viewBox', 'preserveAspectRatio']) {
    const value = svg.getAttribute(name);
    if (value !== null) layer.setAttribute(name, value);
  }
  const groups = new Map<string, SVGGElement[]>();
  for (const group of svg.querySelectorAll<SVGGElement>('[data-ossia-graphic]')) {
    const id = group.dataset.ossiaGraphic!;
    const list = groups.get(id);
    if (list) list.push(group);
    else groups.set(id, [group]);
  }
  // Copy one graphic with its ancestors' transforms and inherited attributes.
  const copy = (group: SVGGElement) => {
    const clone = group.cloneNode(true) as SVGGElement;
    // Paint is decided from the page's computed style; writes go only to detached copies.
    const sources = group.querySelectorAll<SVGElement>(PAINTED_LEAVES);
    const leaves = clone.querySelectorAll<SVGElement>(PAINTED_LEAVES);
    sources.forEach((source, index) => {
      const leaf = leaves[index]!;
      const paint = getComputedStyle(source);
      leaf.style.setProperty('color', 'var(--playback)', 'important');
      if (paints(paint.fill)) leaf.style.setProperty('fill', 'var(--playback)', 'important');
      if (paints(paint.stroke)) leaf.style.setProperty('stroke', 'var(--playback)', 'important');
    });
    for (const node of [clone, ...clone.querySelectorAll('[id]')]) node.removeAttribute('id');
    clone.setAttribute('class', 'playback-active');
    let wrapped: SVGElement = clone;
    for (
      let parent: Element | null = group.parentElement;
      parent && parent !== svg;
      parent = parent.parentElement
    ) {
      const shell = parent.cloneNode(false) as SVGElement;
      for (const name of shell.getAttributeNames())
        if (name === 'id' || name === 'class' || name.startsWith('data-ossia-'))
          shell.removeAttribute(name);
      shell.append(wrapped);
      wrapped = shell;
    }
    return wrapped;
  };
  let shown = new Set<string>();
  return {
    element: layer,
    paint(ids: Set<string>) {
      if (ids.size === shown.size && [...ids].every((id) => shown.has(id))) return;
      shown = new Set(ids);
      layer.replaceChildren(...[...ids].flatMap((id) => (groups.get(id) ?? []).map(copy)));
    },
  };
}
