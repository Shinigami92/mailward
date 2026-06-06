<script setup lang="ts">
import { ACCOUNTS_QUERY, INSPECT, INSPECTABLE_FOLDERS } from "@/lib/graphql";
import { useQuery } from "@urql/vue";
import { computed, ref, watch } from "vue";

interface Inspected {
  uid: string;
  folder: string;
  subject: string;
  from: string;
  fromName: string;
  isRead: boolean;
  ageHours: number;
  bodyPreview: string;
}

const { data: accountsData } = useQuery({ query: ACCOUNTS_QUERY });
const accounts = computed<Array<{ id: string }>>(() => accountsData.value?.accounts ?? []);
const account = ref("");
watch(
  accounts,
  (list) => {
    if (!account.value && list.length > 0) account.value = list[0].id;
  },
  { immediate: true },
);

const { data: foldersData } = useQuery({
  query: INSPECTABLE_FOLDERS,
  variables: computed(() => ({ account: account.value })),
  pause: computed(() => !account.value),
});
const folders = computed<string[]>(() => foldersData.value?.inspectableFolders ?? []);
const folder = ref("");
watch(
  folders,
  (list) => {
    if (list.length > 0 && !list.includes(folder.value)) {
      folder.value = list.includes("INBOX") ? "INBOX" : list[0];
    }
  },
  { immediate: true },
);

const limit = ref(50);
const unreadOnly = ref(false);

const { data, fetching, error, executeQuery } = useQuery({
  query: INSPECT,
  variables: computed(() => ({
    account: account.value,
    folder: folder.value,
    limit: limit.value,
    unreadOnly: unreadOnly.value,
  })),
  pause: true,
});
const messages = computed<Inspected[]>(() => data.value?.inspect ?? []);
const loaded = ref(false);

function load() {
  if (!account.value || !folder.value) return;
  loaded.value = true;
  executeQuery({ requestPolicy: "network-only" });
}

function formatAge(hours: number): string {
  return hours < 48 ? `${Math.round(hours)}h` : `${Math.round(hours / 24)}d`;
}
</script>

<template lang="hsml">
section(class="space-y-4")
  div(class="flex flex-wrap items-end gap-3")
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-ui-fg-faint") Account
      select(v-model="account" class="rounded-md border border-ui-line-strong bg-ui-surface px-3 py-2 text-sm text-ui-fg")
        option(v-for="a in accounts" :key="a.id" :value="a.id") {{ a.id }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-ui-fg-faint") Folder
      select(v-model="folder" class="rounded-md border border-ui-line-strong bg-ui-surface px-3 py-2 text-sm text-ui-fg")
        option(v-for="f in folders" :key="f" :value="f") {{ f }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-ui-fg-faint") Limit
      input(type="number" v-model.number="limit" min="1" max="500" class="w-24 rounded-md border border-ui-line-strong bg-ui-surface px-3 py-2 text-sm text-ui-fg")
    label(class="flex items-center gap-2 text-sm text-ui-fg-muted")
      input(type="checkbox" v-model="unreadOnly" class="size-4 rounded border-ui-line-strong")
      span Unread only
    button(type="button" :disabled="fetching || !folder" class="rounded-md bg-ui-accent px-4 py-2 text-sm font-medium text-ui-accent-fg hover:opacity-90 disabled:opacity-50" @click="load")
      span(v-if="fetching") Loading...
      span(v-else) Load
  p(class="text-xs text-ui-fg-faint") Read-only - inspecting a folder never marks mail as read, and confidential folders are not listed.
  p(v-if="error" class="rounded-md bg-rose-50 px-3 py-2 text-sm text-rose-700 dark:bg-rose-950 dark:text-rose-300") {{ error.message }}
  div(v-if="messages.length" class="space-y-2 sm:hidden")
    div(v-for="msg in messages" :key="msg.uid" class="space-y-1 rounded-lg border border-ui-line bg-ui-surface p-3")
      div(class="flex items-center justify-between gap-2")
        span(v-if="msg.isRead" class="rounded-full bg-ui-surface-2 px-2 py-0.5 text-xs text-ui-fg-muted") read
        span(v-else class="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700 dark:bg-blue-950 dark:text-blue-300") unread
        span(class="text-xs text-ui-fg-faint") {{ formatAge(msg.ageHours) }}
      div(class="font-medium") {{ msg.subject || "(no subject)" }}
      div(class="text-sm text-ui-fg-muted") {{ msg.fromName || msg.from }}
      div(v-if="msg.bodyPreview" class="line-clamp-2 text-xs text-ui-fg-faint") {{ msg.bodyPreview }}
  table(v-if="messages.length" class="hidden w-full table-fixed border-collapse text-sm sm:table")
    thead
      tr(class="border-b border-ui-line text-left text-xs uppercase tracking-wide text-ui-fg-faint")
        th(class="w-20 py-2 pr-3") State
        th(class="py-2 pr-3") Subject
        th(class="w-56 py-2 pr-3") From
        th(class="w-16 py-2") Age
    tbody
      tr(v-for="msg in messages" :key="msg.uid" class="border-b border-ui-line align-top")
        td(class="py-2 pr-3")
          span(v-if="msg.isRead" class="rounded-full bg-ui-surface-2 px-2 py-0.5 text-xs text-ui-fg-muted") read
          span(v-else class="rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-700 dark:bg-blue-950 dark:text-blue-300") unread
        td(class="py-2 pr-3")
          div(class="truncate font-medium") {{ msg.subject || "(no subject)" }}
          div(class="truncate text-xs text-ui-fg-faint") {{ msg.bodyPreview }}
        td(class="truncate py-2 pr-3 text-ui-fg-muted") {{ msg.fromName || msg.from }}
        td(class="py-2 text-ui-fg-faint") {{ formatAge(msg.ageHours) }}
  p(v-else-if="loaded && !fetching" class="text-sm text-ui-fg-muted") No messages in this folder.
  p(v-else class="text-sm text-ui-fg-faint") Pick a folder and press Load.
</template>
