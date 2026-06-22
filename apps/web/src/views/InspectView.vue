<script setup lang="ts">
import {
  Badge,
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Table,
  TableBody,
  TableCell,
  TableEmpty,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui";
import { ACCOUNTS_QUERY, INSPECT, INSPECTABLE_FOLDERS } from "@/lib/graphql";
import type {
  ColumnDef,
  ColumnFiltersState,
  ExpandedState,
  SortingState,
} from "@tanstack/vue-table";
import {
  getCoreRowModel,
  getExpandedRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useVueTable,
} from "@tanstack/vue-table";
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

const { data, fetching, error, executeQuery } = useQuery({
  query: INSPECT,
  variables: computed(() => ({
    account: account.value,
    folder: folder.value,
    limit: limit.value,
    unreadOnly: false,
  })),
  pause: true,
});
const messages = computed<Inspected[]>(() => data.value?.inspect ?? []);
const loaded = ref(false);

const columns: Array<ColumnDef<Inspected>> = [
  { id: "from", accessorFn: (m) => m.fromName || m.from },
  {
    id: "age",
    accessorFn: (m) => m.ageHours,
    filterFn: (row, id, value) => row.getValue<number>(id) <= Number(value),
  },
  {
    id: "state",
    accessorFn: (m) => (m.isRead ? 1 : 0),
    filterFn: (row, id, value) => row.getValue<number>(id) === Number(value),
  },
  { id: "subject", accessorFn: (m) => m.subject },
];

// Default to newest first: a smaller age (in hours) means a more recent message.
const sorting = ref<SortingState>([{ id: "age", desc: false }]);
const globalFilter = ref("");
const expanded = ref<ExpandedState>({});

// Faceted filters (driven by the Select controls, applied as TanStack column filters).
const stateFacet = ref<"all" | "unread" | "read">("all");
const ageFacet = ref<"all" | "24" | "168" | "720">("all");
const columnFilters = computed<ColumnFiltersState>(() => {
  const filters: ColumnFiltersState = [];
  if (stateFacet.value !== "all") {
    filters.push({ id: "state", value: stateFacet.value === "read" ? 1 : 0 });
  }
  if (ageFacet.value !== "all") {
    filters.push({ id: "age", value: Number(ageFacet.value) });
  }
  return filters;
});

const table = useVueTable({
  get data() {
    return messages.value;
  },
  columns,
  state: {
    get sorting() {
      return sorting.value;
    },
    get globalFilter() {
      return globalFilter.value;
    },
    get columnFilters() {
      return columnFilters.value;
    },
    get expanded() {
      return expanded.value;
    },
  },
  onSortingChange: (updater) => {
    sorting.value = typeof updater === "function" ? updater(sorting.value) : updater;
  },
  onGlobalFilterChange: (updater) => {
    globalFilter.value = typeof updater === "function" ? updater(globalFilter.value) : updater;
  },
  onExpandedChange: (updater) => {
    expanded.value = typeof updater === "function" ? updater(expanded.value) : updater;
  },
  globalFilterFn: (row, _columnId, value) => {
    const query = String(value).toLowerCase();
    const m = row.original;
    return `${m.subject} ${m.from} ${m.fromName} ${m.bodyPreview}`.toLowerCase().includes(query);
  },
  getRowCanExpand: () => true,
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  getFilteredRowModel: getFilteredRowModel(),
  getExpandedRowModel: getExpandedRowModel(),
});

function toggleSort(columnId: string): void {
  table.getColumn(columnId)?.toggleSorting();
}
function sortIcon(columnId: string): string {
  const dir = table.getColumn(columnId)?.getIsSorted();
  return dir === "asc" ? "▲" : dir === "desc" ? "▼" : "";
}

function load() {
  if (!account.value || !folder.value) return;
  loaded.value = true;
  executeQuery({ requestPolicy: "network-only" });
}

// Auto-load when the view opens and whenever the folder changes (an account change
// re-resolves the folder, which re-triggers this). Inspect is read-only (BODY.PEEK), so
// auto-loading never marks mail read. The Load button is a manual refresh.
watch(folder, () => load());

function formatAge(hours: number): string {
  return hours < 48 ? `${Math.round(hours)}h` : `${Math.round(hours / 24)}d`;
}
</script>

<template lang="hsml">
section(class="space-y-4")
  div(class="flex flex-wrap items-end gap-3")
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Account
      Select(v-model="account" :disabled="fetching")
        SelectTrigger(class="w-40")
          SelectValue(placeholder="Account")
        SelectContent
          SelectItem(v-for="a in accounts" :key="a.id" :value="a.id") {{ a.id }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Folder
      Select(v-model="folder" :disabled="fetching")
        SelectTrigger(class="w-52")
          SelectValue(placeholder="Folder")
        SelectContent
          SelectItem(v-for="f in folders" :key="f" :value="f") {{ f }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Limit
      input(type="number" v-model.number="limit" min="1" max="500" :disabled="fetching" class="h-9 w-24 rounded-md border border-input bg-transparent px-3 text-sm disabled:opacity-50")
    Button(:disabled="fetching || !folder" @click="load")
      span(v-if="fetching") Loading...
      span(v-else) Load
  p(class="text-xs text-muted-foreground") Read-only - inspecting a folder never marks mail as read, and confidential folders are not listed. Tap a row to expand the full preview.
  div(v-if="fetching" class="space-y-1.5")
    div(class="text-sm text-muted-foreground") Loading {{ folder }}...
    div(class="h-1.5 w-full overflow-hidden rounded-full bg-muted")
      div(class="mw-indeterminate h-full w-1/4 rounded-full bg-primary")
  p(v-if="error" class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive") {{ error.message }}
  div(v-if="messages.length" class="space-y-3")
    div(class="flex flex-wrap items-center gap-3")
      Input(v-model="globalFilter" type="search" placeholder="Filter messages..." class="min-w-48 flex-1 sm:max-w-xs")
      div(class="flex items-center gap-2")
        span(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") State
        Select(v-model="stateFacet")
          SelectTrigger(class="w-32")
            SelectValue
          SelectContent
            SelectItem(value="all") All
            SelectItem(value="unread") Unread
            SelectItem(value="read") Read
      div(class="flex items-center gap-2")
        span(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Age
        Select(v-model="ageFacet")
          SelectTrigger(class="w-32")
            SelectValue
          SelectContent
            SelectItem(value="all") All
            SelectItem(value="24") ≤ 24h
            SelectItem(value="168") ≤ 7d
            SelectItem(value="720") ≤ 30d
    div(class="rounded-md border border-border")
      Table(class="table-fixed")
        TableHeader
          TableRow
            TableHead(class="hidden w-72 md:table-cell")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('from')")
                span From
                span(v-if="sortIcon('from')" class="text-[0.65rem] leading-none") {{ sortIcon('from') }}
            TableHead(class="w-16")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('age')")
                span Age
                span(v-if="sortIcon('age')" class="text-[0.65rem] leading-none") {{ sortIcon('age') }}
            TableHead(class="hidden w-24 sm:table-cell")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('state')")
                span State
                span(v-if="sortIcon('state')" class="text-[0.65rem] leading-none") {{ sortIcon('state') }}
            TableHead
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('subject')")
                span Subject
                span(v-if="sortIcon('subject')" class="text-[0.65rem] leading-none") {{ sortIcon('subject') }}
        TableBody
          template(v-for="row in table.getRowModel().rows" :key="row.id")
            TableRow
              TableCell(class="hidden max-w-56 truncate text-muted-foreground md:table-cell") {{ row.original.fromName || row.original.from }}
              TableCell(class="whitespace-nowrap text-muted-foreground") {{ formatAge(row.original.ageHours) }}
              TableCell(class="hidden sm:table-cell")
                Badge(v-if="row.original.isRead" variant="secondary") read
                Badge(v-else variant="outline" class="border-transparent bg-blue-100 text-blue-700 dark:bg-blue-950 dark:text-blue-300") unread
              TableCell
                button(type="button" class="block w-full text-left" :aria-expanded="row.getIsExpanded()" @click="row.toggleExpanded()")
                  div(class="font-medium") {{ row.original.subject || "(no subject)" }}
                  div(v-if="row.original.bodyPreview && !row.getIsExpanded()" class="truncate text-xs text-muted-foreground") {{ row.original.bodyPreview }}
            TableRow(v-if="row.getIsExpanded()")
              TableCell(:colspan="4" class="bg-muted/30")
                div(class="space-y-2")
                  div(class="text-sm text-muted-foreground md:hidden") {{ row.original.fromName || row.original.from }}
                  div(class="whitespace-pre-wrap break-words text-xs text-muted-foreground") {{ row.original.bodyPreview }}
          TableEmpty(v-if="!table.getRowModel().rows.length" :colspan="4") No messages match the filters.
  p(v-if="loaded && !fetching && !messages.length" class="text-sm text-muted-foreground") No messages in this folder.
  p(v-if="!loaded && !fetching" class="text-sm text-muted-foreground") Pick a folder and press Load.
</template>
