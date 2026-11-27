import type { VerovioToolkit } from 'verovio/esm';

let toolkit: Promise<VerovioToolkit> | null = null;

/** Load Verovio once, on first use: the engraving preview is only needed while entering notes. */
function loadToolkit() {
  toolkit ??= (async () => {
    const [{ default: createVerovioModule }, { VerovioToolkit }] = await Promise.all([
      import('verovio/wasm'),
      import('verovio/esm'),
    ]);
    return new VerovioToolkit(await createVerovioModule());
  })();
  toolkit.catch(() => {
    toolkit = null;
  });
  return toolkit;
}

/** Engrave MEI into SVG pages that fill `width` CSS pixels. */
export async function engraveMei(mei: string, width: number, scale = 38): Promise<string[]> {
  const verovio = await loadToolkit();
  verovio.setOptions({
    scale,
    pageWidth: Math.max(600, Math.round((width * 100) / scale)),
    adjustPageHeight: true,
    breaks: 'auto',
    footer: 'none',
    header: 'none',
  });
  if (!verovio.loadData(mei)) throw new Error('preview could not be engraved');
  const pages: string[] = [];
  for (let page = 1; page <= verovio.getPageCount(); page++) {
    pages.push(verovio.renderToSVG(page));
  }
  return pages;
}

/** Note, chord and rest ids by voice, in score order, for moving the cursor with the arrow keys. */
export function voiceOrder(mei: string): Map<string, string[]> {
  const xml = new DOMParser().parseFromString(mei, 'application/xml');
  const voices = new Map<string, string[]>();
  for (const staff of Array.from(xml.getElementsByTagName('staff'))) {
    for (const layer of Array.from(staff.getElementsByTagName('layer'))) {
      const key = `${staff.getAttribute('n')}:${layer.getAttribute('n')}`;
      const ids = voices.get(key) ?? [];
      for (const element of Array.from(layer.getElementsByTagName('*'))) {
        const id = element.getAttribute('xml:id');
        const name = element.localName;
        const inChord = element.parentElement?.localName === 'chord';
        if (id && !inChord && ['note', 'chord', 'rest', 'mRest'].includes(name)) ids.push(id);
      }
      voices.set(key, ids);
    }
  }
  return voices;
}

/** Source byte offset named by a preview element id such as `o123` or `o123-2`. */
export function offsetOfId(id: string): number | null {
  const match = /^o(\d+)(?:-\d+)?$/.exec(id);
  return match ? Number(match[1]) : null;
}
