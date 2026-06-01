<template>
  <USelectMenu
    placeholder="Select Video Input Device"
    :items="inputs"
    :search-input="false"
    class="font-mono justify-center"
    :ui="{
      item: 'font-mono',
    }"
  />
</template>

<script setup lang="ts">
import { useVideoInputs } from '~/composables/useVideoInputs';

const { availableInputs } = useVideoInputs();

const inputs = computed(() =>
  availableInputs.value.map((input) =>
    input.isPrimary ? `${input.name} (Primary Display)` : input.name,
  ),
);
</script>
