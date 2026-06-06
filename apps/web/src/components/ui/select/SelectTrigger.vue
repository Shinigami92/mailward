<script setup lang="ts">
import { cn } from "@/lib/utils";
import type { SelectTriggerProps } from "reka-ui";
import { SelectIcon, SelectTrigger, useForwardProps } from "reka-ui";
import { computed } from "vue";
import IconChevronDown from "~icons/lucide/chevron-down";

const props = defineProps<SelectTriggerProps & { class?: string }>();

const forwarded = useForwardProps(
  computed(() => {
    const { class: _ignored, ...rest } = props;
    return rest;
  }),
);
</script>

<template lang="hsml">
SelectTrigger(v-bind="forwarded" :class="cn('flex h-9 w-full items-center justify-between gap-2 whitespace-nowrap rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm focus:outline-none focus:ring-1 focus:ring-ring disabled:cursor-not-allowed disabled:opacity-50 [&>span]:line-clamp-1', props.class)")
  slot
  SelectIcon(as-child)
    component(:is="IconChevronDown" class="size-4 opacity-50")
</template>
