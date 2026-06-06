<script setup lang="ts">
import { useDark, useToggle } from "@vueuse/core";
import IconFilter from "~icons/lucide/funnel";
import IconInbox from "~icons/lucide/inbox";
import IconMoon from "~icons/lucide/moon";
import IconSearch from "~icons/lucide/search";
import IconSettings from "~icons/lucide/settings";
import IconSun from "~icons/lucide/sun";
import IconUsers from "~icons/lucide/users";

// Class-based dark mode: follows the OS preference until the user toggles it, then
// persists the choice. The pre-paint script in index.html sets the class to match.
const isDark = useDark({ storageKey: "mailward-theme" });
const toggleDark = useToggle(isDark);

const links = [
  { to: "/", label: "Run", icon: IconInbox },
  { to: "/inspect", label: "Inspect", icon: IconSearch },
  { to: "/accounts", label: "Accounts", icon: IconUsers },
  { to: "/config", label: "Config", icon: IconSettings },
  { to: "/rules", label: "Rules", icon: IconFilter },
];
</script>

<template lang="hsml">
div(class="min-h-screen bg-background text-foreground")
  header(class="border-b border-border bg-card")
    div(class="mx-auto flex max-w-5xl items-center gap-3 px-4 py-3 sm:gap-6")
      h1(class="text-lg font-semibold tracking-tight") mailward
      nav(class="flex flex-1 gap-1 overflow-x-auto")
        RouterLink(v-for="link in links" :key="link.to" :to="link.to" :aria-label="link.label" class="flex items-center gap-1.5 rounded-md px-2.5 py-1.5 text-sm font-medium text-muted-foreground hover:bg-muted sm:px-3" active-class="bg-primary text-primary-foreground hover:bg-primary")
          component(:is="link.icon" class="size-4 shrink-0")
          span(class="hidden sm:inline") {{ link.label }}
      button(type="button" :aria-label="isDark ? 'Switch to light mode' : 'Switch to dark mode'" class="rounded-md p-2 text-muted-foreground hover:bg-muted" @click="toggleDark()")
        component(:is="isDark ? IconSun : IconMoon" class="size-4")
  main(class="mx-auto max-w-5xl px-4 py-6")
    RouterView
</template>
