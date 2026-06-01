<template>
  <div>
    <USlider
      v-model="model"
      :min="min"
      :max="max"
      :step="step"
      :default-value="defaultValue"
      :disabled="disabled"
    />
    <p :class="`font-mono text-sm mt-1 ${disabled ? 'text-slate-500' : ''}`">
      Capture Rate: {{ model }} FPS
    </p>
  </div>
</template>

<script setup lang="ts">
const model = defineModel<number>({ default: 1 });

defineProps<{
  min: number;
  max: number;
  step: number;
  defaultValue: number;
  disabled: boolean;
}>();
</script>
