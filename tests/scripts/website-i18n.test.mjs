import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

import { parse } from 'parse5';
import { test } from 'vite-plus/test';

test('every website translation has a binding and both languages', async () => {
  const html = parse(await readFile('website/index.html', 'utf8'));
  const messages = JSON.parse(await readFile('website/assets/translations.json', 'utf8'));
  const referenced = new Set();
  let mains = 0;
  function visit(node) {
    if (node.tagName === 'main') mains++;
    for (const attribute of node.attrs ?? []) {
      if (['data-i18n', 'data-i18n-alt', 'data-i18n-aria-label'].includes(attribute.name)) {
        referenced.add(attribute.value);
        const message = messages[attribute.value];
        assert.ok(message, `Missing translation: ${attribute.value}`);
        assert.ok(message.en && message.zh, `Incomplete translation: ${attribute.value}`);
      }
    }
    for (const child of node.childNodes ?? []) visit(child);
  }
  visit(html);
  assert.equal(mains, 1);
  const compare = (a, b) => a.localeCompare(b);
  assert.deepEqual([...referenced].sort(compare), Object.keys(messages).sort(compare));
});
