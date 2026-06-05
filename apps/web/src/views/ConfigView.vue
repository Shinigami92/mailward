<script setup lang="ts">
import YamlEditor from "@/components/YamlEditor.vue";
import { CONFIG_YAML_QUERY, UPDATE_CONFIG_YAML } from "@/lib/graphql";
</script>

<template lang="hsml">
YamlEditor(label="config.yaml" description="Folder policy, CLI defaults and IMAP tuning." :query="CONFIG_YAML_QUERY" field="configYaml" :mutation="UPDATE_CONFIG_YAML")
</template>
