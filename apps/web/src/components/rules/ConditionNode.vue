<script setup lang="ts">
import { injectRulesStore } from "@/composables/useRulesDocument";
import { CONDITION_FIELDS, CONDITION_OPS, conditionKindOf, makeLeafNode } from "@/lib/rulesAst";
import { cn } from "@/lib/utils";
import { computed } from "vue";
import { isSeq } from "yaml";

const props = defineProps<{ path: Array<string | number> }>();

const store = injectRulesStore();

function valueAt(segment: string): unknown {
  void store.docVersion.value;
  return store.doc.value.getIn([...props.path, segment]);
}

const node = computed(() => {
  void store.docVersion.value;
  return store.doc.value.getIn(props.path, true);
});
const kind = computed(() => conditionKindOf(node.value));

const childPaths = computed<Array<Array<string | number>>>(() => {
  void store.docVersion.value;
  const k = kind.value;
  if (k === "all" || k === "any") {
    const seq = store.doc.value.getIn([...props.path, k], true);
    return isSeq(seq) ? seq.items.map((_unused, i) => props.path.concat(k, i)) : [];
  }
  if (k === "not") return [props.path.concat("not")];
  return [];
});

const leafField = computed(() => String(valueAt("field") ?? ""));
const leafOp = computed(() => String(valueAt("op") ?? ""));
const usesRef = computed(() => valueAt("ref") !== undefined);
const refValue = computed(() => String(valueAt("ref") ?? ""));
const rawValue = computed(() => valueAt("value"));
const valueIsList = computed(() => Array.isArray(rawValue.value));
const scalarValue = computed(() => (valueIsList.value ? "" : String(rawValue.value ?? "")));

function setKey(segment: string, value: string): void {
  store.commitGuiEdit((doc) => doc.setIn([...props.path, segment], value));
}
function switchToRef(): void {
  store.commitGuiEdit((doc) => {
    doc.deleteIn([...props.path, "value"]);
    doc.setIn([...props.path, "ref"], refValue.value === "" ? "lists." : refValue.value);
  });
}
function switchToValue(): void {
  store.commitGuiEdit((doc) => {
    doc.deleteIn([...props.path, "ref"]);
    doc.setIn([...props.path, "value"], "");
  });
}
function addChild(): void {
  const k = kind.value;
  if (k !== "all" && k !== "any") return;
  store.commitGuiEdit((doc) => {
    const seq = doc.getIn([...props.path, k], true);
    if (isSeq(seq)) seq.add(makeLeafNode(doc));
  });
}
function removeChild(index: number): void {
  const k = kind.value;
  if (k !== "all" && k !== "any") return;
  store.commitGuiEdit((doc) => doc.deleteIn([...props.path, k, index]));
}

function onSelect(segment: string, event: Event): void {
  setKey(segment, (event.target as HTMLSelectElement).value);
}
function onInput(segment: string, event: Event): void {
  setKey(segment, (event.target as HTMLInputElement).value);
}

const selectClass =
  "h-8 rounded-md border border-input bg-transparent px-2 text-xs focus:outline-none focus:ring-1 focus:ring-ring";
const inputClass =
  "h-8 min-w-40 flex-1 rounded-md border border-input bg-transparent px-2 font-mono text-xs focus:outline-none focus:ring-1 focus:ring-ring";
</script>

<template lang="hsml">
div(class="rounded-md border border-border bg-card/40 p-2")
  template(v-if="kind === 'all' || kind === 'any'")
    div(class="mb-2 flex items-center gap-2")
      span(class="rounded bg-muted px-1.5 py-0.5 text-[0.65rem] font-semibold uppercase tracking-wide text-muted-foreground") {{ kind }}
      span(class="text-xs text-muted-foreground") {{ kind === 'all' ? 'all of these match' : 'any of these match' }}
    div(class="space-y-2 border-l border-border pl-3")
      div(v-for="(childPath, i) in childPaths" :key="i" class="flex items-start gap-2")
        ConditionNode(:path="childPath" class="flex-1")
        button(type="button" class="mt-1 text-xs text-muted-foreground hover:text-destructive" @click="removeChild(i)") remove
      button(type="button" class="text-xs font-medium text-primary hover:underline" @click="addChild") + condition
  template(v-else-if="kind === 'not'")
    div(class="mb-2")
      span(class="rounded bg-muted px-1.5 py-0.5 text-[0.65rem] font-semibold uppercase tracking-wide text-muted-foreground") not
    div(class="border-l border-border pl-3")
      ConditionNode(v-if="childPaths[0]" :path="childPaths[0]")
  template(v-else-if="kind === 'leaf'")
    div(class="flex flex-wrap items-center gap-2")
      select(:value="leafField" :class="selectClass" @change="(e) => onSelect('field', e)")
        option(v-for="f in CONDITION_FIELDS" :key="f" :value="f") {{ f }}
      select(:value="leafOp" :class="selectClass" @change="(e) => onSelect('op', e)")
        option(v-for="o in CONDITION_OPS" :key="o" :value="o") {{ o }}
      div(class="inline-flex overflow-hidden rounded-md border border-input text-xs")
        button(type="button" :class="cn('px-2 py-1', !usesRef ? 'bg-primary text-primary-foreground' : 'text-muted-foreground')" @click="switchToValue") value
        button(type="button" :class="cn('px-2 py-1', usesRef ? 'bg-primary text-primary-foreground' : 'text-muted-foreground')" @click="switchToRef") ref
      input(v-if="usesRef" :value="refValue" :class="inputClass" placeholder="lists.spamDomains" @change="(e) => onInput('ref', e)")
      span(v-else-if="valueIsList" class="text-xs italic text-muted-foreground") inline list - edit in the YAML tab
      input(v-else :value="scalarValue" :class="inputClass" placeholder="value" @change="(e) => onInput('value', e)")
  template(v-else)
    p(class="text-xs italic text-muted-foreground") Unsupported condition shape - edit in the YAML tab.
</template>
