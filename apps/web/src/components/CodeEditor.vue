<script setup lang="ts">
import { yaml } from "@codemirror/lang-yaml";
import { EditorView, basicSetup } from "codemirror";
import { onBeforeUnmount, onMounted, ref, watch } from "vue";

const props = defineProps<{ value: string }>();
const emit = defineEmits<{ "update:value": [string]; focus: [boolean] }>();

const host = ref<HTMLElement | null>(null);
let view: EditorView | null = null;
// Set while we push a prop-driven change into the editor, so the update listener can tell our own
// programmatic edit from a real user keystroke and not echo it straight back (the feedback loop).
let applyingExternal = false;

// Theme reads the app's shadcn CSS vars, so one definition serves both light and dark (.dark).
const theme = EditorView.theme({
  "&": { backgroundColor: "transparent", color: "var(--foreground)", fontSize: "0.75rem" },
  ".cm-content": {
    fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    padding: "0.75rem 0",
  },
  ".cm-gutters": {
    backgroundColor: "transparent",
    color: "var(--muted-foreground)",
    border: "none",
  },
  ".cm-activeLine": { backgroundColor: "color-mix(in oklab, var(--accent) 50%, transparent)" },
  ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--foreground)" },
  ".cm-cursor": { borderLeftColor: "var(--foreground)" },
  "&.cm-focused": { outline: "none" },
});

onMounted(() => {
  const parent = host.value;
  if (parent === null) return;
  view = new EditorView({
    parent,
    doc: props.value,
    extensions: [
      basicSetup,
      yaml(),
      theme,
      EditorView.lineWrapping,
      EditorView.updateListener.of((update) => {
        if (update.focusChanged) emit("focus", view?.hasFocus ?? false);
        if (!update.docChanged || applyingExternal) return;
        emit("update:value", update.state.doc.toString());
      }),
    ],
  });
});

// Push external (prop) changes in only when they actually differ, so a value we just emitted does
// not reset the document and jump the cursor.
watch(
  () => props.value,
  (next) => {
    if (view === null) return;
    const current = view.state.doc.toString();
    if (next === current) return;
    applyingExternal = true;
    view.dispatch({ changes: { from: 0, to: current.length, insert: next } });
    applyingExternal = false;
  },
);

onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});
</script>

<template lang="hsml">
div(ref="host" class="max-h-[32rem] min-h-96 overflow-auto rounded-md border border-input bg-transparent text-foreground shadow-inner focus-within:ring-1 focus-within:ring-ring")
</template>
