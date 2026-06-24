<script setup lang="ts">
import CodeEditor from "@/components/CodeEditor.vue";
import RuleEngine from "@/components/rules/RuleEngine.vue";
import { rulesStoreKey, useRulesDocument } from "@/composables/useRulesDocument";
import { RULES_YAML_QUERY, UPDATE_RULES_YAML } from "@/lib/graphql";
import { cn } from "@/lib/utils";
import { useMutation, useQuery } from "@urql/vue";
import { useDebounceFn } from "@vueuse/core";
import { computed, provide, ref, watch } from "vue";

const store = useRulesDocument();
provide(rulesStoreKey, store);

const activeTab = ref<"yaml" | "engine">("yaml");
const yamlText = ref("");
const savedSnapshot = ref("");
const loaded = ref(false);

const { data, fetching, error } = useQuery({ query: RULES_YAML_QUERY });
watch(
  data,
  (value) => {
    if (value == null || loaded.value) return;
    const raw = (value as Record<string, string>).rulesYaml ?? "";
    yamlText.value = raw;
    savedSnapshot.value = raw;
    store.seed(raw);
    loaded.value = true;
  },
  { immediate: true },
);

// Text edits: mirror immediately into the buffer (so Save and the dirty check are current), parse
// on a debounce. yamlText IS what the user sees and what we persist.
const debouncedIngest = useDebounceFn((next: string) => store.ingestText(next), 150);
function onYamlInput(next: string): void {
  yamlText.value = next;
  void debouncedIngest(next);
}

// GUI edits re-serialize the AST; mirror that back into the editor buffer. Seed/text edits already
// ARE the buffer, so they are skipped - otherwise we would reformat what the user is typing.
watch(
  () => store.docVersion.value,
  () => {
    if (store.origin.value === "gui") yamlText.value = store.text.value;
  },
);

// Opening the rule engine flushes the freshest text into the AST (the ingest debounce may be pending).
watch(activeTab, (tab) => {
  if (tab === "engine") store.ingestText(yamlText.value);
});

const dirty = computed(() => yamlText.value !== savedSnapshot.value);
const status = computed<{ label: string; class: string }>(() => {
  if (store.parseError.value !== null) {
    return {
      label: "Parse error",
      class: "border-destructive/40 bg-destructive/10 text-destructive",
    };
  }
  if (dirty.value) {
    return {
      label: "Unsaved changes",
      class: "border-amber-500/40 bg-amber-500/10 text-amber-600 dark:text-amber-400",
    };
  }
  return {
    label: "Synced",
    class: "border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
  };
});

const { executeMutation } = useMutation(UPDATE_RULES_YAML);
const saving = ref(false);
const saveError = ref<string | null>(null);
async function save(): Promise<void> {
  saving.value = true;
  saveError.value = null;
  const result = await executeMutation({ content: yamlText.value });
  saving.value = false;
  if (result.error == null) {
    savedSnapshot.value = yamlText.value;
  } else {
    saveError.value = result.error.message;
  }
}

function tabClass(tab: "yaml" | "engine"): string {
  return cn(
    "rounded px-3 py-1 text-sm font-medium transition-colors",
    activeTab.value === tab
      ? "bg-background text-foreground shadow-sm"
      : "text-muted-foreground hover:text-foreground",
  );
}
</script>

<template lang="hsml">
section(class="space-y-3")
  div
    h2(class="text-base font-semibold") rules.yaml
    p(class="text-sm text-muted-foreground") Sender lists, regex patterns, thresholds and the classify/cleanup rules. Edit as YAML or with the rule engine - changes sync both ways.
  div(class="flex flex-wrap items-center justify-between gap-3")
    div(class="inline-flex items-center gap-1 rounded-md border border-border bg-muted/40 p-0.5")
      button(type="button" :class="tabClass('yaml')" @click="activeTab = 'yaml'") YAML
      button(type="button" :class="tabClass('engine')" @click="activeTab = 'engine'") Rule engine
    div(class="flex items-center gap-3")
      span(:class="cn('rounded-full border px-2.5 py-0.5 text-xs font-medium', status.class)") {{ status.label }}
      button(type="button" :disabled="fetching || saving" class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 disabled:opacity-50" @click="save")
        span(v-if="saving") Saving...
        span(v-else) Save
  p(v-if="error" class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive") {{ error.message }}
  p(v-if="store.parseError.value" class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive") YAML error: {{ store.parseError.value }} - the rule engine shows the last valid state.
  p(v-if="saveError" class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive") {{ saveError }}
  div(v-show="activeTab === 'yaml'")
    CodeEditor(:value="yamlText" @update:value="onYamlInput")
  div(v-show="activeTab === 'engine'")
    RuleEngine
</template>
