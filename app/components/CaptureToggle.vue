<template>
  <UButton
    :disabled="disabled"
    class="font-mono uppercase disabled:bg-slate-500"
    :color="model ? 'error' : 'success'"
    :icon="model ? 'i-lucide-octagon' : 'i-lucide-play'"
    @click="toggle"
    >{{ model ? 'stop' : 'start' }} Watcher</UButton
  >
</template>

<script setup lang="ts">
const model = defineModel<boolean>({ default: false });
defineProps<{
  disabled: boolean;
}>();

function toggle() {
  model.value = !model.value;
}
</script>
