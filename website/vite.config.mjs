import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { parse, serialize } from 'parse5';

const root = fileURLToPath(new URL('.', import.meta.url));

function websiteHtml() {
  return {
    name: 'ossia-website-html',
    enforce: 'post',
    transformIndexHtml: {
      order: 'pre',
      handler(html, context) {
        if (!context.bundle && !context.server) {
          const language = readFileSync(new URL('./assets/language.js', import.meta.url), 'utf8');
          return html.replace(
            '<script src="./assets/language.js"></script>',
            `<script>\n${language}\n</script>`,
          );
        }
        return html;
      },
    },
    generateBundle(_, bundle) {
      const page = bundle['index.html'];
      if (!page || page.type !== 'asset') throw new Error('Missing built website index.html');
      const document = parse(String(page.source));
      function visit(node) {
        if (node.tagName === 'a' && node.attrs.some((attr) => attr.name === 'data-lightbox')) {
          const image = node.childNodes?.find((child) => child.tagName === 'img');
          const src = image?.attrs.find((attr) => attr.name === 'src')?.value;
          const href = node.attrs.find((attr) => attr.name === 'href');
          if (!src || !href) throw new Error('Lightbox link must contain an image and href');
          href.value = src;
        }
        for (const child of node.childNodes ?? []) visit(child);
      }
      visit(document);
      page.source = serialize(document);
    },
  };
}

export default {
  root,
  base: './',
  plugins: [websiteHtml()],
  build: {
    outDir: '../dist/website',
    emptyOutDir: true,
    assetsInlineLimit: 0,
    rolldownOptions: {
      output: {
        assetFileNames: 'assets/[name]-[hash:16][extname]',
        chunkFileNames: 'assets/[name]-[hash:16].js',
        entryFileNames: 'assets/[name]-[hash:16].js',
      },
    },
  },
};
