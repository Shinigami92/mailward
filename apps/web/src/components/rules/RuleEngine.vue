<script setup lang="ts">
import RefsEditor from "@/components/rules/RefsEditor.vue";
import RuleCard from "@/components/rules/RuleCard.vue";
import { injectRulesStore } from "@/composables/useRulesDocument";
import { makeRuleNode } from "@/lib/rulesAst";
import { computed } from "vue";
import { isSeq } from "yaml";

const store = injectRulesStore();

function ruleCount(section: "classify" | "cleanup"): number {
  void store.docVersion.value;
  const seq = store.doc.value.getIn([section], true);
  return isSeq(seq) ? seq.items.length : 0;
}
const classifyCount = computed(() => ruleCount("classify"));
const cleanupCount = computed(() => ruleCount("cleanup"));

function addRule(section: "classify" | "cleanup"): void {
  store.commitGuiEdit((doc) => {
    if (!isSeq(doc.getIn([section], true))) doc.setIn([section], doc.createNode([]));
    const seq = doc.getIn([section], true);
    if (isSeq(seq)) seq.add(makeRuleNode(doc, `${section}-rule`));
  });
}
function removeRule(section: "classify" | "cleanup", index: number): void {
  store.commitGuiEdit((doc) => doc.deleteIn([section, index]));
}

const addButtonClass =
  "rounded-md border border-input bg-transparent px-3 py-1 text-xs font-medium hover:bg-muted";
</script>

<template lang="hsml">
div(class="space-y-6")
  RefsEditor
  section(class="space-y-3")
    div(class="flex items-center justify-between")
      h3(class="text-sm font-semibold uppercase tracking-wide text-muted-foreground") Classify rules ({{ classifyCount }})
      button(type="button" :class="addButtonClass" @click="addRule('classify')") + rule
    p(v-if="classifyCount === 0" class="text-sm text-muted-foreground") No classify rules.
    RuleCard(v-for="i in classifyCount" :key="'classify-' + i" :path="['classify', i - 1]" @remove="removeRule('classify', i - 1)")
  section(class="space-y-3")
    div(class="flex items-center justify-between")
      h3(class="text-sm font-semibold uppercase tracking-wide text-muted-foreground") Cleanup rules ({{ cleanupCount }})
      button(type="button" :class="addButtonClass" @click="addRule('cleanup')") + rule
    p(v-if="cleanupCount === 0" class="text-sm text-muted-foreground") No cleanup rules.
    RuleCard(v-for="i in cleanupCount" :key="'cleanup-' + i" :path="['cleanup', i - 1]" @remove="removeRule('cleanup', i - 1)")
</template>
