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
interface AudioOutput {
  context: AudioContext;
  master: GainNode;
}
interface AudioState extends AudioOutput {
  catalog: AudioCatalog;
  base: URL;
  extension: 'ogg' | 'mp3';
}
interface Bank {
  samples: Sample[];
  gain: number;
  buffers: Map<string, AudioBuffer>;
  pending: Map<string, Promise<void>>;
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

function nearestSample(bank: Bank, pitch: number) {
  return bank.samples.reduce((best, candidate) =>
    Math.abs(candidate.pitch - pitch) < Math.abs(best.pitch - pitch) ? candidate : best,
  );
}

/** Lazily loads the sample banks required by the current score. */
export class ScoreAudio {
  private output: AudioOutput | null = null;
  private loaded: AudioState | null = null;
  private loading: Promise<AudioState> | null = null;
  private banks = new Map<SampleInstrument, Bank>();
  private sources = new Set<AudioBufferSourceNode>();

  async load(instruments: readonly SampleInstrument[] = []) {
    if (!this.loaded) {
      this.loading ??= this.loadCatalog().finally(() => {
        this.loading = null;
      });
      this.loaded = await this.loading;
    }
    const state = this.loaded;
    await Promise.all(
      [...new Set(instruments)].map((instrument) => this.loadSamples(state, instrument)),
    );
    return state;
  }

  /** Start decoding only the samples needed for these pitches. */
  async prepare(instrument: SampleInstrument, pitches: readonly number[]) {
    await this.loadSamples(await this.load(), instrument, pitches);
  }

  async resume() {
    // Request resume during user activation, even while the catalog is still loading.
    const { context } = this.audioOutput();
    if (context.state !== 'running') await context.resume();
  }
  async suspend() {
    if (this.output?.context.state === 'running') await this.output.context.suspend();
  }
  now() {
    return this.output?.context.currentTime ?? 0;
  }

  async play(
    pitches: readonly number[],
    start: number,
    duration: number,
    instrument: SampleInstrument,
    exactDuration = false,
  ) {
    const state = await this.load();
    const bank = await this.loadSamples(state, instrument, pitches);
    this.start(state, bank, instrument, pitches, start, duration, exactDuration);
  }

  /** Whether these pitches can start without waiting for decoding or resuming. */
  ready(instrument: SampleInstrument, pitches: readonly number[]) {
    const bank = this.banks.get(instrument);
    return (
      this.output?.context.state === 'running' &&
      !!bank &&
      pitches.every((pitch) => bank.buffers.has(nearestSample(bank, pitch).name))
    );
  }

  /** Start decoded pitches at once on a running context. */
  playNow(pitches: readonly number[], duration: number, instrument: SampleInstrument) {
    const bank = this.banks.get(instrument);
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
      const sample = nearestSample(bank, pitch);
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

  private audioOutput() {
    if (!this.output) {
      const context = new (contextConstructor())();
      const master = context.createGain();
      master.connect(context.destination);
      this.output = { context, master };
    }
    return this.output;
  }

  private async loadCatalog(): Promise<AudioState> {
    const base = new URL(`${import.meta.env.BASE_URL}audio/`, window.location.href);
    const url = new URL('manifest.json', base);
    const response = await fetch(url);
    if (!response.ok) throw new Error(`audio catalog ${url} failed to load`);
    const catalog: AudioCatalog = await response.json();
    const extension = document.createElement('audio').canPlayType('audio/ogg; codecs="vorbis"')
      ? 'ogg'
      : 'mp3';
    return { ...this.audioOutput(), catalog, base, extension };
  }

  private async loadSamples(
    state: AudioState,
    instrument: SampleInstrument,
    pitches?: readonly number[],
  ) {
    let bank = this.banks.get(instrument);
    if (!bank) {
      const entry = state.catalog.instruments[instrument];
      const samples = entry?.samples;
      const gain = entry?.gain ?? 1;
      if (!Number.isFinite(gain) || gain <= 0) throw new Error(`invalid ${instrument} sample gain`);
      if (!Array.isArray(samples) || !samples.length)
        throw new Error(`invalid ${instrument} sample catalog`);
      bank = { samples, gain, buffers: new Map(), pending: new Map() };
      this.banks.set(instrument, bank);
    }
    const currentBank = bank;
    const selected = pitches
      ? new Set(pitches.map((pitch) => nearestSample(currentBank, pitch)))
      : bank.samples;
    const { buffers, pending } = bank;
    const loads: Promise<void>[] = [];
    for (const sample of selected) {
      if (buffers.has(sample.name)) continue;
      let loading = pending.get(sample.name);
      if (!loading) {
        const url = new URL(sample[state.extension], state.base);
        loading = fetch(url)
          .then(async (response) => {
            if (!response.ok) throw new Error(`${instrument} sample ${url} failed to load`);
            try {
              buffers.set(
                sample.name,
                await state.context.decodeAudioData(await response.arrayBuffer()),
              );
            } catch (error) {
              throw new Error(`cannot decode ${instrument} sample ${url}: ${String(error)}`);
            }
          })
          .finally(() => pending.delete(sample.name));
        pending.set(sample.name, loading);
      }
      loads.push(loading);
    }
    await Promise.all(loads);
    return bank;
  }
}

export const scoreAudio = new ScoreAudio();
