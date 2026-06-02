import { listen } from '@tauri-apps/api/event';
import * as z from 'zod';

const TrackStatusSchema = z.union([
  z.literal('Waiting'),
  z.literal('SessionStart'),
  z.literal('GreenFlag'),
  z.literal('YellowFlag'),
  z.literal('FullCourseYellow'),
  z.literal('SafetyCar'),
  z.literal('VirtualSafetyCar'),
  z.literal('SafetyCarEnding'),
  z.literal('RedFlag'),
  z.literal('CheckeredFlag'),
]);
export type TrackStatus = z.infer<typeof TrackStatusSchema>;

export function useTrackStatus() {
  const status = ref<TrackStatus>('Waiting');

  listen<string>(
    'track-status',
    (event) => (status.value = TrackStatusSchema.parse(event.payload)),
  );

  return { status };
}
