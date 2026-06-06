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
      label(class="text-xs font-medium uppercase tracking-wide text-ui-fg-faint") Account
      select(v-model="selected" class="rounded-md border border-ui-line-strong bg-ui-surface px-3 py-2 text-sm text-ui-fg")
        option(value="") All accounts
        option(v-for="account in accounts" :key="account.id" :value="account.id") {{ account.id }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-ui-fg-faint") Mode
      select(v-model="mode" class="rounded-md border border-ui-line-strong bg-ui-surface px-3 py-2 text-sm text-ui-fg")
        option(value="CLASSIFY") Classify (unread)
        option(value="CLEANUP") Cleanup (read)
    label(class="flex items-center gap-2 text-sm text-ui-fg-muted")
      input(type="checkbox" v-model="apply" class="size-4 rounded border-ui-line-strong")
      span Apply (not a dry run)
    button(type="button" :disabled="fetching" class="rounded-md px-4 py-2 text-sm font-medium disabled:opacity-50" :class="apply ? 'bg-rose-600 text-white hover:bg-rose-500' : 'bg-ui-accent text-ui-accent-fg hover:opacity-90'" @click="run")
      span(v-if="fetching") Running...
      span(v-else-if="apply") Apply
      span(v-else) Dry run
    span(v-if="live.length" class="text-sm text-ui-fg-muted") {{ live.length }} processed
  p(v-if="errorMessage" class="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950 dark:text-rose-300") {{ errorMessage }}
  div(v-if="result" class="text-sm text-ui-fg-muted") Scanned {{ result.scanned }} · {{ result.applied ? "acted on" : "would act on" }} {{ result.actioned }} · applied: {{ result.applied }}
  p(v-if="result && !result.decisions.length" class="rounded-lg border border-dashed border-ui-line px-4 py-6 text-center text-sm text-ui-fg-muted") Nothing to act on - the scanned mail already looks clean.
  div(v-if="result && result.decisions.length" class="space-y-2 sm:hidden")
    div(v-for="(item, index) in result.decisions" :key="index" class="space-y-1 rounded-lg border border-ui-line bg-ui-surface p-3")
      div(class="flex items-center justify-between gap-2")
        DecisionBadge(:decision="item.decision")
        span(class="truncate text-xs text-ui-fg-faint") {{ item.folder }}
      div(class="font-medium") {{ item.subject || "(no subject)" }}
      div(class="truncate text-sm text-ui-fg-muted") {{ item.from }}
      div(v-if="item.reason" class="text-xs text-ui-fg-faint") {{ item.reason }}
  table(v-if="result && result.decisions.length" class="hidden w-full border-collapse text-sm sm:table")
    thead
      tr(class="border-b border-ui-line text-left text-xs uppercase tracking-wide text-ui-fg-faint")
        th(class="py-2 pr-3") Decision
        th(class="py-2 pr-3") Folder
        th(class="py-2 pr-3") Subject
        th(class="py-2 pr-3") From
        th(class="py-2") Rule
    tbody
      tr(v-for="(item, index) in result.decisions" :key="index" class="border-b border-ui-line")
        td(class="py-2 pr-3")
          DecisionBadge(:decision="item.decision")
        td(class="py-2 pr-3 text-ui-fg-muted") {{ item.folder }}
        td(class="py-2 pr-3") {{ item.subject || "(no subject)" }}
        td(class="py-2 pr-3 text-ui-fg-muted") {{ item.from }}
        td(class="py-2 text-ui-fg-faint") {{ item.reason }}
</template>
