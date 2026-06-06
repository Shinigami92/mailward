<script setup lang="ts">
import type { DecisionKind } from "@/lib/decisions";
import { decisionClasses, decisionLabel } from "@/lib/decisions";

defineProps<{ decision: DecisionKind }>();
</script>

<template lang="hsml">
span(class="inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset" :class="decisionClasses(decision)") {{ decisionLabel(decision) }}
</template>
