import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { LogFileStatus, LogViewerPage, LogViewerQuery } from './types';

export async function selectDirectory(
  defaultPath?: string
): Promise<string | null> {
  return open({
    directory: true,
    defaultPath,
  });
}

export async function clearAudioCache(): Promise<number> {
  return invoke<number>('clear_audio_cache');
}

export async function sendTestNotification(): Promise<void> {
  return invoke<void>('send_test_notification');
}

export async function listLogRecords(
  query: LogViewerQuery
): Promise<LogViewerPage> {
  return invoke<LogViewerPage>('list_log_records', { query });
}

export async function getLogFileStatus(): Promise<LogFileStatus> {
  return invoke<LogFileStatus>('get_log_file_status');
}
