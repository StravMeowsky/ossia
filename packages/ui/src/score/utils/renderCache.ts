import type { RenderGeometry, RenderReference, ScorePlayback } from '../../generated/protocol.ts';
import type { Api } from './api.ts';

const VERSION_LIMIT = 24;
const GEOMETRY_LIMIT = 24;
const PLAYBACK_LIMIT = 24;
const PAGE_LIMIT = 128;
const PAGE_BYTES = 32 * 1024 * 1024;

/** Small insertion-ordered LRU used for immutable render-derived data. */
class RenderCache<K, V> {
  private readonly values = new Map<K, V>();
  private readonly limit: number;
  private size = 0;
  private readonly byteLimit: number;
  private readonly bytes: (value: V) => number;

  constructor(limit: number, byteLimit = Infinity, bytes: (value: V) => number = () => 0) {
    this.limit = limit;
    this.byteLimit = byteLimit;
    this.bytes = bytes;
  }

  get(key: K) {
    const value = this.values.get(key);
    if (value === undefined) return undefined;
    this.values.delete(key);
    this.values.set(key, value);
    return value;
  }

  set(key: K, value: V) {
    const previous = this.values.get(key);
    if (previous !== undefined) this.size -= this.bytes(previous);
    this.values.delete(key);
    this.values.set(key, value);
    this.size += this.bytes(value);
    while (this.values.size > this.limit || this.size > this.byteLimit) {
      const oldest = this.values.keys().next();
      if (oldest.done) break;
      this.size -= this.bytes(this.values.get(oldest.value)!);
      this.values.delete(oldest.value);
    }
  }
}

const versions = new RenderCache<string, RenderReference>(VERSION_LIMIT);
const geometry = new RenderCache<string, RenderGeometry>(GEOMETRY_LIMIT);
const playback = new RenderCache<string, ScorePlayback>(PLAYBACK_LIMIT);
const pages = new RenderCache<string, string>(
  PAGE_LIMIT,
  PAGE_BYTES,
  (source) => source.length * 2,
);
const versionRequests = new Map<string, Promise<RenderReference>>();
const geometryRequests = new Map<string, Promise<RenderGeometry>>();
const playbackRequests = new Map<string, Promise<ScorePlayback>>();
interface PageRequest {
  key: string;
  priority: number;
  signals: Set<AbortSignal>;
  started: boolean;
  controller: AbortController;
  promise: Promise<string>;
  run: () => Promise<void>;
  cancel: () => void;
}
const pageRequests = new Map<string, PageRequest>();
const pageQueue: PageRequest[] = [];
let activePages = 0;

function drainPages() {
  pageQueue.sort((a, b) => a.priority - b.priority);
  for (let index = 0; index < pageQueue.length;) {
    const request = pageQueue[index]!;
    if ([...request.signals].every((signal) => signal.aborted)) {
      pageQueue.splice(index, 1);
      if (pageRequests.get(request.key) === request) pageRequests.delete(request.key);
      request.cancel();
    } else if (activePages < 4) {
      pageQueue.splice(index, 1);
      request.started = true;
      activePages++;
      void request.run().finally(() => {
        // Let the browser finish dispatching cancellation before starting replacement requests.
        const release = () => {
          activePages--;
          drainPages();
        };
        if (request.controller.signal.aborted) setTimeout(release, 0);
        else release();
      });
    } else index++;
  }
}

function versionKey(documentId: string, versionId: string) {
  return `${documentId}:${versionId}`;
}

function pageKey(renderKey: string, page: number) {
  return `${renderKey}:${page}`;
}

function cachedRequest<T>(requests: Map<string, Promise<T>>, key: string, load: () => Promise<T>) {
  const existing = requests.get(key);
  if (existing) return existing;
  const request = load().finally(() => {
    if (requests.get(key) === request) requests.delete(key);
  });
  requests.set(key, request);
  return request;
}

export function peekVersionRender(documentId: string | null, versionId: string | null) {
  if (!documentId || !versionId) return undefined;
  return versions.get(versionKey(documentId, versionId));
}

export function loadVersionRender(api: Api, documentId: string, versionId: string) {
  const key = versionKey(documentId, versionId);
  const cached = versions.get(key);
  if (cached) return Promise.resolve(cached);
  return cachedRequest(versionRequests, key, async () => {
    const job = await api.renderVersion(documentId, versionId);
    const render = await api.waitJob<RenderReference>(job);
    versions.set(key, render);
    return render;
  });
}

export function peekRenderGeometry(renderKey: string) {
  return geometry.get(renderKey);
}

export function loadRenderGeometry(api: Api, renderKey: string) {
  const cached = geometry.get(renderKey);
  if (cached) return Promise.resolve(cached);
  return cachedRequest(geometryRequests, renderKey, async () => {
    const value = await api.geometry(renderKey);
    geometry.set(renderKey, value);
    return value;
  });
}

export function peekScorePlayback(renderKey: string) {
  return playback.get(renderKey);
}

export function loadScorePlayback(api: Api, renderKey: string) {
  const cached = playback.get(renderKey);
  if (cached) return Promise.resolve(cached);
  return cachedRequest(playbackRequests, renderKey, async () => {
    const value = await api.playback(renderKey);
    playback.set(renderKey, value);
    return value;
  });
}

export function cachedRenderPages(renderKey: string, pageNumbers: readonly number[]) {
  const result: Record<number, string> = {};
  for (const page of pageNumbers) {
    const value = pages.get(pageKey(renderKey, page));
    if (value !== undefined) result[page] = value;
  }
  return result;
}

/** Fetch an immutable page through the shared, visibility-prioritized four-request queue. */
export function loadRenderPage(
  api: Api,
  renderKey: string,
  page: number,
  signal: AbortSignal,
  priority: number,
) {
  const key = pageKey(renderKey, page);
  const cached = pages.get(key);
  if (cached !== undefined) return Promise.resolve(cached);
  let request = pageRequests.get(key);
  if (request?.controller.signal.aborted) request = undefined;
  if (!request) {
    let resolve!: (source: string) => void;
    let reject!: (error: unknown) => void;
    const promise = new Promise<string>((yes, no) => {
      resolve = yes;
      reject = no;
    });
    const controller = new AbortController();
    request = {
      key,
      priority,
      signals: new Set(),
      started: false,
      controller,
      promise,
      cancel: () => reject(new DOMException('Page no longer visible', 'AbortError')),
      run: async () => {
        try {
          const value = await (await api.page(renderKey, page, controller.signal)).text();
          pages.set(key, value);
          resolve(value);
        } catch (error) {
          reject(error);
        } finally {
          if (pageRequests.get(key) === request) pageRequests.delete(key);
        }
      },
    };
    pageRequests.set(key, request);
    pageQueue.push(request);
  }
  request.priority = Math.min(request.priority, priority);
  request.signals.add(signal);
  const current = request;
  const abort = () =>
    queueMicrotask(() => {
      // A priority change can resubscribe to the same page during this React commit.
      if ([...current.signals].some((subscriber) => !subscriber.aborted)) return;
      if (current.started) current.controller.abort();
      else drainPages();
    });
  signal.addEventListener('abort', abort, { once: true });
  // Queue all pages from this commit before choosing visible work ahead of neighbours.
  queueMicrotask(drainPages);
  return request.promise.finally(() => {
    signal.removeEventListener('abort', abort);
    current.signals.delete(signal);
  });
}
