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
      :disabled="videoInputId === -1 && availableInputs.length > 0"
      class="mt-7 mb-3 w-40"
    />
    <VideoInputMenu
      v-model="videoInputId"
      :inputs="availableInputs"
      :disabled="captureEnabled"
      class="w-3/4"
    />
    <span class="font-mono text-sm text-slate-400 uppercase mt-8 mb-3">Override Track Status</span>
    <StatusOverrideButton
      class="mb-2 font-mono bg-slate-500 hover:bg-slate-600 active:bg-slate-600 text-slate-100 uppercase"
      status="SessionStart"
      >Pre-Start</StatusOverrideButton
    >
    <StatusOverrideButton
      class="bg-green-600 hover:bg-green-700 active:bg-green-700 font-mono text-slate-100 uppercase"
      status="GreenFlag"
      >Green Flag</StatusOverrideButton
    >
  </div>
</template>

<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core';
import { useMetrics } from '~/composables/useMetrics';
import { useVideoInputs } from '~/composables/useVideoInputs';
import { useErrors } from '~/composables/useErrors';

const captureEnabled = ref(false);
const videoInputId = ref(-1);

const { lastFrameTime } = useMetrics();
const { availableInputs } = useVideoInputs();
const toast = useToast();
const { copy } = useClipboard();

useErrors((error) => {
  toast.add({
    title: 'Unexpected Error',
    description: error,
    icon: 'i-lucide-circle-alert',
    color: 'error',
    actions: [
      {
        icon: 'i-lucide-files',
        label: 'Copy Error',
        color: 'neutral',
        variant: 'outline',
        onClick: (e) => {
          e?.stopPropagation();
          copy(error);
        },
      },
    ],
  });
});

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
