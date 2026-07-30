import { invoke } from '@tauri-apps/api/core';

export interface SkillFile {
    id: string;
    name: string;
    description: string;
    category: string;
    content: string;
}

export async function listSkills(): Promise<SkillFile[]> {
    return await invoke<SkillFile[]>('list_skills');
}

export async function getSkill(id: string): Promise<SkillFile> {
    return await invoke<SkillFile>('get_skill', { id });
}

export async function saveSkill(id: string, content: string): Promise<SkillFile> {
    return await invoke<SkillFile>('save_skill', { id, content });
}

export async function deleteSkill(id: string): Promise<void> {
    return await invoke<void>('delete_skill', { id });
}
