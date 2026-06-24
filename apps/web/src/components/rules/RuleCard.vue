<script setup lang="ts">
import ConditionNode from "@/components/rules/ConditionNode.vue";
import { injectRulesStore } from "@/composables/useRulesDocument";
import { RULE_DECISIONS } from "@/lib/rulesAst";
import { computed } from "vue";
import { isMap } from "yaml";
import type { YAMLMap } from "yaml";

const props = defineProps<{ path: Array<string | number> }>();
const emit = defineEmits<{ remove: [] }>();

const store = injectRulesStore();

const ruleNode = computed<YAMLMap | null>(() => {
  void store.docVersion.value;
  const node = store.doc.value.getIn(props.path, true);
  return isMap(node) ? node : null;
});
const name = computed(() => String(store.doc.value.getIn([...props.path, "name"]) ?? ""));
const decision = computed(() => String(store.doc.value.getIn([...props.path, "then"]) ?? "keep"));
// eemeli stores `commentBefore` without the leading '#'; it keeps the space after it, so strip one.
const comment = computed(() => {
  void store.docVersion.value;
  const raw = ruleNode.value?.commentBefore;
  return raw == null
    ? ""
    : raw
        .split("\n")
        .map((line) => line.replace(/^\s/u, ""))
        .join("\n");
});
const whenPath = computed(() => [...props.path, "when"]);

function setName(event: Event): void {
  store.commitGuiEdit((doc) =>
    doc.setIn([...props.path, "name"], (event.target as HTMLInputElement).value),
  );
}
function setDecision(event: Event): void {
  store.commitGuiEdit((doc) =>
    doc.setIn([...props.path, "then"], (event.target as HTMLSelectElement).value),
  );
}
function setComment(event: Event): void {
  const text = (event.target as HTMLTextAreaElement).value;
  store.commitGuiEdit(() => {
    const node = ruleNode.value;
    if (node == null) return;
    node.commentBefore =
      text === ""
        ? undefined
        : text
            .split("\n")
            .map((line) => ` ${line}`)
            .join("\n");
  });
}

const decisionClass: Record<string, string> = {
  keep: "border-emerald-500/40 text-emerald-600 dark:text-emerald-400",
  markRead: "border-sky-500/40 text-sky-600 dark:text-sky-400",
  delete: "border-rose-500/40 text-rose-600 dark:text-rose-400",
};
</script>

<template lang="hsml">
div(class="space-y-3 rounded-lg border border-border p-3")
  div(class="flex flex-wrap items-center gap-2")
    input(:value="name" placeholder="rule-name" class="h-8 min-w-48 flex-1 rounded-md border border-input bg-transparent px-2 text-sm font-medium focus:outline-none focus:ring-1 focus:ring-ring" @change="setName")
    select(:value="decision" :class="['h-8 rounded-md border bg-transparent px-2 text-xs font-medium focus:outline-none focus:ring-1 focus:ring-ring', decisionClass[decision]]" @change="setDecision")
      option(v-for="d in RULE_DECISIONS" :key="d" :value="d") {{ d }}
    button(type="button" class="text-xs text-muted-foreground hover:text-destructive" @click="emit('remove')") remove
  textarea(:value="comment" rows="1" placeholder="Comment (shown above the rule in YAML)" class="w-full rounded-md border border-input bg-transparent px-2 py-1 text-xs italic text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring" @change="setComment")
  ConditionNode(:path="whenPath")
</template>
