<template>
  <div class="flex flex-col w-full items-center">
    <div class="flex my-10 font-mono font-bold text-5xl uppercase tracking-widest">
      <span>Lume</span><UIcon class="mx-2" name="i-lucide-lightbulb"></UIcon><span>Mans</span>
    </div>
    <SessionTimer class="h-8" />
    <TrackStatus class="w-3/4 h-24 mb-3" />
    <span class="font-mono h-[1em]">
      {{ captureEnabled ? `${fps.toFixed(1)} FPS (${lastFrameTime} ms)` : '' }}
    </span>
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
  invoke(enabled ? 'start_capture' : 'stop_capture').catch((e) => {
    console.error(e);
  });
});

watch(videoInputId, (id) => {
  if (id !== -1) {
    invoke('set_capture_device', { id }).catch((e) => {
      console.error(e);
    });
  }
});
</script>
