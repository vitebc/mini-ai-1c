import type { LLMProfile } from '../api/profiles';

export interface ModelMetadata {
    id: string;
    context_window?: number | null;
    default_reasoning_effort?: string | null;
    supported_reasoning_efforts?: string[] | null;
}

export interface ApplySelectedModelOptions {
    syncMaxTokens?: boolean;
}

const applyReasoningMetadata = (
    profile: LLMProfile,
    model: ModelMetadata,
): LLMProfile => {
    const supported = model.supported_reasoning_efforts;
    if (!supported?.length || (profile.reasoning_effort && supported.includes(profile.reasoning_effort))) {
        return profile;
    }

    const fallback = model.default_reasoning_effort;
    if (!fallback || !supported.includes(fallback)) {
        return profile;
    }

    return {
        ...profile,
        reasoning_effort: fallback as LLMProfile['reasoning_effort'],
    };
};

export const applySelectedModelMetadata = (
    profile: LLMProfile,
    model: ModelMetadata,
    options: ApplySelectedModelOptions = {},
): LLMProfile => {
    const next: LLMProfile = {
        ...profile,
        model: model.id,
        context_window_override: model.context_window ?? profile.context_window_override,
    };
    if (options.syncMaxTokens && model.context_window) {
        next.max_tokens = model.context_window;
    }
    return applyReasoningMetadata(next, model);
};

export const applyFetchedModelMetadata = (
    profile: LLMProfile,
    model: ModelMetadata,
): LLMProfile => applyReasoningMetadata(
    {
        ...profile,
        context_window_override: model.context_window ?? profile.context_window_override,
    },
    model,
);

