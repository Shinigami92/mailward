<script setup lang="ts">
import { cn } from "@/lib/utils";

const props = defineProps<{ class?: string }>();
</script>

<template lang="hsml">
tbody(:class="cn('[&_tr:last-child]:border-0', props.class)")
  slot
</template>
