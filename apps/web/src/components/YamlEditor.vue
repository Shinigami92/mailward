<script setup lang="ts">
import type { TypedDocumentNode } from "@urql/vue";
import { useMutation, useQuery } from "@urql/vue";
import { ref, watch } from "vue";

const props = defineProps<{
  label: string;
  description?: string;
  query: TypedDocumentNode<any, any>;
  field: string;
  mutation: TypedDocumentNode<any, { content: string }>;
}>();

const { data, fetching, error } = useQuery({ query: props.query });
const content = ref("");
const loaded = ref(false);
watch(
  data,
  (value) => {
    if (value && !loaded.value) {
      content.value = (value as Record<string, string>)[props.field] ?? "";
      loaded.value = true;
    }
  },
  { immediate: true },
);

const { executeMutation } = useMutation(props.mutation);
const status = ref<{
  kind: "idle" | "saving" | "saved" | "error";
  message?: string;
}>({
  kind: "idle",
});

async function save() {
  status.value = { kind: "saving" };
  const result = await executeMutation({ content: content.value });
  status.value = result.error
    ? { kind: "error", message: result.error.message }
    : { kind: "saved" };
}
</script>

<template lang="hsml">
section(class="space-y-3")
  div
    h2(class="text-base font-semibold") {{ label }}
    p(v-if="description" class="text-sm text-slate-500") {{ description }}
  p(v-if="error" class="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700") {{ error.message }}
  textarea(v-model="content" spellcheck="false" class="h-96 w-full rounded-md border border-slate-300 bg-white p-3 font-mono text-xs leading-relaxed shadow-inner focus:border-slate-500 focus:outline-none")
  div(class="flex items-center gap-3")
    button(type="button" :disabled="fetching || status.kind === 'saving'" class="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-50" @click="save") Save
    span(v-if="status.kind === 'saved'" class="text-sm text-emerald-600") Saved.
    span(v-if="status.kind === 'error'" class="text-sm text-rose-600") {{ status.message }}
</template>
