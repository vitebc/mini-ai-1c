import { invoke } from '@tauri-apps/api/core';
import { AppSettings, McpServerConfig } from '../types/settings';

export type { McpServerConfig, AppSettings };

export interface ExportSettingsResult {
    status: 'saved' | 'cancelled';
    path?: string | null;
}

/**
 * Get application settings
 */
export async function getSettings(): Promise<AppSettings> {
    return await invoke<AppSettings>('get_settings');
}

/**
 * Save application settings
 */
export async function saveSettings(newSettings: AppSettings): Promise<void> {
    return await invoke('save_settings', { newSettings });
}

export async function exportSettings(): Promise<ExportSettingsResult> {
    return await invoke<ExportSettingsResult>('export_settings');
}

export async function importSettings(jsonData: string): Promise<void> {
    await invoke<void>('import_settings', { jsonData });
}

export async function validateImportSettingsFile(filePath: string): Promise<void> {
    await invoke<void>('validate_import_settings_file', { filePath });
}

export async function importSettingsFromFile(filePath: string): Promise<void> {
    await invoke<void>('import_settings_from_file', { filePath });
}
