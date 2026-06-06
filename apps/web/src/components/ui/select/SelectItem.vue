<script setup lang="ts">
import { cn } from "@/lib/utils";
import type { SelectItemProps } from "reka-ui";
import { SelectItemIndicator, SelectItem, SelectItemText, useForwardProps } from "reka-ui";
import { computed } from "vue";
import IconCheck from "~icons/lucide/check";

const props = defineProps<SelectItemProps & { class?: string }>();

const forwarded = useForwardProps(
  computed(() => {
    const { class: _ignored, ...rest } = props;
    return rest;
  }),
);
</script>

<template lang="hsml">
SelectItem(v-bind="forwarded" :class="cn('relative flex w-full cursor-default select-none items-center rounded-sm py-1.5 pl-2 pr-8 text-sm outline-none focus:bg-accent focus:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50', props.class)")
  span(class="absolute right-2 flex size-3.5 items-center justify-center")
    SelectItemIndicator
      component(:is="IconCheck" class="size-4")
  SelectItemText
    slot
</template>
