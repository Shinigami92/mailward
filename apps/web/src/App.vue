<script setup lang="ts">
import IconFilter from "~icons/lucide/funnel";
import IconInbox from "~icons/lucide/inbox";
import IconSearch from "~icons/lucide/search";
import IconSettings from "~icons/lucide/settings";
import IconUsers from "~icons/lucide/users";

const links = [
  { to: "/", label: "Run", icon: IconInbox },
  { to: "/inspect", label: "Inspect", icon: IconSearch },
  { to: "/accounts", label: "Accounts", icon: IconUsers },
  { to: "/config", label: "Config", icon: IconSettings },
  { to: "/rules", label: "Rules", icon: IconFilter },
];
</script>

<template lang="hsml">
div(class="min-h-screen bg-slate-50 text-slate-900")
  header(class="border-b border-slate-200 bg-white")
    div(class="mx-auto flex max-w-5xl items-center gap-6 px-4 py-3")
      h1(class="text-lg font-semibold tracking-tight") mailward
      nav(class="flex flex-1 gap-1 overflow-x-auto")
        RouterLink(v-for="link in links" :key="link.to" :to="link.to" class="flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium text-slate-600 hover:bg-slate-100" active-class="bg-slate-900 text-white hover:bg-slate-900")
          component(:is="link.icon" class="size-4")
          span {{ link.label }}
  main(class="mx-auto max-w-5xl px-4 py-6")
    RouterView
</template>
