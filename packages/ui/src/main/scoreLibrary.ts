import type { ManagementScoreSummary } from '../generated/protocol.ts';

export type ScoreLayout = 'grid' | 'list';
export type ScoreSortMetric = 'created' | 'updated';
export type ScoreSortDirection = 'asc' | 'desc';

export interface ScoreSort {
  metric: ScoreSortMetric;
  direction: ScoreSortDirection;
}

export type FriendlyScoreTime =
  | { kind: 'today'; time: string }
  | { kind: 'yesterday' }
  | { kind: 'day-before-yesterday' }
  | { kind: 'this-week' }
  | { kind: 'date'; date: string };

export const SCORE_LAYOUT_KEY = 'ossia:management:scores-layout';
export const SCORE_SORT_KEY = 'ossia:management:scores-sort';
export const DEFAULT_SCORE_LAYOUT: ScoreLayout = 'grid';
export const DEFAULT_SCORE_SORT: ScoreSort = { metric: 'updated', direction: 'desc' };

export function parseScoreLayout(value: string | null): ScoreLayout {
  return value === 'grid' || value === 'list' ? value : DEFAULT_SCORE_LAYOUT;
}

export function parseScoreSort(value: string | null): ScoreSort {
  if (value === 'created-asc' || value === 'created-desc') {
    return { metric: 'created', direction: value.endsWith('asc') ? 'asc' : 'desc' };
  }
  if (value === 'updated-asc' || value === 'updated-desc') {
    return { metric: 'updated', direction: value.endsWith('asc') ? 'asc' : 'desc' };
  }
  return DEFAULT_SCORE_SORT;
}

export function serializeScoreSort(sort: ScoreSort) {
  return `${sort.metric}-${sort.direction}`;
}

function normalized(value: string) {
  return value.normalize('NFKC').toLocaleLowerCase();
}

/** Library catalog filters; `null` shows every score. */
export interface ScoreCatalogFilter {
  composer: string | null;
  instrument: string | null;
  period: string | null;
}

export const NO_CATALOG_FILTER: ScoreCatalogFilter = {
  composer: null,
  instrument: null,
  period: null,
};

/** Distinct composers, instruments and periods present in the library, in display order. */
export function scoreCatalogOptions(scores: ManagementScoreSummary[], locale: string) {
  const collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
  const distinct = (values: (string | null | undefined)[]) =>
    [...new Set(values.filter((value): value is string => !!value))].sort(collator.compare);
  return {
    composers: distinct(scores.map((score) => score.composer)),
    instruments: distinct(scores.flatMap((score) => score.instruments ?? [])),
    periods: distinct(scores.map((score) => score.period)).sort(
      (left, right) => periodRank(left) - periodRank(right),
    ),
  };
}

const PERIODS = [
  'Medieval',
  'Renaissance',
  'Baroque',
  'Classical',
  'Romantic',
  'Modern',
  'Contemporary',
];

function periodRank(period: string) {
  const index = PERIODS.findIndex((known) => known.toLowerCase() === period.toLowerCase());
  return index < 0 ? PERIODS.length : index;
}

const ZH_LABELS: Record<string, string> = {
  medieval: '中世纪',
  renaissance: '文艺复兴',
  baroque: '巴洛克',
  classical: '古典',
  romantic: '浪漫',
  modern: '现代',
  contemporary: '当代',
  piano: '钢琴',
  violin: '小提琴',
  viola: '中提琴',
  cello: '大提琴',
  'double bass': '低音提琴',
  guitar: '吉他',
  flute: '长笛',
  oboe: '双簧管',
  clarinet: '单簧管',
  bassoon: '巴松管',
  harpsichord: '羽管键琴',
  organ: '管风琴',
  voice: '人声',
  ukulele: '尤克里里',
};

/** Show a stored period or instrument name in the interface language. */
export function catalogLabel(value: string, language: string) {
  return language.startsWith('zh') ? (ZH_LABELS[value.toLowerCase()] ?? value) : value;
}

function matchesCatalog(score: ManagementScoreSummary, filter: ScoreCatalogFilter) {
  return (
    (!filter.composer || score.composer === filter.composer) &&
    (!filter.instrument || (score.instruments ?? []).includes(filter.instrument)) &&
    (!filter.period || score.period === filter.period)
  );
}

export function filterAndSortScores(
  scores: ManagementScoreSummary[],
  query: string,
  sort: ScoreSort,
  locale: string,
  catalog: ScoreCatalogFilter = NO_CATALOG_FILTER,
) {
  const needle = normalized(query.trim());
  const filtered = scores.filter(
    (score) =>
      matchesCatalog(score, catalog) &&
      (!needle ||
        normalized(score.title).includes(needle) ||
        normalized(score.composer ?? '').includes(needle)),
  );
  const collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
  const multiplier = sort.direction === 'asc' ? 1 : -1;
  return filtered.sort((left, right) => {
    const leftTime = Date.parse(sort.metric === 'created' ? left.createdAt : left.updatedAt);
    const rightTime = Date.parse(sort.metric === 'created' ? right.createdAt : right.updatedAt);
    if (leftTime !== rightTime) return (leftTime - rightTime) * multiplier;
    return (
      collator.compare(left.title, right.title) || left.documentId.localeCompare(right.documentId)
    );
  });
}

function localDayNumber(date: Date) {
  return Math.floor(
    Date.UTC(date.getFullYear(), date.getMonth(), date.getDate()) / (24 * 60 * 60 * 1000),
  );
}

export function friendlyScoreTime(value: string, now: Date, locale: string): FriendlyScoreTime {
  const date = new Date(value);
  const day = localDayNumber(date);
  const today = localDayNumber(now);
  const difference = today - day;
  if (difference === 0) {
    return {
      kind: 'today',
      time: new Intl.DateTimeFormat(locale, {
        hour: '2-digit',
        minute: '2-digit',
        hourCycle: 'h23',
      }).format(date),
    };
  }
  if (difference === 1) return { kind: 'yesterday' };
  if (difference === 2) return { kind: 'day-before-yesterday' };
  const mondayOffset = (now.getDay() + 6) % 7;
  const startOfWeek = today - mondayOffset;
  if (difference > 2 && day >= startOfWeek) return { kind: 'this-week' };
  return {
    kind: 'date',
    date: new Intl.DateTimeFormat(locale, { month: 'short', day: 'numeric' }).format(date),
  };
}
