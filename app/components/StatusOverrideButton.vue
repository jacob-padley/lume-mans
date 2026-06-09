<template>
  <UButton @click="overrideStatus"><slot /></UButton>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';

const props = defineProps<{
  status: string;
}>();

function overrideStatus() {
  invoke('override_status', { status: props.status }).catch((e) => {
    console.error(e);
  });
}
</script>
