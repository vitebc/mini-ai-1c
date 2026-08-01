import { invoke } from '@tauri-apps/api/core';

export interface BslStatus {
    installed: boolean;
    java_info: string;
    runtime_info: string;
    server_version: string;
    server_path: string;
    workspace_path: string;
    active_port: number;
    connected: boolean;
    mcp_available: boolean;
}

export interface BslDiagnostic {
    line: number;
    character: number;
    message: string;
    severity: string;
}

export interface BslDiagnosticItem {
    status: 'ok' | 'warn' | 'error';
    title: string;
    message: string;
    suggestion?: string | null;
}

/**
 * Check BSL LS status
 */
export async function checkBslStatus(): Promise<BslStatus> {
    return await invoke<BslStatus>('check_bsl_status_cmd');
}

/**
 * Install BSL Language Server
 */
export async function installBslLs(): Promise<string> {
    return await invoke<string>('install_bsl_ls_cmd');
}

/**
 * Reconnect BSL Language Server
 */
export async function reconnectBslLs(): Promise<void> {
    return await invoke('reconnect_bsl_ls_cmd');
}

/**
 * Analyze BSL code
 */
export async function analyzeBsl(code: string): Promise<BslDiagnostic[]> {
    return await invoke<BslDiagnostic[]>('analyze_bsl', { code });
}

/**
 * Format BSL code
 */
export async function formatBsl(code: string): Promise<string> {
    return await invoke<string>('format_bsl', { code });
}

/**
 * Diagnose BSL LS launch issues
 */
export async function diagnoseBslLs(): Promise<BslDiagnosticItem[]> {
    return await invoke<BslDiagnosticItem[]>('diagnose_bsl_ls_cmd');
}
