<template>
  <UButton class="font-mono" :color="model ? 'error' : 'success'" @click="toggle"
    >{{ model ? 'Stop' : 'Start' }} Watcher</UButton
  >
</template>

<script setup lang="ts">
const model = defineModel<boolean>({ default: false });

function toggle() {
  model.value = !model.value;
}
</script>
