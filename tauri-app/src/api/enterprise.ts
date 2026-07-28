import { invoke } from '@tauri-apps/api/core';

export interface EnterpriseStatus {
    enabled: boolean;
    server_url: string;
}

export interface UpdateCheckResult {
    available: boolean;
    version: string | null;
    url: string | null;
    changelog: string | null;
}

export async function getEnterpriseStatus(): Promise<EnterpriseStatus> {
    return await invoke<EnterpriseStatus>('get_enterprise_status');
}

export async function fetchEnterpriseConfig(): Promise<boolean> {
    return await invoke<boolean>('fetch_enterprise_config');
}

export async function checkForUpdates(): Promise<UpdateCheckResult> {
    return await invoke<UpdateCheckResult>('check_for_updates');
}

export async function downloadUpdate(version: string, url: string): Promise<string> {
    return await invoke<string>('download_update', { version, url });
}
