<script setup lang="ts">
import { cn } from "@/lib/utils";

const props = defineProps<{ class?: string; colspan?: number }>();
</script>

<template lang="hsml">
tr
  td(:colspan="props.colspan ?? 1" :class="cn('p-6 text-center text-sm text-muted-foreground', props.class)")
    slot
</template>
