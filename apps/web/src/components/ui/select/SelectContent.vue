<script setup lang="ts">
import { cn } from "@/lib/utils";
import type { SelectContentEmits, SelectContentProps } from "reka-ui";
import { SelectContent, SelectPortal, SelectViewport, useForwardPropsEmits } from "reka-ui";
import { computed } from "vue";

const props = withDefaults(defineProps<SelectContentProps & { class?: string }>(), {
  position: "popper",
});
const emits = defineEmits<SelectContentEmits>();

const forwarded = useForwardPropsEmits(
  computed(() => {
    const { class: _ignored, ...rest } = props;
    return rest;
  }),
  emits,
);
</script>

<template lang="hsml">
SelectPortal
  SelectContent(v-bind="forwarded" :class="cn('relative z-50 max-h-96 min-w-32 overflow-hidden rounded-md border bg-popover text-popover-foreground shadow-md data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95', props.class)")
    SelectViewport(class="h-[var(--reka-select-trigger-height)] w-full min-w-[var(--reka-select-trigger-width)] p-1")
      slot
</template>
