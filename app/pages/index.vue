<template>
  <div class="flex flex-col w-full items-center">
    <UPageHeader title="Link Mans" />
    <TrackStatus class="w-3/4 h-24 mb-3" />
    <p class="font-mono h-[1em]">
      {{ captureEnabled ? `${fps.toFixed(1)} FPS (${lastFrameTime} ms)` : '' }}
    </p>
    <CaptureToggle
      v-model="captureEnabled"
      :disabled="videoInputId === -1"
      class="mt-10 mb-3 w-40"
    />
    <VideoInputMenu v-model="videoInputId" :disabled="captureEnabled" class="w-3/4" />
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useMetrics } from '~/composables/useMetrics';

const captureEnabled = ref(false);
const videoInputId = ref(-1);

const { lastFrameTime } = useMetrics();

const fps = computed(() => {
  if (lastFrameTime.value <= 0) {
    return 0;
  }
  return 1000.0 / lastFrameTime.value;
});

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
</script>
