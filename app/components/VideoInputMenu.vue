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
    :highlight="!disabled && model === -1"
    :ui="{
      item: 'font-mono',
    }"
  />
</template>

<script setup lang="ts">
const model = defineModel<number>();
const props = defineProps<{
  inputs: VideoInputList;
  disabled: boolean;
}>();

type InputOption = {
  id: number;
  label: string;
  icon: string;
};

const selectedInputEntry = ref<InputOption>();

const displayedInputs = computed(() =>
  props.inputs.map((input) => ({
    id: input.id,
    label: input.is_primary ? `${input.name} (Primary Display)` : input.name,
    icon: 'i-lucide-monitor',
  })),
);

watch(selectedInputEntry, (selected) => {
  if (selected) {
    model.value = selected.id;
  } else {
    model.value = -1;
  }
});
</script>
