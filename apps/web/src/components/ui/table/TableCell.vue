<script setup lang="ts">
import { cn } from "@/lib/utils";

const props = defineProps<{ class?: string }>();
</script>

<template lang="hsml">
td(:class="cn('p-2 align-top', props.class)")
  slot
</template>
