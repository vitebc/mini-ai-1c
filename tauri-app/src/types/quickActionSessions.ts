import type { BslDiagnostic } from '../api/bsl';

export type QuickActionAction = 'describe' | 'elaborate' | 'fix' | 'explain' | 'review';

export type QuickActionCaptureScope = 'selection' | 'current_method' | 'module';

export type QuickActionWriteIntent =
    | 'replace_selection'
    | 'replace_current_method'
    | 'insert_before_current_method'
    | 'replace_module';

export interface OverlayQuickActionSessionPayload {
    action: 'elaborate' | 'fix' | 'review';
    mode: 'write' | 'chat';
    confHwnd: number;
    scope: QuickActionCaptureScope;
    code: string;
    originalCode: string;
    useSelectAll: boolean;
    writeIntent?: QuickActionWriteIntent;
    caretLine?: number | null;
    methodStartLine?: number | null;
    methodName?: string | null;
    runtimeId?: string | null;
    task?: string | null;
    diagnostics?: BslDiagnostic[] | null;
    diagnosticsError?: string | null;
}
