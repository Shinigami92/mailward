<script setup lang="ts">
import DecisionBadge from "@/components/DecisionBadge.vue";
import type { DecisionKind } from "@/lib/decisions";
import { ACCOUNTS_QUERY, RUN_PROGRESS, TRIGGER_RUN } from "@/lib/graphql";
import { useMutation, useQuery, useSubscription } from "@urql/vue";
import { computed, ref } from "vue";

interface Decision {
  account: string;
  folder: string;
  uid: string;
  subject: string;
  from: string;
  decision: DecisionKind;
  reason: string;
}
interface RunResult {
  scanned: number;
  actioned: number;
  applied: boolean;
  decisions: Decision[];
}

const { data: accountsData } = useQuery({ query: ACCOUNTS_QUERY });
const accounts = computed<Array<{ id: string }>>(() => accountsData.value?.accounts ?? []);
const selected = ref("");
const mode = ref<"CLASSIFY" | "CLEANUP">("CLASSIFY");
const apply = ref(false);

const live = ref<Decision[]>([]);
useSubscription<{ runProgress: Decision }, { runProgress: Decision }>(
  { query: RUN_PROGRESS },
  (_previous, data) => {
    live.value.push(data.runProgress);
    return data;
  },
);

const { executeMutation, fetching } = useMutation(TRIGGER_RUN);
const result = ref<RunResult | null>(null);
const errorMessage = ref("");

async function run() {
  if (
    apply.value &&
    !window.confirm("Apply changes to your mailbox? This marks mail read / moves it to Trash.")
  ) {
    return;
  }
  errorMessage.value = "";
  result.value = null;
  live.value = [];
  const response = await executeMutation({
    account: selected.value || null,
    mode: mode.value,
    dryRun: !apply.value,
  });
  if (response.error) {
    errorMessage.value = response.error.message;
  } else {
    result.value = (response.data?.triggerRun as RunResult) ?? null;
  }
}
</script>

<template lang="hsml">
section(class="space-y-5")
  div(class="flex flex-wrap items-end gap-3")
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-slate-400") Account
      select(v-model="selected" class="rounded-md border border-slate-300 bg-white px-3 py-2 text-sm")
        option(value="") All accounts
        option(v-for="account in accounts" :key="account.id" :value="account.id") {{ account.id }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-slate-400") Mode
      select(v-model="mode" class="rounded-md border border-slate-300 bg-white px-3 py-2 text-sm")
        option(value="CLASSIFY") Classify (unread)
        option(value="CLEANUP") Cleanup (read)
    label(class="flex items-center gap-2 text-sm text-slate-600")
      input(type="checkbox" v-model="apply" class="size-4 rounded border-slate-300")
      span Apply (not a dry run)
    button(type="button" :disabled="fetching" class="rounded-md px-4 py-2 text-sm font-medium text-white disabled:opacity-50" :class="apply ? 'bg-rose-600 hover:bg-rose-500' : 'bg-slate-900 hover:bg-slate-700'" @click="run")
      span(v-if="fetching") Running...
      span(v-else-if="apply") Apply
      span(v-else) Dry run
    span(v-if="live.length" class="text-sm text-slate-500") {{ live.length }} processed
  p(v-if="errorMessage" class="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700") {{ errorMessage }}
  div(v-if="result" class="text-sm text-slate-600") Scanned {{ result.scanned }} · {{ result.applied ? "acted on" : "would act on" }} {{ result.actioned }} · applied: {{ result.applied }}
  table(v-if="result" class="w-full border-collapse text-sm")
    thead
      tr(class="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-400")
        th(class="py-2 pr-3") Decision
        th(class="py-2 pr-3") Folder
        th(class="py-2 pr-3") Subject
        th(class="py-2 pr-3") From
        th(class="py-2") Rule
    tbody
      tr(v-for="(item, index) in result.decisions" :key="index" class="border-b border-slate-100")
        td(class="py-2 pr-3")
          DecisionBadge(:decision="item.decision")
        td(class="py-2 pr-3 text-slate-500") {{ item.folder }}
        td(class="py-2 pr-3") {{ item.subject || "(no subject)" }}
        td(class="py-2 pr-3 text-slate-500") {{ item.from }}
        td(class="py-2 text-slate-400") {{ item.reason }}
</template>
