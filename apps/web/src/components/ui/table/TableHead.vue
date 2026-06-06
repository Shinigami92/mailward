<script setup lang="ts">
import { cn } from "@/lib/utils";

const props = defineProps<{ class?: string }>();
</script>

<template lang="hsml">
th(:class="cn('h-10 px-2 text-left align-middle text-xs font-medium uppercase tracking-wide text-muted-foreground', props.class)")
  slot
</template>
