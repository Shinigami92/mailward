<script setup lang="ts">
import YamlEditor from "@/components/YamlEditor.vue";
import {
  ACCOUNTS_QUERY,
  ACCOUNTS_YAML_QUERY,
  AUTH_COMPLETE,
  AUTH_START,
  UPDATE_ACCOUNTS_YAML,
} from "@/lib/graphql";
import { useMutation, useQuery } from "@urql/vue";
import { computed, ref } from "vue";

interface Account {
  id: string;
  vendor: string;
  username: string;
  imapHost: string;
  authMethod: string;
  authenticated: boolean;
}

const { data, executeQuery } = useQuery({
  query: ACCOUNTS_QUERY,
  requestPolicy: "cache-and-network",
});
const accounts = computed<Account[]>(() => data.value?.accounts ?? []);

const { executeMutation: startAuth } = useMutation(AUTH_START);
const { executeMutation: completeAuth } = useMutation(AUTH_COMPLETE);

const signingId = ref("");
const authorizeUrl = ref("");
const redirectUrl = ref("");
const authError = ref("");
const completing = ref(false);

async function beginSignIn(id: string) {
  authError.value = "";
  redirectUrl.value = "";
  authorizeUrl.value = "";
  signingId.value = id;
  const response = await startAuth({ account: id });
  if (response.error) {
    authError.value = response.error.message;
    signingId.value = "";
  } else {
    authorizeUrl.value = response.data?.authStart?.authorizeUrl ?? "";
  }
}

async function finishSignIn() {
  completing.value = true;
  authError.value = "";
  const response = await completeAuth({
    account: signingId.value,
    redirectUrl: redirectUrl.value,
  });
  completing.value = false;
  if (response.error) {
    authError.value = response.error.message;
  } else {
    cancel();
    executeQuery({ requestPolicy: "network-only" });
  }
}

function cancel() {
  signingId.value = "";
  authorizeUrl.value = "";
  redirectUrl.value = "";
  authError.value = "";
}
</script>

<template lang="hsml">
section(class="space-y-6")
  div(class="space-y-3")
    div(v-for="account in accounts" :key="account.id" class="rounded-lg border border-slate-200 bg-white p-4")
      div(class="flex flex-wrap items-center gap-3")
        div(class="flex-1")
          div(class="flex items-center gap-2")
            span(class="font-medium") {{ account.id }}
            span(class="rounded bg-slate-100 px-1.5 py-0.5 text-xs text-slate-500") {{ account.vendor }}
          p(class="text-sm text-slate-500") {{ account.username }}
        span(v-if="account.authenticated" class="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700 ring-1 ring-inset ring-emerald-200") signed in
        span(v-else class="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-600 ring-1 ring-inset ring-slate-200") not signed in
        button(type="button" class="rounded-md border border-slate-300 px-3 py-1.5 text-sm font-medium hover:bg-slate-50" @click="beginSignIn(account.id)") {{ account.authenticated ? "Re-sign in" : "Sign in" }}
      div(v-if="signingId === account.id" class="mt-4 space-y-3 border-t border-slate-100 pt-4")
        p(class="text-sm text-slate-600") Open this URL, sign in, then paste the redirect URL you land on:
        a(:href="authorizeUrl" target="_blank" rel="noreferrer" class="block truncate text-sm text-blue-600 underline") {{ authorizeUrl }}
        input(v-model="redirectUrl" placeholder="https://localhost/?code=..." class="w-full rounded-md border border-slate-300 px-3 py-2 text-sm")
        div(class="flex gap-2")
          button(type="button" :disabled="completing || !redirectUrl" class="rounded-md bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-50" @click="finishSignIn") Complete
          button(type="button" class="rounded-md border border-slate-300 px-4 py-2 text-sm font-medium hover:bg-slate-50" @click="cancel") Cancel
        p(v-if="authError" class="text-sm text-rose-600") {{ authError }}
  YamlEditor(label="accounts.yaml" description="Your mailboxes (id, vendor, username)." :query="ACCOUNTS_YAML_QUERY" field="accountsYaml" :mutation="UPDATE_ACCOUNTS_YAML")
</template>
