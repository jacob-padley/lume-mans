<template>
  <div
    :class="`flex justify-center items-center font-mono uppercase text-xl tracking-wide ${statusClass(status)} rounded-xl`"
  >
    <p class="flex text-center">
      {{ prettyStatus(status) }}
    </p>
  </div>
</template>

<script setup lang="ts">
import { useTrackStatus } from '~/composables/useTrackStatus';

const { status } = useTrackStatus();

const prettyStatusMap = {
  Waiting: 'Waiting for Race Start',
  SessionStart: 'Waiting for Race Start',
  GreenFlag: 'Green Flag',
  YellowFlag: 'Yellow Flag',
  FullCourseYellow: 'Full Course Yellow',
  SafetyCar: 'Safety Car',
  VirtualSafetyCar: 'VSC',
  SafetyCarEnding: 'Safety Car Ending',
  RedFlag: 'Red Flag',
  CheckeredFlag: ' ', // No text since we will display a checkered background
};

function prettyStatus(status: TrackStatus) {
  const result = prettyStatusMap[status];
  if (result) {
    return result;
  }
  return status;
}

function statusClass(status: TrackStatus) {
  if (status === 'GreenFlag') {
    return 'bg-green-700';
  } else if (
    status === 'YellowFlag' ||
    status === 'FullCourseYellow' ||
    status === 'SafetyCar' ||
    status === 'VirtualSafetyCar' ||
    status === 'SafetyCarEnding'
  ) {
    return 'bg-yellow-600';
  } else if (status === 'RedFlag') {
    return 'bg-red-600';
  } else if (status === 'CheckeredFlag') {
    return 'bg-[conic-gradient(#000_90deg,#fff_90deg_180deg,#000_180deg_270deg,#fff_270deg)] bg-[length:4rem_4rem]';
  }

  return 'bg-slate-600';
}
</script>
