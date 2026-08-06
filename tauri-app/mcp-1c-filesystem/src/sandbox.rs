//! Резолвинг и валидация путей внутри sandbox.
//!
//! Порт логики `resolveSandboxPath` из 1c-filesystem.ts. Отличие от TS-версии:
//! используется канонизация через `canonicalize`, что исключает обход через
//! `..`, симлинки и префиксные коллизии (`/sandbox` vs `/sandbox-other`).

use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Sandbox {
    root: PathBuf,
    canonical: PathBuf,
}

impl Sandbox {
    /// Создаёт sandbox по пути из переменной окружения.
    /// Возвращает `None`, если путь пуст или не существует.
    pub fn from_env() -> Option<Self> {
        let raw = std::env::var("MINI_AI_1C_SANDBOX_PATH")
            .unwrap_or_default()
            .trim()
            .to_string();
        if raw.is_empty() {
            return None;
        }
        let root = PathBuf::from(&raw);
        if !root.is_dir() {
            return None;
        }
        Self::from_env_with_test_root(root)
    }

    /// Создаёт sandbox из явно заданного корня (используется в тестах).
    pub fn from_env_with_test_root(root: PathBuf) -> Option<Self> {
        let canonical = root.canonicalize().ok()?;
        Some(Self { root, canonical })
    }

    /// Резолвит относительный путь внутри sandbox.
    ///
    /// Возвращает `None`, если путь выходит за пределы sandbox
    /// (через `..`, абсолютные пути, симлинки).
    pub fn resolve(&self, requested: &str) -> Option<PathBuf> {
        if requested.is_empty() {
            return None;
        }
        // Отклоняем любые `..`-сегменты (как в TS-версии)
        if requested.split(['/', '\\']).any(|s| s == "..") {
            return None;
        }
        let candidate = self.root.join(requested);
        // Канонизируем существующую часть для защиты от симлинков
        let resolved = candidate.canonicalize().unwrap_or(candidate);
        if !resolved.starts_with(&self.canonical) {
            return None;
        }
        Some(resolved)
    }

    /// Резолвит путь для записи (каталог может ещё не существовать).
    ///
    /// Не требует `canonicalize` существующей части. Отбрасывает `..`-сегменты
    /// (путь не может выйти за корень), но явно отклоняет входные пути с `..`,
    /// чтобы сохранить семантику TS-версии ("Path escapes sandbox").
    pub fn resolve_for_write(&self, requested: &str) -> Option<PathBuf> {
        if requested.is_empty() {
            return None;
        }
        if requested.split(['/', '\\']).any(|s| s == "..") {
            return None;
        }
        let candidate = self.root.join(requested);
        let normalized = normalize_for_write(&candidate, &self.root);
        if normalized.starts_with(&self.canonical) {
            Some(normalized)
        } else {
            None
        }
    }
}

/// Нормализует путь для записи, отбрасывая `..`-сегменты и «склеивая» их.
///
/// Работает покомпонентно от корня sandbox: `..` отбрасывает предыдущий сегмент.
fn normalize_for_write(candidate: &Path, root: &Path) -> PathBuf {
    let mut out = root.to_path_buf();
    for comp in candidate.strip_prefix(root).unwrap_or(candidate).components() {
        use std::path::Component;
        match comp {
            Component::Normal(c) => out.push(c),
            Component::ParentDir => {
                if out != root {
                    out.pop();
                }
            }
            Component::CurDir => {}
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_within_sandbox() {
        let dir = std::env::temp_dir().join(format!("mcp-fs-sb-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sb = Sandbox::from_env_with_test_root(dir.clone()).unwrap();

        assert!(sb.resolve("file.txt").is_some());
        assert!(sb.resolve("sub/file.txt").is_some());
        // Выход за пределы через ..
        assert!(sb.resolve("..").is_none());
        assert!(sb.resolve("../../etc/passwd").is_none());
        // Абсолютные пути
        assert!(sb.resolve("/etc/passwd").is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resolve_for_write_handles_parent() {
        let dir = std::env::temp_dir().join(format!("mcp-fs-sb-w-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sb = Sandbox::from_env_with_test_root(dir.clone()).unwrap();

        assert!(sb.resolve_for_write("a/b/c.txt").is_some());
        // Любой путь с .. отклоняется (как в TS-версии)
        assert!(sb.resolve_for_write("a/../b.txt").is_none());
        assert!(sb.resolve_for_write("../escape.txt").is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
