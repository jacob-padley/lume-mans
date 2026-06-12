<template>
  <USelectMenu
    v-if="props.inputs.length > 0"
    v-model="selectedInputEntry"
    placeholder="Select Video Source"
    icon="i-lucide-monitor"
    :items="displayedInputs"
    :disabled="disabled"
    :search-input="false"
    class="font-mono justify-center"
    :highlight="!disabled && !model"
    :ui="{
      item: 'font-mono',
    }"
    @update:open="(isOpen) => isOpen && emit('open')"
  />
</template>

<script setup lang="ts">
import type { SourceType, VideoInputList } from '~/composables/useVideoInputs';

const model = defineModel<{ id: number; source_type: SourceType }>();
const props = defineProps<{
  inputs: VideoInputList;
  disabled: boolean;
}>();
const emit = defineEmits<{
  (e: 'open'): void;
}>();

type InputOption = {
  id: number;
  label: string;
  icon: string;
  source_type: SourceType;
};

const selectedInputEntry = ref<InputOption>();

const displayedInputs = computed(() => {
  const inputs = props.inputs.map((input) => ({
    id: input.id,
    label: input.is_primary ? `${input.name} (Primary Display)` : input.name,
    source_type: input.source_type,
    icon: input.source_type === 'Monitor' ? 'i-lucide-monitor' : 'i-lucide-app-window',
  }));
  const monitorInputs = inputs.filter((input) => input.source_type === 'Monitor');
  const windowInputs = inputs.filter((input) => input.source_type === 'Window');
  return [monitorInputs, windowInputs];
});

watch(selectedInputEntry, (selected) => {
  if (selected) {
    model.value = {
      id: selected.id,
      source_type: selected.source_type,
    };
  } else {
    model.value = undefined;
  }
});
</script>
