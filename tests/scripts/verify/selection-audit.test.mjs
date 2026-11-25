import assert from 'node:assert/strict';

import { test } from 'vite-plus/test';

import { auditSelection } from '../../../scripts/verify/selection-audit.mjs';

function fixture() {
  const staff = { x0: 5, x1: 45, top: 10, bot: 14 };
  const system = { barXs: [5, 25, 45], staves: [staff] };
  const locators = [1, 2].map((number) => ({
    staffIndex: 0,
    measureStart: number,
    measureEnd: number,
    voice: null,
  }));
  const bounds = (x0, x1) => ({ x0, x1, y0: staff.top, y1: staff.bot });
  const regions = locators.map((locator, i) => ({
    locator,
    systemIndex: 0,
    bounds: bounds(5 + i * 20, 25 + i * 20),
  }));
  const objects = [
    { kind: 'NoteHead', staffIndex: 0, bbox: bounds(10, 12) },
    { kind: 'MultiMeasureRest', staffIndex: 0, bbox: bounds(30, 32) },
  ].map((object, i) => ({
    ...object,
    span: { byteStart: 10 + i * 10, byteEnd: 12 + i * 10, start: { line: i + 2 } },
  }));
  return {
    geometry: { sourceHash: 'source', pages: [{ page: 1, elements: [], regions }] },
    layout: {
      sourceHash: 'source',
      pages: [{ page: 1, staffSpace: 1, systems: [system], objects }],
    },
    selection: {
      reliable: true,
      staves: [{ voices: ['upper'] }],
      voices: [{ name: 'upper', musicSpan: { byteStart: 5, byteEnd: 30 } }],
      pages: [
        locators.map((locator, i) => ({
          target: { kind: 'measure', locator },
          spans: [{ byteStart: objects[i].span.byteStart, byteEnd: objects[i].span.byteEnd }],
        })),
      ],
    },
  };
}

test('audits each engraved interval with the actual hit function and source spans', () => {
  const bundle = fixture();
  const [first] = bundle.geometry.pages[0].regions;
  bundle.geometry.pages[0].elements.push({
    bounds: { x0: 13, x1: 17, y0: 11, y1: 13 },
    priority: 0,
    hitBounds: [],
    region: { target: { kind: 'element', elementId: 'obstruction' }, spans: [] },
  });
  const report = auditSelection(bundle);
  assert.equal(report.cells, 2);
  assert.deepEqual(report.issues, []);
  assert.deepEqual(report.clicks[0].target.locator, first.locator);
});

test('reports a missing click target even if the source measure is indexed', () => {
  const bundle = fixture();
  bundle.geometry.pages[0].regions.pop();
  assert.deepEqual(auditSelection(bundle).issues, [
    { kind: 'unselectable-measure', page: 1, systemIndex: 0, column: 1, staffIndex: 0 },
  ]);
});

test('distinguishes inaccessible source ranges from ranges pointing at other music', () => {
  const bundle = fixture();
  const [first, second] = bundle.selection.pages[0];
  first.spans = [];
  second.spans = [{ byteStart: 10, byteEnd: 12 }];
  assert.deepEqual(auditSelection(bundle).issues, [
    {
      kind: 'unresolved-measure-target',
      page: 1,
      systemIndex: 0,
      column: 0,
      staffIndex: 0,
      locator: first.target.locator,
    },
    {
      kind: 'wrong-measure-source',
      page: 1,
      systemIndex: 0,
      column: 1,
      staffIndex: 0,
      locator: second.target.locator,
      lines: [3],
    },
  ]);
});

test('allows cross-staff and hit-only objects without demanding their source in the visible staff', () => {
  const bundle = fixture();
  const [first] = bundle.layout.pages[0].objects;
  const otherVoice = { ...first, span: { byteStart: 32, byteEnd: 34, start: { line: 5 } } };
  const hitOnly = {
    ...first,
    hitOnly: true,
    span: { byteStart: 19, byteEnd: 20, start: { line: 4 } },
  };
  bundle.layout.pages[0].objects.push(otherVoice, hitOnly);
  assert.deepEqual(auditSelection(bundle).issues, []);
});

test('refuses mismatched snapshots and explains absent geometry or barlines', () => {
  const bundle = fixture();
  assert.throws(
    () => auditSelection({ ...bundle, geometry: { ...bundle.geometry, sourceHash: 'old' } }),
    /mismatched selection render/,
  );
  bundle.selection.reliable = false;
  bundle.selection.measureCountError = 'ambiguous';
  bundle.layout.pages[0].systems[0].barXs = [5, 5.1];
  assert.deepEqual(auditSelection(bundle).issues, [
    { kind: 'unreliable-musical-identity', reason: 'ambiguous' },
    { kind: 'empty-measure-grid' },
  ]);
  bundle.layout.pages[0].systems[0].barXs = [5];
  assert.equal(auditSelection(bundle).issues[1].kind, 'missing-barlines');
  bundle.geometry.pages = [];
  assert.equal(auditSelection(bundle).issues[1].kind, 'missing-page-grid');
});
