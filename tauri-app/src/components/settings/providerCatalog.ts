export interface ProviderDefinition {
    value: string;
    label: string;
    defaultModel: string;
    defaultUrl: string;
    type: 'standard' | 'ollama-cloud' | 'cli' | 'naparnik';
}

export const MINIMAX_PROVIDER_DEFINITION = {
    value: 'MiniMax',
    label: 'MiniMax',
    defaultModel: 'MiniMax-M3',
    defaultUrl: 'https://api.minimax.io/v1',
    type: 'standard',
} satisfies ProviderDefinition;
