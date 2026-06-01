<template>
  <div class="flex flex-col w-full items-center">
    <UPageHeader title="Link Mans" />
    <TrackStatus class="w-3/4 h-24 mb-3"> Track Clear </TrackStatus>
    <CaptureToggle
      v-model="captureEnabled"
      :disabled="videoInputId === -1"
      class="mt-10 mb-3 w-40"
    />
    <VideoInputMenu v-model="videoInputId" :disabled="captureEnabled" class="w-3/4" />
  </div>
</template>

<script setup lang="ts">
const captureEnabled = ref(false);
const videoInputId = ref(-1);
</script>
