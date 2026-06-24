<script setup lang="ts">
import { injectRulesStore } from "@/composables/useRulesDocument";
import { computed, ref } from "vue";
import { isMap, isScalar, isSeq } from "yaml";

const store = injectRulesStore();

function mapKeys(path: Array<string | number>): string[] {
  void store.docVersion.value;
  const node = store.doc.value.getIn(path, true);
  if (!isMap(node)) return [];
  return node.items.map((item) => (isScalar(item.key) ? String(item.key.value) : String(item.key)));
}
function listEntries(name: string): string[] {
  void store.docVersion.value;
  const seq = store.doc.value.getIn(["refs", "lists", name], true);
  if (!isSeq(seq)) return [];
  return seq.items.map((item) => (isScalar(item) ? String(item.value) : ""));
}
function scalarText(path: Array<string | number>): string {
  void store.docVersion.value;
  return String(store.doc.value.getIn(path) ?? "");
}

const listNames = computed(() => mapKeys(["refs", "lists"]));
const patternNames = computed(() => mapKeys(["refs", "patterns"]));
const thresholdNames = computed(() => mapKeys(["refs", "thresholds"]));

function targetValue(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

function setEntry(list: string, index: number, event: Event): void {
  store.commitGuiEdit((doc) => doc.setIn(["refs", "lists", list, index], targetValue(event)));
}
function addEntry(list: string): void {
  store.commitGuiEdit((doc) => {
    const seq = doc.getIn(["refs", "lists", list], true);
    if (isSeq(seq)) seq.add("");
  });
}
function removeEntry(list: string, index: number): void {
  store.commitGuiEdit((doc) => doc.deleteIn(["refs", "lists", list, index]));
}
function removeList(list: string): void {
  store.commitGuiEdit((doc) => doc.deleteIn(["refs", "lists", list]));
}
function setPattern(name: string, event: Event): void {
  store.commitGuiEdit((doc) => doc.setIn(["refs", "patterns", name], targetValue(event)));
}
function removePattern(name: string): void {
  store.commitGuiEdit((doc) => doc.deleteIn(["refs", "patterns", name]));
}
function setThreshold(name: string, event: Event): void {
  store.commitGuiEdit((doc) => doc.setIn(["refs", "thresholds", name], targetValue(event)));
}
function removeThreshold(name: string): void {
  store.commitGuiEdit((doc) => doc.deleteIn(["refs", "thresholds", name]));
}

const newList = ref("");
const newPattern = ref("");
const newThreshold = ref("");
function addList(): void {
  const name = newList.value.trim();
  if (name === "") return;
  store.commitGuiEdit((doc) => doc.setIn(["refs", "lists", name], doc.createNode([])));
  newList.value = "";
}
function addPattern(): void {
  const name = newPattern.value.trim();
  if (name === "") return;
  store.commitGuiEdit((doc) => doc.setIn(["refs", "patterns", name], ""));
  newPattern.value = "";
}
function addThreshold(): void {
  const name = newThreshold.value.trim();
  if (name === "") return;
  store.commitGuiEdit((doc) => doc.setIn(["refs", "thresholds", name], "7d"));
  newThreshold.value = "";
}

const entryClass =
  "h-8 flex-1 rounded-md border border-input bg-transparent px-2 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-ring";
const addClass =
  "h-8 rounded-md border border-input bg-transparent px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring";
</script>

<template lang="hsml">
section(class="space-y-4")
  h3(class="text-sm font-semibold uppercase tracking-wide text-muted-foreground") Refs
  div(class="space-y-3")
    div(class="text-xs font-medium text-muted-foreground") Lists (sender domains/addresses)
    div(v-for="name in listNames" :key="name" class="space-y-1.5 rounded-md border border-border p-2")
      div(class="flex items-center justify-between gap-2")
        span(class="text-sm font-medium") {{ name }}
        button(type="button" class="text-xs text-muted-foreground hover:text-destructive" @click="removeList(name)") remove list
      div(v-for="(entry, i) in listEntries(name)" :key="i" class="flex items-center gap-2")
        input(:value="entry" :class="entryClass" @change="(e) => setEntry(name, i, e)")
        button(type="button" class="text-xs text-muted-foreground hover:text-destructive" @click="removeEntry(name, i)") ×
      button(type="button" class="text-xs font-medium text-primary hover:underline" @click="addEntry(name)") + entry
    div(class="flex items-center gap-2")
      input(v-model="newList" :class="addClass" placeholder="new list name")
      button(type="button" class="text-xs font-medium text-primary hover:underline" @click="addList") + list
  div(class="space-y-2")
    div(class="text-xs font-medium text-muted-foreground") Patterns (regex)
    div(v-for="name in patternNames" :key="name" class="flex items-center gap-2")
      span(class="w-40 shrink-0 truncate text-sm font-medium") {{ name }}
      input(:value="scalarText(['refs', 'patterns', name])" :class="entryClass" @change="(e) => setPattern(name, e)")
      button(type="button" class="text-xs text-muted-foreground hover:text-destructive" @click="removePattern(name)") ×
    div(class="flex items-center gap-2")
      input(v-model="newPattern" :class="addClass" placeholder="new pattern name")
      button(type="button" class="text-xs font-medium text-primary hover:underline" @click="addPattern") + pattern
  div(class="space-y-2")
    div(class="text-xs font-medium text-muted-foreground") Thresholds (durations)
    div(v-for="name in thresholdNames" :key="name" class="flex items-center gap-2")
      span(class="w-40 shrink-0 truncate text-sm font-medium") {{ name }}
      input(:value="scalarText(['refs', 'thresholds', name])" class="h-8 w-28 rounded-md border border-input bg-transparent px-2 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-ring" @change="(e) => setThreshold(name, e)")
      button(type="button" class="text-xs text-muted-foreground hover:text-destructive" @click="removeThreshold(name)") ×
    div(class="flex items-center gap-2")
      input(v-model="newThreshold" :class="addClass" placeholder="new threshold name")
      button(type="button" class="text-xs font-medium text-primary hover:underline" @click="addThreshold") + threshold
</template>
