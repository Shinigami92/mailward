<script setup lang="ts">
import type { SelectValueProps } from "reka-ui";
import { SelectValue } from "reka-ui";

defineProps<SelectValueProps>();
</script>

<template lang="hsml">
SelectValue(v-bind="$props")
  slot
</template>
