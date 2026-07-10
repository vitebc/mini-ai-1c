import { invoke } from '@tauri-apps/api/core';
import type { QuickActionAction } from '../types/quickActionSessions';

export interface WindowInfo {
    hwnd: number;
    title: string;
    process_id: number;
}

export type ConfiguratorWriteIntent =
    | 'replace_selection'
    | 'replace_current_method'
    | 'insert_before_current_method'
    | 'replace_module';

export interface ConfiguratorApplySupport {
    canApplyDirectly: boolean;
    preferredWriter: string;
    targetKind?: string | null;
    reason?: string | null;
}

export interface ConfiguratorPasteOptions {
    action?: QuickActionAction;
    writeIntent?: ConfiguratorWriteIntent;
    caretLine?: number | null;
    methodStartLine?: number | null;
    methodName?: string | null;
    runtimeId?: string | null;
    forceLegacyApply?: boolean;
}

/**
 * Find 1C Configurator windows matching the pattern
 */
export async function findConfiguratorWindows(pattern: string): Promise<WindowInfo[]> {
    return await invoke<WindowInfo[]>('find_configurator_windows_cmd', { pattern });
}

/**
 * Get code from specific Configurator window
 * @param hwnd Window handle
 * @param useSelectAll If true, sends Ctrl+A before Copy
 */
export async function getCodeFromConfigurator(hwnd: number, useSelectAll: boolean = false): Promise<string> {
    return await invoke<string>('get_code_from_configurator', { hwnd, useSelectAll });
}

/**
 * Get active fragment (selection or current line)
 */
export async function getActiveFragment(hwnd: number): Promise<string> {
    return await invoke<string>('get_active_fragment_cmd', { hwnd });
}

/**
 * Paste code to specific Configurator window
 * @param hwnd Window handle
 * @param code Code to paste
 * @param useSelectAll If true, sends Ctrl+A before Paste (replacing everything)
 * @param originalContent Original content for conflict detection
 */
export async function pasteCodeToConfigurator(
    hwnd: number,
    code: string,
    useSelectAll: boolean = false,
    originalContent?: string,
    options?: ConfiguratorPasteOptions,
): Promise<void> {
    return await invoke('paste_code_to_configurator', {
        hwnd,
        code,
        useSelectAll,
        originalContent: originalContent ?? null,
        action: options?.action ?? null,
        writeIntent: options?.writeIntent ?? null,
        caretLine: options?.caretLine ?? null,
        methodStartLine: options?.methodStartLine ?? null,
        methodName: options?.methodName ?? null,
        runtimeId: options?.runtimeId ?? null,
        forceLegacyApply: options?.forceLegacyApply ?? null,
    });
}

export async function getConfiguratorApplySupport(
    hwnd: number,
    useSelectAll: boolean = false,
    action?: 'describe' | 'elaborate' | 'fix' | 'explain' | 'review',
    writeIntent?: ConfiguratorWriteIntent,
    originalContent?: string,
): Promise<ConfiguratorApplySupport> {
    return await invoke<ConfiguratorApplySupport>('get_configurator_apply_support_cmd', {
        hwnd,
        useSelectAll,
        action: action ?? null,
        writeIntent: writeIntent ?? null,
        originalContent: originalContent ?? null,
    });
}

/**
 * Undo last code change in specific Configurator window
 */
export async function undoLastChange(hwnd: number): Promise<void> {
    return await invoke('undo_last_change', { hwnd });
}

/**
 * Check if there is an active selection in the window
 */
export async function checkSelectionState(hwnd: number): Promise<boolean> {
    return await invoke<boolean>('check_selection_state', { hwnd });
}

/**
 * Align active Configurator window and AI window
 */
export async function alignWithConfigurator(hwnd: number): Promise<void> {
    return await invoke('align_with_configurator', { hwnd });
}

/**
 * Set RDP compatibility mode for Configurator keyboard operations.
 * When enabled: disables 1C process filter and uses longer delays.
 */
export async function setConfiguratorRdpMode(enabled: boolean): Promise<void> {
    return await invoke('set_configurator_rdp_mode', { enabled });
}

export async function setConfiguratorEditorBridgeEnabled(enabled: boolean): Promise<void> {
    return await invoke('set_configurator_editor_bridge_enabled', { enabled });
}

export async function restartEditorBridge(): Promise<void> {
    return await invoke('restart_editor_bridge_cmd');
}
