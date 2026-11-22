window.__ModuleLoader__.load({
  id: 'ossia-dsh-plugin',
  factory(require) {
    const React = require('react');
    const CHANNEL = '/ossia-selection';
    const POLL_MS = 500;
    const SEPARATOR = '\n\n';
    const MESSAGES = {
      en: {
        selectionOne: '/ossia 1 selection selected.',
        selectionMany: '/ossia {count} selections selected.',
        annotationOne: '/ossia Added 1 annotation.',
        annotationMany: '/ossia Added {count} annotations.',
        annotationTitleOne: '/ossia 「{title}」 Added 1 annotation.',
        annotationTitleMany: '/ossia 「{title}」 Added {count} annotations.',
        combined: '/ossia Selected {selectionCount} objects, added {annotationCount} annotations.',
      },
      zh: {
        selectionOne: '/ossia 已选择 {count} 个对象。',
        selectionMany: '/ossia 已选择 {count} 个对象。',
        annotationOne: '/ossia 已添加 {count} 个标注。',
        annotationMany: '/ossia 已添加 {count} 个标注。',
        annotationTitleOne: '/ossia 「{title}」已添加 {count} 个标注。',
        annotationTitleMany: '/ossia 「{title}」已添加 {count} 个标注。',
        combined: '/ossia 已选择 {selectionCount} 个对象，添加 {annotationCount} 个标注。',
      },
      ja: {
        selectionOne: '/ossia {count} 件の選択範囲。',
        selectionMany: '/ossia {count} 件の選択範囲。',
        annotationOne: '/ossia {count} 件の注釈を追加しました。',
        annotationMany: '/ossia {count} 件の注釈を追加しました。',
        annotationTitleOne: '/ossia 「{title}」に注釈を{count}件追加しました。',
        annotationTitleMany: '/ossia 「{title}」に注釈を{count}件追加しました。',
        combined: '/ossia {selectionCount} 件を選択し、{annotationCount} 件の注釈を追加しました。',
      },
      es: {
        selectionOne: '/ossia {count} selección.',
        selectionMany: '/ossia {count} selecciones.',
        annotationOne: '/ossia Se añadió {count} anotación.',
        annotationMany: '/ossia Se añadieron {count} anotaciones.',
        annotationTitleOne: '/ossia 「{title}」: se añadió {count} anotación.',
        annotationTitleMany: '/ossia 「{title}」: se añadieron {count} anotaciones.',
        combined:
          '/ossia Se seleccionaron {selectionCount} objetos y se añadieron {annotationCount} anotaciones.',
      },
      fr: {
        selectionOne: '/ossia {count} sélection.',
        selectionMany: '/ossia {count} sélections.',
        annotationOne: '/ossia {count} annotation ajoutée.',
        annotationMany: '/ossia {count} annotations ajoutées.',
        annotationTitleOne: '/ossia 「{title}」 : {count} annotation ajoutée.',
        annotationTitleMany: '/ossia 「{title}」 : {count} annotations ajoutées.',
        combined:
          '/ossia {selectionCount} objets sélectionnés, {annotationCount} annotations ajoutées.',
      },
      de: {
        selectionOne: '/ossia {count} Auswahl.',
        selectionMany: '/ossia {count} Auswahlen.',
        annotationOne: '/ossia {count} Anmerkung hinzugefügt.',
        annotationMany: '/ossia {count} Anmerkungen hinzugefügt.',
        annotationTitleOne: '/ossia 「{title}」: {count} Anmerkung hinzugefügt.',
        annotationTitleMany: '/ossia 「{title}」: {count} Anmerkungen hinzugefügt.',
        combined:
          '/ossia {selectionCount} Objekte ausgewählt, {annotationCount} Anmerkungen hinzugefügt.',
      },
      ru: {
        selectionOne: '/ossia Выбран {count} фрагмент.',
        selectionMany: '/ossia Выбрано фрагментов: {count}.',
        annotationOne: '/ossia Добавлена {count} аннотация.',
        annotationMany: '/ossia Добавлено аннотаций: {count}.',
        annotationTitleOne: '/ossia 「{title}」: добавлена {count} аннотация.',
        annotationTitleMany: '/ossia 「{title}」: добавлено аннотаций: {count}.',
        combined:
          '/ossia Выбрано объектов: {selectionCount}, добавлено аннотаций: {annotationCount}.',
      },
      pt: {
        selectionOne: '/ossia {count} seleção.',
        selectionMany: '/ossia {count} seleções.',
        annotationOne: '/ossia {count} anotação adicionada.',
        annotationMany: '/ossia {count} anotações adicionadas.',
        annotationTitleOne: '/ossia 「{title}」: {count} anotação adicionada.',
        annotationTitleMany: '/ossia 「{title}」: {count} anotações adicionadas.',
        combined:
          '/ossia {selectionCount} objetos selecionados, {annotationCount} anotações adicionadas.',
      },
    };
    const LEGACY_HINTS = ['/ossia 已选择 {count} 个选区。'];
    const HINT = new RegExp(
      `^(?:${Object.values(MESSAGES)
        .flatMap((messages) => Object.values(messages))
        .concat(LEGACY_HINTS)
        .map(templatePattern)
        .join('|')})`,
    );
    const applied = { epoch: null, revision: -1 };
    let warned = false;

    function templatePattern(template) {
      return template
        .split(/(\{\w+\})/u)
        .map((part) => {
          if (part === '{title}') return '[^\\r\\n]+?';
          if (/^\{\w+\}$/u.test(part)) return '\\d+';
          return part.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        })
        .join('');
    }

    function format(template, params) {
      return template.replace(/\{(\w+)\}/g, (match, name) =>
        name in params ? String(params[name]) : match,
      );
    }

    function translate(locale, key, params = {}) {
      const messages = MESSAGES[locale] ?? MESSAGES.en;
      return format(messages[key] ?? MESSAGES.en[key] ?? key, params);
    }

    function englishT(key, params = {}) {
      return translate('en', key, params);
    }

    function hintFor(annotationCount = 0, documentTitle = null, t = englishT) {
      if (annotationCount <= 0) return null;
      const title =
        typeof documentTitle === 'string' ? documentTitle.replace(/\s+/gu, ' ').trim() : '';
      if (title.length > 0) {
        return t(annotationCount === 1 ? 'annotationTitleOne' : 'annotationTitleMany', {
          count: annotationCount,
          title,
        });
      }
      return t(annotationCount === 1 ? 'annotationOne' : 'annotationMany', {
        count: annotationCount,
      });
    }

    function reconcile(draft, annotationCount = 0, documentTitle = null, t = englishT) {
      const match = HINT.exec(draft);
      const hint = hintFor(annotationCount, documentTitle, t);
      if (hint === null) {
        if (match === null) return null;
        let end = match[0].length;
        while (draft[end] === '\n' || draft[end] === '\r') end += 1;
        return { start: 0, end, text: '' };
      }
      if (match !== null) {
        if (match[0] === hint) return null;
        return { start: 0, end: match[0].length, text: hint };
      }
      if (draft.length === 0) return { start: 0, end: 0, text: hint };
      return { start: 0, end: 0, text: `${hint}${SEPARATOR}` };
    }

    function warnMissing() {
      if (warned) return;
      warned = true;
      console.warn(
        '[ossia-dsh-plugin] composer inputActions.insertText is unavailable; annotation hints are disabled',
      );
    }

    function OssiaSelectionHintHost(props) {
      const revision = React.useSyncExternalStore(
        (listener) => props.locale.subscribe(listener),
        () => props.locale.getSnapshot().revision,
      );
      return React.createElement(OssiaSelectionHint, { ...props, localeRevision: revision });
    }

    function OssiaSelectionHint(props) {
      const draft = props.useInput((state) => state.draft);
      const draftRev = props.useInput((state) => state.draftRev);
      const live = React.useRef({ draft, draftRev });
      live.current = { draft, draftRev };
      const mountedAt = React.useRef(Date.now());
      const lastCounts = React.useRef(null);

      React.useEffect(() => {
        if (
          props.inputActions === undefined ||
          typeof props.inputActions.insertText !== 'function'
        ) {
          warnMissing();
          return;
        }
        mountedAt.current = Date.now();
        let stopped = false;
        let busy = false;
        const tick = async () => {
          if (stopped || busy) return;
          busy = true;
          try {
            const result = await props.rpc.call(CHANNEL, 'latest', {});
            if (!result?.ok || !result.value) return;
            const latest = result.value;
            if (!Number.isInteger(latest.annotationCount) || latest.annotationCount < 0) {
              return;
            }
            if (latest.documentTitle != null && typeof latest.documentTitle !== 'string') {
              return;
            }
            const seen = applied.epoch === latest.epoch ? applied.revision : -1;
            if (latest.revision <= seen) return;
            const current = live.current;
            const change = reconcile(
              current.draft,
              latest.annotationCount,
              latest.documentTitle,
              props.t,
            );
            const recent = latest.at >= mountedAt.current - 1000;
            if (!recent && applied.epoch !== latest.epoch) {
              applied.epoch = latest.epoch;
              applied.revision = latest.revision;
              return;
            }
            lastCounts.current = {
              annotationCount: latest.annotationCount,
              documentTitle: latest.documentTitle,
            };
            if (change === null) {
              applied.epoch = latest.epoch;
              applied.revision = latest.revision;
              return;
            }
            if (
              props.inputActions.insertText(change.text, {
                start: change.start,
                end: change.end,
                draftRev: current.draftRev,
              })
            ) {
              applied.epoch = latest.epoch;
              applied.revision = latest.revision;
            }
          } catch {
            // DSH connection may not be ready yet; retry on the next tick.
          } finally {
            busy = false;
          }
        };
        void tick();
        const timer = setInterval(tick, POLL_MS);
        return () => {
          stopped = true;
          clearInterval(timer);
        };
      }, [props.inputActions, props.rpc, props.session?.sessionId, props.t]);

      React.useEffect(() => {
        const counts = lastCounts.current;
        if (counts === null) return;
        if (counts.annotationCount > 0 && !HINT.test(live.current.draft)) {
          return;
        }
        const change = reconcile(
          live.current.draft,
          counts.annotationCount,
          counts.documentTitle,
          props.t,
        );
        if (change === null || props.inputActions === undefined) return;
        props.inputActions.insertText(change.text, {
          start: change.start,
          end: change.end,
          draftRev: live.current.draftRev,
        });
      }, [props.localeRevision, props.inputActions, props.t]);

      return null;
    }

    function apply(ctx) {
      const rpc = ctx.connection.rpc;
      const t = ctx.locale.bind('ossia');
      ctx.effect(() => ctx.locale.register('ossia', MESSAGES), 'ossia-selection: locale');
      ctx.slots.inject('conversation.input.dock', () =>
        ctx.slots.register(
          { name: 'conversation.input.dock', id: 'ossia-selection-hint', order: 1000 },
          (props) =>
            React.createElement(OssiaSelectionHintHost, { ...props, rpc, t, locale: ctx.locale }),
        ),
      );
    }

    return {
      inject: ['slots', 'connection', 'locale'],
      apply,
      __test: { hintFor, reconcile, translate },
    };
  },
});
