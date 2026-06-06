<script setup lang="ts">
import { cn } from "@/lib/utils";
import type { ButtonVariants } from ".";
import { buttonVariants } from ".";

const props = defineProps<{
  variant?: ButtonVariants["variant"];
  size?: ButtonVariants["size"];
  type?: "button" | "submit" | "reset";
  class?: string;
}>();
</script>

<template lang="hsml">
button(:type="props.type ?? 'button'" :class="cn(buttonVariants({ variant: props.variant, size: props.size }), props.class)")
  slot
</template>
