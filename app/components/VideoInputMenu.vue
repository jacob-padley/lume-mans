<template>
  <USelectMenu
    v-model="selectedInputEntry"
    placeholder="Select Video Input Device"
    :items="inputs"
    :disabled="disabled"
    :search-input="false"
    class="font-mono justify-center"
    :highlight="!disabled && model === -1"
    :ui="{
      item: 'font-mono',
    }"
  />
</template>

<script setup lang="ts">
import { useVideoInputs } from '~/composables/useVideoInputs';

const selectedInputEntry = ref<{ id: number; label: string; icon: string }>();
const { availableInputs } = useVideoInputs();

const model = defineModel<number>();
defineProps<{
  disabled: boolean;
}>();

watch(selectedInputEntry, (selected) => {
  if (selected) {
    model.value = selected.id;
  } else {
    model.value = -1;
  }
});

const inputs = computed(() =>
  availableInputs.value.map((input) => ({
    id: input.id,
    label: input.is_primary ? `${input.name} (Primary Display)` : input.name,
    icon: 'i-lucide-monitor',
  })),
);
</script>
