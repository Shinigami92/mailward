<script setup lang="ts">
import DecisionBadge from "@/components/DecisionBadge.vue";
import {
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
import type { DecisionKind } from "@/lib/decisions";
import { ACCOUNTS_QUERY, RUN_PROGRESS, TRIGGER_RUN } from "@/lib/graphql";
import { cn } from "@/lib/utils";
import type { ColumnDef, ColumnFiltersState, SortingState } from "@tanstack/vue-table";
import {
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useVueTable,
} from "@tanstack/vue-table";
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
const selected = ref("all");
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
const decisions = computed<Decision[]>(() => result.value?.decisions ?? []);

// Sort decisions by action severity rather than alphabetically.
const DECISION_RANK: Record<DecisionKind, number> = { KEEP: 0, MARK_READ: 1, DELETE: 2 };
const DECISION_OPTIONS: Array<{ value: DecisionKind; label: string }> = [
  { value: "KEEP", label: "keep" },
  { value: "MARK_READ", label: "mark read" },
  { value: "DELETE", label: "delete" },
];

const columns: Array<ColumnDef<Decision>> = [
  {
    id: "folder",
    accessorFn: (d) => d.folder,
    filterFn: (row, id, value) => row.getValue<string>(id) === value,
  },
  { id: "from", accessorFn: (d) => d.from },
  { id: "rule", accessorFn: (d) => d.reason },
  {
    id: "decision",
    accessorFn: (d) => d.decision,
    sortingFn: (a, b) =>
      DECISION_RANK[a.getValue<DecisionKind>("decision")] -
      DECISION_RANK[b.getValue<DecisionKind>("decision")],
    filterFn: (row, id, value) =>
      Array.isArray(value) && value.includes(row.getValue<DecisionKind>(id)),
  },
  { id: "subject", accessorFn: (d) => d.subject },
];

const sorting = ref<SortingState>([]);
const globalFilter = ref("");
const decisionFacet = ref<DecisionKind[]>([]);
const folderFacet = ref("all");

const decisionFolders = computed<string[]>(() =>
  [...new Set(decisions.value.map((d) => d.folder))].toSorted(),
);
const columnFilters = computed<ColumnFiltersState>(() => {
  const filters: ColumnFiltersState = [];
  if (decisionFacet.value.length > 0) {
    filters.push({ id: "decision", value: decisionFacet.value });
  }
  if (folderFacet.value !== "all") {
    filters.push({ id: "folder", value: folderFacet.value });
  }
  return filters;
});

const table = useVueTable({
  get data() {
    return decisions.value;
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
  },
  onSortingChange: (updater) => {
    sorting.value = typeof updater === "function" ? updater(sorting.value) : updater;
  },
  onGlobalFilterChange: (updater) => {
    globalFilter.value = typeof updater === "function" ? updater(globalFilter.value) : updater;
  },
  globalFilterFn: (row, _columnId, value) => {
    const query = String(value).toLowerCase();
    const d = row.original;
    return `${d.folder} ${d.from} ${d.reason} ${d.decision} ${d.subject}`
      .toLowerCase()
      .includes(query);
  },
  getCoreRowModel: getCoreRowModel(),
  getSortedRowModel: getSortedRowModel(),
  getFilteredRowModel: getFilteredRowModel(),
});

function toggleSort(columnId: string): void {
  table.getColumn(columnId)?.toggleSorting();
}
function sortIcon(columnId: string): string {
  const dir = table.getColumn(columnId)?.getIsSorted();
  return dir === "asc" ? "▲" : dir === "desc" ? "▼" : "";
}
function toggleDecision(value: DecisionKind): void {
  decisionFacet.value = decisionFacet.value.includes(value)
    ? decisionFacet.value.filter((d) => d !== value)
    : [...decisionFacet.value, value];
}

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
    account: selected.value === "all" ? null : selected.value,
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
      label(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Account
      Select(v-model="selected")
        SelectTrigger(class="w-44")
          SelectValue(placeholder="All accounts")
        SelectContent
          SelectItem(value="all") All accounts
          SelectItem(v-for="account in accounts" :key="account.id" :value="account.id") {{ account.id }}
    div(class="flex flex-col gap-1")
      label(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Mode
      Select(v-model="mode")
        SelectTrigger(class="w-44")
          SelectValue
        SelectContent
          SelectItem(value="CLASSIFY") Classify (unread)
          SelectItem(value="CLEANUP") Cleanup (read)
    label(class="flex items-center gap-2 text-sm text-muted-foreground")
      input(type="checkbox" v-model="apply" class="size-4 rounded border-input")
      span Apply (not a dry run)
    Button(:variant="apply ? 'destructive' : 'default'" :disabled="fetching" @click="run")
      span(v-if="fetching") Running...
      span(v-else-if="apply") Apply
      span(v-else) Dry run
    span(v-if="live.length" class="text-sm text-muted-foreground") {{ live.length }} processed
  p(v-if="errorMessage" class="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive") {{ errorMessage }}
  p(v-if="!result && !fetching && !errorMessage" class="rounded-lg border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground") Choose a mode and press Dry run to classify your mail. Nothing changes unless you tick Apply.
  div(v-if="result" class="text-sm text-muted-foreground") Scanned {{ result.scanned }} · {{ result.applied ? "acted on" : "would act on" }} {{ result.actioned }} · applied: {{ result.applied }}
  p(v-if="result && !decisions.length" class="rounded-lg border border-dashed border-border px-4 py-8 text-center text-sm text-muted-foreground") Nothing to act on - the scanned mail already looks clean.
  div(v-if="decisions.length" class="space-y-3")
    div(class="flex flex-wrap items-center gap-3")
      Input(v-model="globalFilter" type="search" placeholder="Filter decisions..." class="min-w-48 flex-1 sm:max-w-xs")
      div(class="flex items-center gap-1.5")
        button(v-for="option in DECISION_OPTIONS" :key="option.value" type="button" :class="cn('rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors', decisionFacet.includes(option.value) ? 'border-primary bg-primary text-primary-foreground' : 'border-border text-muted-foreground hover:bg-muted')" @click="toggleDecision(option.value)") {{ option.label }}
      div(class="flex items-center gap-2")
        span(class="text-xs font-medium uppercase tracking-wide text-muted-foreground") Folder
        Select(v-model="folderFacet")
          SelectTrigger(class="w-44")
            SelectValue
          SelectContent
            SelectItem(value="all") All folders
            SelectItem(v-for="f in decisionFolders" :key="f" :value="f") {{ f }}
    div(class="rounded-md border border-border")
      Table
        TableHeader
          TableRow
            TableHead(class="hidden whitespace-nowrap lg:table-cell")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('folder')")
                span Folder
                span(v-if="sortIcon('folder')" class="text-[0.65rem] leading-none") {{ sortIcon('folder') }}
            TableHead(class="hidden whitespace-nowrap sm:table-cell")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('from')")
                span From
                span(v-if="sortIcon('from')" class="text-[0.65rem] leading-none") {{ sortIcon('from') }}
            TableHead(class="hidden whitespace-nowrap lg:table-cell")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('rule')")
                span Rule
                span(v-if="sortIcon('rule')" class="text-[0.65rem] leading-none") {{ sortIcon('rule') }}
            TableHead(class="w-28 whitespace-nowrap")
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('decision')")
                span Decision
                span(v-if="sortIcon('decision')" class="text-[0.65rem] leading-none") {{ sortIcon('decision') }}
            TableHead
              button(type="button" class="inline-flex items-center gap-1 hover:text-foreground" @click="toggleSort('subject')")
                span Subject
                span(v-if="sortIcon('subject')" class="text-[0.65rem] leading-none") {{ sortIcon('subject') }}
        TableBody
          TableRow(v-for="row in table.getRowModel().rows" :key="row.id")
            TableCell(class="hidden whitespace-nowrap text-muted-foreground lg:table-cell") {{ row.original.folder }}
            TableCell(class="hidden max-w-48 truncate whitespace-nowrap text-muted-foreground sm:table-cell") {{ row.original.from }}
            TableCell(class="hidden whitespace-nowrap text-muted-foreground lg:table-cell") {{ row.original.reason }}
            TableCell(class="whitespace-nowrap")
              DecisionBadge(:decision="row.original.decision")
            TableCell {{ row.original.subject || "(no subject)" }}
          TableEmpty(v-if="!table.getRowModel().rows.length" :colspan="5") No decisions match the filters.
</template>
