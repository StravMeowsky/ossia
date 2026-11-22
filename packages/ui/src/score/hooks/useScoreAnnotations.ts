import { useEffect, useRef, useState } from 'react';
import type { RefObject } from 'react';

import type {
  AnnotationBox,
  GeometryBounds,
  GeometryPage,
  SelectionAnnotation,
  SelectionTarget,
} from '../../generated/protocol.ts';
import type { AnnotationEditorPlacement, EditorRect } from '../utils/annotationEditorLayout.ts';
import { firstTargetBounds } from '../utils/annotationGeometry.ts';
import { isAnnotationModifierPressed } from '../utils/annotationInput.ts';
import { selectionKey } from '../utils/api.ts';
import { createUuid } from '../utils/uuid.ts';

interface AnnotationDraft {
  awaitingRelease: boolean;
  bounds: GeometryBounds;
  boxes: AnnotationBox[];
  page: number;
  targets: SelectionTarget[];
}

interface AnnotationEditorState {
  annotation?: SelectionAnnotation;
  bounds: GeometryBounds;
  boxes: AnnotationBox[];
  page: number;
  targets: SelectionTarget[];
}

interface AnnotationSelection {
  boxes: AnnotationBox[];
  targets: SelectionTarget[];
}

/** Owns annotation mode gestures, draft groups and the inline editor anchor. */
export function useScoreAnnotations({
  annotate,
  annotations,
  onAnnotationsChange,
  onClose,
  pageHosts,
  pages,
  renderKey,
  viewport,
}: {
  annotate: boolean;
  annotations: SelectionAnnotation[];
  onAnnotationsChange?: (annotations: SelectionAnnotation[]) => void;
  onClose?: () => void;
  pageHosts: RefObject<Map<number, HTMLDivElement>>;
  pages: GeometryPage[] | undefined;
  renderKey: string;
  viewport: RefObject<HTMLDivElement | null>;
}) {
  const [draft, setDraft] = useState<AnnotationDraft | null>(null);
  const [editor, setEditor] = useState<AnnotationEditorState | null>(null);
  const [, setViewportTick] = useState(0);
  const draftRef = useRef<AnnotationDraft | null>(null);
  const editorRef = useRef<AnnotationEditorState | null>(null);
  const annotateRef = useRef(annotate);
  const closeRef = useRef(onClose);
  annotateRef.current = annotate;
  closeRef.current = onClose;
  draftRef.current = draft;
  editorRef.current = editor;

  function updateDraft(next: AnnotationDraft | null) {
    draftRef.current = next;
    setDraft(next);
  }

  function updateEditor(next: AnnotationEditorState | null) {
    editorRef.current = next;
    setEditor(next);
  }

  function dismiss() {
    updateDraft(null);
    updateEditor(null);
  }

  useEffect(() => {
    let annotationModifierWasDown = false;
    const syncModifier = (event: KeyboardEvent) => {
      const down = isAnnotationModifierPressed(event);
      if (annotationModifierWasDown && !down) {
        const current = draftRef.current;
        if (annotateRef.current && current?.awaitingRelease) {
          const next = { ...current, awaitingRelease: false };
          updateDraft(next);
          updateEditor({
            bounds: next.bounds,
            boxes: next.boxes,
            page: next.page,
            targets: next.targets,
          });
        }
      }
      annotationModifierWasDown = down;
    };
    const clear = () => {
      annotationModifierWasDown = false;
      dismiss();
    };
    window.addEventListener('keydown', syncModifier);
    window.addEventListener('keyup', syncModifier);
    window.addEventListener('blur', clear);
    return () => {
      window.removeEventListener('keydown', syncModifier);
      window.removeEventListener('keyup', syncModifier);
      window.removeEventListener('blur', clear);
    };
  }, []);

  useEffect(() => {
    if (!annotate) {
      dismiss();
    }
  }, [annotate]);

  useEffect(() => {
    dismiss();
  }, [renderKey]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (
        document.querySelector('dialog[open]') ||
        event.key !== 'Escape' ||
        event.defaultPrevented ||
        event.isComposing
      ) {
        return;
      }
      if (editorRef.current || draftRef.current) {
        event.preventDefault();
        dismiss();
      } else if (annotateRef.current) {
        event.preventDefault();
        closeRef.current?.();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
  }, []);

  useEffect(() => {
    if (editor?.annotation && !annotations.some((item) => item.id === editor.annotation?.id)) {
      dismiss();
    }
  }, [annotations, editor]);

  useEffect(() => {
    if (!editor) {
      return;
    }
    const bump = () => setViewportTick((value) => value + 1);
    window.addEventListener('resize', bump);
    return () => {
      window.removeEventListener('resize', bump);
    };
  }, [editor]);

  function locateTarget(target: SelectionTarget) {
    for (const pageLayout of pages ?? []) {
      const bounds = firstTargetBounds(pageLayout, target);
      if (bounds) {
        return { bounds, page: pageLayout.page };
      }
    }
  }

  function currentDraftSelection(): AnnotationSelection {
    const current = draftRef.current;
    if (current) {
      return { boxes: current.boxes, targets: current.targets };
    }
    const currentEditor = editorRef.current;
    return currentEditor && !currentEditor.annotation
      ? { boxes: currentEditor.boxes, targets: currentEditor.targets }
      : { boxes: [], targets: [] };
  }

  function chooseTargets(selection: AnnotationSelection, multiple: boolean) {
    const { boxes, targets } = selection;
    const unique = targets.filter(
      (target, index) =>
        targets.findIndex((candidate) => selectionKey(candidate) === selectionKey(target)) ===
        index,
    );
    if (!unique.length) {
      return;
    }
    if (multiple) {
      const current = currentDraftSelection();
      const merged = current.targets.filter(
        (target, index) =>
          current.targets.findIndex(
            (candidate) => selectionKey(candidate) === selectionKey(target),
          ) === index,
      );
      for (const target of unique) {
        if (!merged.some((candidate) => selectionKey(candidate) === selectionKey(target))) {
          merged.push(target);
        }
      }
      const anchor = locateTarget(current.targets[0] ?? unique[0]!);
      if (!anchor) {
        return;
      }
      const mergedBoxes = [...current.boxes, ...boxes];
      updateDraft({
        awaitingRelease: true,
        bounds: anchor.bounds,
        boxes: mergedBoxes,
        page: anchor.page,
        targets: merged,
      });
      const currentEditor = editorRef.current;
      if (currentEditor && !currentEditor.annotation) {
        updateEditor({
          bounds: anchor.bounds,
          boxes: mergedBoxes,
          page: anchor.page,
          targets: merged,
        });
      }
      return;
    }
    const anchor = locateTarget(unique[0]!);
    if (!anchor) {
      return;
    }
    updateDraft(null);
    updateEditor({ bounds: anchor.bounds, boxes, page: anchor.page, targets: unique });
  }

  function chooseTarget(
    page: number,
    pageLayout: GeometryPage,
    target: SelectionTarget,
    multiple: boolean,
  ) {
    const bounds = firstTargetBounds(pageLayout, target);
    if (!bounds) {
      return;
    }
    const key = selectionKey(target);
    let existing: SelectionAnnotation | undefined;
    for (let index = annotations.length - 1; index >= 0; index--) {
      const annotation = annotations[index]!;
      if (annotation.targets.some((candidate) => selectionKey(candidate) === key)) {
        existing = annotation;
        break;
      }
    }
    if (existing) {
      if (editorRef.current?.annotation?.id === existing.id) {
        return;
      }
      updateDraft(null);
      updateEditor({
        annotation: existing,
        bounds,
        boxes: existing.boxes,
        page,
        targets: existing.targets,
      });
      return;
    }
    if (multiple) {
      const current = draftRef.current;
      const currentEditor = editorRef.current;
      const selection = currentDraftSelection();
      const targets = selection.targets;
      if (targets.some((candidate) => selectionKey(candidate) === key)) {
        const remaining = targets.filter((candidate) => selectionKey(candidate) !== key);
        if (!remaining.length) {
          updateDraft(null);
          if (currentEditor && !currentEditor.annotation) {
            updateEditor(null);
          }
          return;
        }
        const anchor = locateTarget(remaining[0]!);
        if (!anchor) {
          return;
        }
        updateDraft({
          awaitingRelease: current?.awaitingRelease ?? false,
          bounds: anchor.bounds,
          boxes: selection.boxes,
          page: anchor.page,
          targets: remaining,
        });
        if (currentEditor && !currentEditor.annotation) {
          updateEditor({
            bounds: anchor.bounds,
            boxes: selection.boxes,
            page: anchor.page,
            targets: remaining,
          });
        }
        return;
      }
      chooseTargets({ boxes: [], targets: [target] }, true);
      return;
    }
    const currentEditor = editorRef.current;
    if (
      currentEditor &&
      !currentEditor.annotation &&
      currentEditor.targets.length === 1 &&
      selectionKey(currentEditor.targets[0]!) === key
    ) {
      return;
    }
    chooseTargets({ boxes: [], targets: [target] }, false);
  }

  function edit(annotation: SelectionAnnotation, bounds: GeometryBounds, page: number) {
    if (editorRef.current?.annotation?.id === annotation.id) {
      return;
    }
    updateDraft(null);
    updateEditor({
      annotation,
      bounds,
      boxes: annotation.boxes,
      page,
      targets: annotation.targets,
    });
  }

  function confirm(text: string) {
    if (!editor) {
      return;
    }
    if (editor.annotation) {
      onAnnotationsChange?.(
        annotations.map((annotation) =>
          annotation.id === editor.annotation?.id ? { ...annotation, text } : annotation,
        ),
      );
    } else {
      onAnnotationsChange?.([
        ...annotations,
        {
          id: createUuid(),
          text,
          targets: editor.targets,
          boxes: editor.boxes,
        },
      ]);
    }
    dismiss();
  }

  function remove() {
    const id = editor?.annotation?.id;
    if (id) {
      onAnnotationsChange?.(annotations.filter((annotation) => annotation.id !== id));
    }
    dismiss();
  }

  const editorPage = editor
    ? pages?.find((candidate) => candidate.page === editor.page)
    : undefined;
  const editorHost = editor ? pageHosts.current.get(editor.page) : undefined;
  const editorBox = editorPage?.viewBox;
  let editorPlacement: AnnotationEditorPlacement | undefined;
  if (editor && editorBox && editorHost) {
    const host = editorHost.getBoundingClientRect();
    const anchor = {
      x:
        host.left +
        ((editor.bounds.x0 + editor.bounds.x1 - editorBox.x0 * 2) /
          Math.max(1e-9, editorBox.x1 - editorBox.x0)) *
          host.width *
          0.5,
      y:
        host.top +
        ((editor.bounds.y0 - editorBox.y0) / Math.max(1e-9, editorBox.y1 - editorBox.y0)) *
          host.height,
    };
    const viewportBox = viewport.current?.getBoundingClientRect();
    const viewportRect = {
      x0: viewportBox?.left ?? 0,
      x1: viewportBox?.right ?? window.innerWidth,
      y0: viewportBox?.top ?? 0,
      y1: viewportBox?.bottom ?? window.innerHeight,
    };
    const obstacles: EditorRect[] = [];
    for (const pageLayout of pages ?? []) {
      const targetHost = pageHosts.current.get(pageLayout.page);
      const targetBox = pageLayout.viewBox;
      if (!targetHost || !targetBox) {
        continue;
      }
      const targetHostBox = targetHost.getBoundingClientRect();
      const scaleX = targetHostBox.width / Math.max(1e-9, targetBox.x1 - targetBox.x0);
      const scaleY = targetHostBox.height / Math.max(1e-9, targetBox.y1 - targetBox.y0);
      const toViewport = (bounds: GeometryBounds): EditorRect => ({
        x0: targetHostBox.left + (bounds.x0 - targetBox.x0) * scaleX,
        x1: targetHostBox.left + (bounds.x1 - targetBox.x0) * scaleX,
        y0: targetHostBox.top + (bounds.y0 - targetBox.y0) * scaleY,
        y1: targetHostBox.top + (bounds.y1 - targetBox.y0) * scaleY,
      });
      for (const target of editor.targets) {
        const key = selectionKey(target);
        const occurrences =
          target.kind === 'measure'
            ? pageLayout.regions
                .filter(
                  (region) => selectionKey({ kind: 'measure', locator: region.locator }) === key,
                )
                .map((region) => region.bounds)
            : pageLayout.elements
                .filter((element) => selectionKey(element.region.target) === key)
                .flatMap((element) =>
                  element.hitBounds.length ? element.hitBounds : [element.bounds],
                );
        for (const bounds of occurrences) {
          const rect = toViewport(bounds);
          if (
            rect.x1 >= viewportRect.x0 &&
            rect.x0 <= viewportRect.x1 &&
            rect.y1 >= viewportRect.y0 &&
            rect.y0 <= viewportRect.y1
          ) {
            obstacles.push(rect);
          }
        }
      }
    }
    const selection =
      obstacles.length > 0
        ? {
            x0: Math.min(...obstacles.map((bounds) => bounds.x0)),
            x1: Math.max(...obstacles.map((bounds) => bounds.x1)),
            y0: Math.min(...obstacles.map((bounds) => bounds.y0)),
            y1: Math.max(...obstacles.map((bounds) => bounds.y1)),
          }
        : { x0: anchor.x - 1, x1: anchor.x + 1, y0: anchor.y - 1, y1: anchor.y + 1 };
    editorPlacement = { anchor, obstacles, selection, viewport: viewportRect };
  }

  return {
    chooseTarget,
    chooseTargets,
    confirm,
    dismiss,
    draft,
    edit,
    editor,
    editorPlacement,
    remove,
  };
}
