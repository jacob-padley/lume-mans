<template>
  <div class="flex flex-col w-full items-center">
    <UPageHeader title="Link Mans" />
    <TrackStatus class="w-3/4 h-24 mb-3" />
    <CaptureToggle
      v-model="captureEnabled"
      :disabled="videoInputId === -1"
      class="mt-10 mb-3 w-40"
    />
    <VideoInputMenu v-model="videoInputId" :disabled="captureEnabled" class="w-3/4" />
    <IntervalSlider
      v-model="capturesPerSecond"
      class="w-3/4 my-5"
      :disabled="captureEnabled"
      :min="0.25"
      :max="5"
      :step="0.25"
      :default-value="1"
    />
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';

const captureEnabled = ref(false);
const videoInputId = ref(-1);
const capturesPerSecond = ref(1);

watch(captureEnabled, (enabled) => {
  if (enabled) {
    invoke('start_capture');
  } else {
    invoke('stop_capture');
  }
});

watch(videoInputId, (id) => {
  if (id !== -1) {
    invoke('set_capture_device', { id });
  }
});

watch(
  capturesPerSecond,
  (fps) => {
    invoke('set_capture_interval', { interval: Math.round(1000 / fps) });
  },
  { immediate: true },
);
</script>
