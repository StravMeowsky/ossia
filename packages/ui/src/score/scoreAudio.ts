import type { SampleInstrument } from '../generated/protocol.ts';

interface Sample {
  name: string;
  pitch: number;
  ogg: string;
  mp3: string;
}
interface AudioCatalog {
  instruments: Record<string, { samples: Sample[]; gain?: number }>;
}
interface AudioState {
  context: AudioContext;
  master: GainNode;
  catalog: AudioCatalog;
  base: URL;
}
interface Bank {
  samples: Sample[];
  gain: number;
  buffers: Map<string, AudioBuffer>;
}

const MASTER_GAIN = 0.82;
const RELEASE_SECONDS = 0.35;

function contextConstructor() {
  const ctor =
    window.AudioContext ??
    (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!ctor) throw new Error('Web Audio is not supported in this browser');
  return ctor;
}

/** Lazily loads the sample banks required by the current score. */
export class ScoreAudio {
  private loaded: AudioState | null = null;
  private loading: Promise<AudioState> | null = null;
  private banks = new Map<SampleInstrument, Promise<Bank>>();
  private decoded = new Map<SampleInstrument, Bank>();
  private sources = new Set<AudioBufferSourceNode>();

  async load(instruments: readonly SampleInstrument[] = []) {
    if (!this.loaded) {
      this.loading ??= this.loadCatalog().finally(() => {
        this.loading = null;
      });
      this.loaded = await this.loading;
    }
    const state = this.loaded;
    await Promise.all([...new Set(instruments)].map((instrument) => this.bank(state, instrument)));
    return state;
  }

  async resume() {
    const { context } = await this.load();
    if (context.state !== 'running') await context.resume();
  }
  async suspend() {
    if (this.loaded?.context.state === 'running') await this.loaded.context.suspend();
  }
  now() {
    return this.loaded?.context.currentTime ?? 0;
  }

  async play(
    pitches: readonly number[],
    start: number,
    duration: number,
    instrument: SampleInstrument,
    exactDuration = false,
  ) {
    const state = await this.load([instrument]);
    const bank = await this.bank(state, instrument);
    this.start(state, bank, instrument, pitches, start, duration, exactDuration);
  }

  /** Whether `playNow` can sound these instruments without waiting for loading or resuming. */
  ready(instruments: readonly SampleInstrument[]) {
    return (
      this.loaded?.context.state === 'running' &&
      instruments.every((instrument) => this.decoded.has(instrument))
    );
  }

  /** Start pitches at once on a running context with a decoded bank; check `ready` first. */
  playNow(pitches: readonly number[], duration: number, instrument: SampleInstrument) {
    const bank = this.decoded.get(instrument);
    if (!this.loaded || !bank) throw new Error(`${instrument} samples are not loaded`);
    this.start(
      this.loaded,
      bank,
      instrument,
      pitches,
      this.loaded.context.currentTime,
      duration,
      false,
    );
  }

  private start(
    state: AudioState,
    bank: Bank,
    instrument: SampleInstrument,
    pitches: readonly number[],
    start: number,
    duration: number,
    exactDuration: boolean,
  ) {
    const when = Math.max(start, state.context.currentTime);
    const length = exactDuration ? duration : Math.max(0.05, duration);
    for (const pitch of pitches) {
      const sample = bank.samples.reduce((best, candidate) =>
        Math.abs(candidate.pitch - pitch) < Math.abs(best.pitch - pitch) ? candidate : best,
      );
      const buffer = bank.buffers.get(sample.name);
      if (!buffer) throw new Error(`missing ${instrument} sample ${sample.name}`);
      const source = state.context.createBufferSource();
      const gain = state.context.createGain();
      source.buffer = buffer;
      source.playbackRate.value = 2 ** ((pitch - sample.pitch) / 12);
      gain.gain.setValueAtTime(MASTER_GAIN * bank.gain, when);
      gain.gain.setValueAtTime(MASTER_GAIN * bank.gain, when + length);
      gain.gain.exponentialRampToValueAtTime(0.001 * bank.gain, when + length + RELEASE_SECONDS);
      source.connect(gain);
      gain.connect(state.master);
      source.start(when);
      source.stop(when + length + RELEASE_SECONDS + 0.02);
      this.sources.add(source);
      source.onended = () => {
        this.sources.delete(source);
        source.disconnect();
        gain.disconnect();
      };
    }
  }

  stopAll() {
    for (const source of this.sources) {
      try {
        source.stop();
      } catch {
        /* Already ended. */
      }
      source.disconnect();
    }
    this.sources.clear();
  }

  private async loadCatalog(): Promise<AudioState> {
    const context = new (contextConstructor())();
    const master = context.createGain();
    master.connect(context.destination);
    const base = new URL(`${import.meta.env.BASE_URL}audio/`, window.location.href);
    try {
      const url = new URL('manifest.json', base);
      const response = await fetch(url);
      if (!response.ok) throw new Error(`audio catalog ${url} failed to load`);
      const catalog: AudioCatalog = await response.json();
      return { context, master, catalog, base };
    } catch (error) {
      await context.close().catch(() => {});
      throw error;
    }
  }

  private bank(state: AudioState, instrument: SampleInstrument) {
    let promise = this.banks.get(instrument);
    if (!promise) {
      promise = (async () => {
        const entry = state.catalog.instruments[instrument];
        const samples = entry?.samples;
        const gain = entry?.gain ?? 1;
        if (!Number.isFinite(gain) || gain <= 0)
          throw new Error(`invalid ${instrument} sample gain`);
        if (!Array.isArray(samples) || !samples.length)
          throw new Error(`invalid ${instrument} sample catalog`);
        const audio = document.createElement('audio');
        const extension = audio.canPlayType('audio/ogg; codecs="vorbis"') ? 'ogg' : 'mp3';
        const buffers = new Map<string, AudioBuffer>();
        await Promise.all(
          samples.map(async (sample) => {
            const url = new URL(sample[extension], state.base);
            const response = await fetch(url);
            if (!response.ok) throw new Error(`${instrument} sample ${url} failed to load`);
            try {
              buffers.set(
                sample.name,
                await state.context.decodeAudioData(await response.arrayBuffer()),
              );
            } catch (error) {
              throw new Error(`cannot decode ${instrument} sample ${url}: ${String(error)}`);
            }
          }),
        );
        const bank = { samples, buffers, gain };
        this.decoded.set(instrument, bank);
        return bank;
      })().catch((error) => {
        this.banks.delete(instrument);
        throw error;
      });
      this.banks.set(instrument, promise);
    }
    return promise;
  }
}

export const scoreAudio = new ScoreAudio();
