<script setup lang="ts">
import YamlEditor from "@/components/YamlEditor.vue";
import { RULES_YAML_QUERY, UPDATE_RULES_YAML } from "@/lib/graphql";
</script>

<template lang="hsml">
YamlEditor(label="rules.yaml" description="Sender lists, regex patterns, thresholds and the classify/cleanup rules." :query="RULES_YAML_QUERY" field="rulesYaml" :mutation="UPDATE_RULES_YAML")
</template>
