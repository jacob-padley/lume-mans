<template>
  <div
    :class="`flex items-center align-middle gap-1 font-mono font-md ${isStale ? 'text-red-500' : ''}`"
  >
    <span>{{ hours.toString().padStart(2, '0') }}</span
    >:<span>{{ minutes.toString().padStart(2, '0') }}</span
    >:<span>{{ seconds.toString().padStart(2, '0') }}</span>
    <UTooltip
      v-if="isStale"
      text="The track timer has not updated in over a minute"
      :ui="{
        text: 'font-mono',
      }"
    >
      <UIcon name="i-lucide-circle-alert" />
    </UTooltip>
  </div>
</template>

<script setup lang="ts">
const { hours, minutes, seconds } = useSessionTime();
// If the timer doesn't update in over a minute, turn it red to indicate that the app might be
// stuck. This is purely visual, the user is free to do with this information as they please.
const isStale = ref(false);
let staleTimerId = -1;

watch([hours, minutes, seconds], (_) => {
  isStale.value = false;
  resetStaleTimer();
});

function resetStaleTimer() {
  if (staleTimerId >= 0) {
    clearTimeout(staleTimerId);
  }
  staleTimerId = setTimeout(() => {
    isStale.value = true;
  }, 60000);
}

onMounted(() => {
  resetStaleTimer();
});
</script>
