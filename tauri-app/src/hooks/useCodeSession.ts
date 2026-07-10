import { useEffect, useMemo, useReducer, useRef } from 'react';
import type { MutableRefObject } from 'react';

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

export interface CodeSessionState {
  /** Последний код, полученный из 1С Конфигуратора (загрузка / успешный Apply) */
  configuratorCode: string;
  /** Базовая линия для diff-preview (принятый пользователем вариант) */
  baselineCode: string;
  /** Текущий контент Monaco (рабочая копия) */
  workingCode: string;
  /** Код, загруженный через "выбрать" в Конфигураторе — используется как контекст в чате */
  loadedContextCode: string | null;
  /** true = выбранный фрагмент (selection), false = весь файл */
  isContextSelection: boolean;
}

export const INITIAL_CODE_SESSION_STATE: CodeSessionState = {
  configuratorCode: '',
  baselineCode: '',
  workingCode: '',
  loadedContextCode: null,
  isContextSelection: false,
};

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

type Action =
  | { type: 'loadFromConfigurator'; code: string; isSelection: boolean }
  | { type: 'applySucceeded' }
  | { type: 'syncBaseline'; code: string }
  | { type: 'userEdit'; code: string }
  | { type: 'applyAICode'; code: string }
  | { type: 'acceptDiff' }
  | { type: 'clearContext' }
  | { type: 'clearAll' };

// ---------------------------------------------------------------------------
// Reducer
// ---------------------------------------------------------------------------

export function codeSessionReducer(
  state: CodeSessionState,
  action: Action,
): CodeSessionState {
  switch (action.type) {
    // Загрузка кода из 1С: все поля синхронизируются
    case 'loadFromConfigurator':
      return {
        configuratorCode: action.code,
        baselineCode: action.code,
        workingCode: action.code,
        loadedContextCode: action.code,
        isContextSelection: action.isSelection,
      };

    // Успешный Apply в 1С: подтверждаем, что конфигуратор принял workingCode
    case 'applySucceeded':
      return {
        ...state,
        configuratorCode: state.workingCode,
        baselineCode: state.workingCode,
      };

    // Внешний сброс диффа: обновляем базовую линию и подтверждённый код из 1С,
    // но не трогаем workingCode, чтобы не терять локальные правки редактора.
    case 'syncBaseline':
      return {
        ...state,
        configuratorCode: action.code,
        baselineCode: action.code,
      };

    // Пользователь редактирует в Monaco
    case 'userEdit':
      return { ...state, workingCode: action.code };

    // Нажали «Применить» в чате (AI-дифф применён к workingCode)
    case 'applyAICode':
      return { ...state, workingCode: action.code };

    // Принятие диффа: baseline = workingCode (пользователь доволен)
    case 'acceptDiff':
      return { ...state, baselineCode: state.workingCode };

    // После отправки сообщения сбрасываем загруженный контекст
    case 'clearContext':
      return { ...state, loadedContextCode: null, isContextSelection: false };

    // Очистка чата / сессии
    case 'clearAll':
      return INITIAL_CODE_SESSION_STATE;

    default:
      return state;
  }
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export interface CodeSessionActions {
  loadFromConfigurator: (code: string, isSelection?: boolean) => void;
  applySucceeded: () => void;
  syncBaseline: (code: string) => void;
  userEdit: (code: string) => void;
  applyAICode: (code: string) => void;
  acceptDiff: () => void;
  clearContext: () => void;
  clearAll: () => void;
}

export interface UseCodeSessionReturn {
  state: CodeSessionState;
  /** Ref всегда содержит актуальный state — для event listeners без замыканий */
  stateRef: MutableRefObject<CodeSessionState>;
  actions: CodeSessionActions;
}

export function useCodeSession(): UseCodeSessionReturn {
  const [state, dispatch] = useReducer(codeSessionReducer, INITIAL_CODE_SESSION_STATE);

  // Ref синхронизируется с state после каждого рендера —
  // используется внутри Tauri event listeners вместо lastConfiguratorCodeRef
  const stateRef = useRef<CodeSessionState>(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const actions = useMemo<CodeSessionActions>(() => ({
    loadFromConfigurator: (code, isSelection = false) =>
      dispatch({ type: 'loadFromConfigurator', code, isSelection }),
    applySucceeded: () => dispatch({ type: 'applySucceeded' }),
    syncBaseline: (code) => dispatch({ type: 'syncBaseline', code }),
    userEdit: (code) => dispatch({ type: 'userEdit', code }),
    applyAICode: (code) => dispatch({ type: 'applyAICode', code }),
    acceptDiff: () => dispatch({ type: 'acceptDiff' }),
    clearContext: () => dispatch({ type: 'clearContext' }),
    clearAll: () => dispatch({ type: 'clearAll' }),
  }), []);

  return { state, stateRef, actions };
}
