//! BM25 in-memory поисковый движок для скиллов, документации и правил.
//!
//! Реализация BM25 (Okapi BM25) без внешних зависимостей:
//! - инвертированный индекс: терм → список (doc_idx, term_freq);
//! - IDF кэшируется при сборке индекса;
//! - ранжирование по формуле: Σ IDF(t) × tf_norm(t, d).
//!
//! Параметры: k1 = 1.5 (насыщение частоты терма), b = 0.75 (нормализация длины).

use std::collections::HashMap;

use crate::skills::{DocInfo, RuleInfo, SkillInfo};

const K1: f64 = 1.5;
const B: f64 = 0.75;

/// Постинг-лист инвертированного индекса: (doc_idx, term_freq).
type Postings = Vec<(usize, u32)>;

/// Скоринг-источник: (idf, постинг-лист, boost).
type ScoringSource<'a> = (f64, &'a Postings, f64);

#[derive(Debug, Clone, PartialEq)]
pub enum DocKind {
    Skill,
    Doc,
    Rule,
}

impl DocKind {
    pub fn from_str(s: &str) -> Option<DocKind> {
        match s {
            "skill" => Some(DocKind::Skill),
            "doc" => Some(DocKind::Doc),
            "rule" => Some(DocKind::Rule),
            _ => None,
        }
    }
}

struct Bm25Doc {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: DocKind,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub len: usize,
}

pub struct Bm25 {
    docs: Vec<Bm25Doc>,
    inverted: HashMap<String, Postings>,
    idf: HashMap<String, f64>,
    avgdl: f64,
    n: usize,
}

pub struct SearchHit {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kind: DocKind,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub score: f64,
}

/// Разбивает строку на токены: lowercase, сплит по разделителям,
/// отбрасывает токены короче 2 символов (шум: "c", "1", "и").
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| {
            c.is_whitespace()
                || matches!(
                    c,
                    ',' | '.'
                        | '_'
                        | '-'
                        | '/'
                        | ':'
                        | ';'
                        | '('
                        | ')'
                        | '['
                        | ']'
                        | '<'
                        | '>'
                        | '|'
                        | '«'
                        | '»'
                        | '№'
                        | '—'
                )
        })
        .map(|s| {
            s.trim_matches(|c: char| c.is_ascii_punctuation() || c == '"' || c == '\'' || c == '`')
                .to_string()
        })
        .filter(|s| !s.is_empty() && s.len() >= 2)
        .collect()
}

impl Bm25 {
    /// Собирает индекс по всем трём коллекциям.
    pub fn build(skills: &[SkillInfo], docs: &[DocInfo], rules: &[RuleInfo]) -> Self {
        let mut engine = Bm25 {
            docs: Vec::with_capacity(skills.len() + docs.len() + rules.len()),
            inverted: HashMap::new(),
            idf: HashMap::new(),
            avgdl: 0.0,
            n: 0,
        };

        let mut total_len = 0usize;

        for s in skills {
            let tags = s.tags.join(" ");
            let text = format!(
                "{} {} {} {} {}",
                s.name,
                s.description,
                s.id.replace('/', " "),
                s.category.as_deref().unwrap_or(""),
                tags
            );
            total_len += engine.push_doc(
                s.id.clone(),
                s.name.clone(),
                s.description.clone(),
                DocKind::Skill,
                s.category.clone(),
                s.tags.clone(),
                &text,
            );
        }
        for d in docs {
            let text = format!(
                "{} {} {} {}",
                d.name,
                d.description,
                d.id.replace('/', " "),
                d.category.as_deref().unwrap_or("")
            );
            total_len += engine.push_doc(
                d.id.clone(),
                d.name.clone(),
                d.description.clone(),
                DocKind::Doc,
                d.category.clone(),
                Vec::new(),
                &text,
            );
        }
        for r in rules {
            let text = format!("{} {} {}", r.name, r.description, r.id.replace('/', " "));
            total_len += engine.push_doc(
                r.id.clone(),
                r.name.clone(),
                r.description.clone(),
                DocKind::Rule,
                None,
                Vec::new(),
                &text,
            );
        }

        engine.n = engine.docs.len();
        engine.avgdl = if engine.n > 0 {
            total_len as f64 / engine.n as f64
        } else {
            1.0
        };

        for (term, postings) in &engine.inverted {
            let df = postings.len() as f64;
            let idf = ((engine.n as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
            engine.idf.insert(term.clone(), idf);
        }

        engine
    }

    /// Добавляет документ в индекс. Возвращает количество токенов.
    #[allow(clippy::too_many_arguments)]
    fn push_doc(
        &mut self,
        id: String,
        name: String,
        description: String,
        kind: DocKind,
        category: Option<String>,
        tags: Vec<String>,
        text: &str,
    ) -> usize {
        let tokens = tokenize(text);
        let len = tokens.len();
        let doc_idx = self.docs.len();
        self.docs.push(Bm25Doc {
            id,
            name,
            description,
            kind,
            category,
            tags,
            len,
        });

        for t in tokens {
            let postings = self.inverted.entry(t).or_default();
            match postings.last_mut() {
                Some((idx, freq)) if *idx == doc_idx => *freq += 1,
                _ => postings.push((doc_idx, 1)),
            }
        }
        len
    }

    /// Ищет по запросу. Если kind_filter задан — только документы указанных типов.
    pub fn search(
        &self,
        query: &str,
        kind_filter: Option<&[DocKind]>,
        tags_filter: Option<&[String]>,
        limit: usize,
    ) -> Vec<SearchHit> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() || self.n == 0 {
            return Vec::new();
        }

        let mut scores: Vec<f64> = vec![0.0; self.n];

        for qt in &query_tokens {
            // Точное совпадение термина в индексе.
            let exact: Vec<ScoringSource<'_>> = self
                .idf
                .get(qt)
                .and_then(|idf| self.inverted.get(qt).map(|p| (*idf, p)))
                .into_iter()
                .map(|(idf, postings)| (idf, postings, 1.0))
                .collect();
            // Префиксный матчинг для русской морфологии: query "форма" должен найти
            // документы с токенами "форму"/"формы"/"форма" — общий префикс >= 3.
            // Вес тем выше, чем длиннее общий префикс.
            let prefix: Vec<ScoringSource<'_>> = if qt.len() >= 3 {
                self.inverted
                    .iter()
                    .filter(|(term, _)| *term != qt)
                    .filter_map(|(term, postings)| {
                        let common = qt.chars().zip(term.chars()).take_while(|(a, b)| a == b).count();
                        if common >= 3 {
                            let boost = 0.4 + 0.15 * (common as f64).min(6.0);
                            self.idf.get(term).map(|idf| (*idf, postings, boost))
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                Vec::new()
            };

            for (idf, postings, boost) in exact.into_iter().chain(prefix) {
                for &(doc_idx, tf) in postings {
                    let doc = &self.docs[doc_idx];
                    if let Some(filter) = kind_filter {
                        if !filter.contains(&doc.kind) {
                            continue;
                        }
                    }
                    if let Some(tfilters) = tags_filter {
                        let matched = doc.kind == DocKind::Skill
                            && tfilters
                                .iter()
                                .any(|t| doc.tags.iter().any(|tag| tag.eq_ignore_ascii_case(t)));
                        if !matched {
                            continue;
                        }
                    }
                    let tf_norm = tf as f64 * (K1 + 1.0)
                        / (tf as f64 + K1 * (1.0 - B + B * doc.len as f64 / self.avgdl));
                    scores[doc_idx] += idf * tf_norm * boost;
                }
            }
        }

        let mut ranked: Vec<(usize, f64)> = scores
            .iter()
            .enumerate()
            .filter(|(_, s)| **s > 0.0)
            .map(|(i, s)| (i, *s))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked.truncate(limit);

        ranked
            .into_iter()
            .map(|(doc_idx, score)| {
                let doc = &self.docs[doc_idx];
                SearchHit {
                    id: doc.id.clone(),
                    name: doc.name.clone(),
                    description: doc.description.clone(),
                    kind: doc.kind.clone(),
                    category: doc.category.clone(),
                    tags: doc.tags.clone(),
                    score,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skill(id: &str, name: &str, desc: &str) -> SkillInfo {
        SkillInfo {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            category: Some("1c-skills".to_string()),
            argument_hint: None,
            allowed_tools: Vec::new(),
            tags: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    fn skill_with_tags(id: &str, name: &str, desc: &str, tags: &[&str]) -> SkillInfo {
        let mut s = skill(id, name, desc);
        s.tags = tags.iter().map(|t| t.to_string()).collect();
        s
    }

    #[test]
    fn tokenizes_russian_and_english() {
        let t = tokenize("Создать обработку 1C-epf-build / XML базы");
        assert!(t.contains(&"создать".to_string()));
        assert!(t.contains(&"обработку".to_string()));
        assert!(t.contains(&"epf".to_string()));
        assert!(t.contains(&"build".to_string()));
        assert!(t.contains(&"xml".to_string()));
        // короткие токены отброшены
        assert!(!t.iter().any(|x| x.len() < 2));
    }

    #[test]
    fn ranks_best_match_first() {
        let skills = vec![
            skill("a", "1c-form-add", "Добавить управляемую форму к объекту конфигурации"),
            skill("b", "1c-epf-build", "Собрать внешнюю обработку 1С (EPF/ERF) из XML-исходников"),
            skill("c", "1c-epf-init", "Создать пустую внешнюю обработку 1С (scaffold XML-исходников)"),
        ];
        let bm = Bm25::build(&skills, &[], &[]);
        let hits = bm.search("собрать обработку", Some(&[DocKind::Skill]), None, 10);
        assert!(!hits.is_empty());
        // 1c-epf-build (точное совпадение "собрать") выше 1c-epf-init (только "обработку")
        let top = hits.iter().find(|h| h.id == "b").unwrap();
        let init = hits.iter().find(|h| h.id == "c").unwrap();
        assert!(top.score > init.score);
    }

    #[test]
    fn multiword_query_with_punctuation() {
        let skills = vec![
            skill("a", "1c-db-dump-xml", "Выгрузка конфигурации в XML"),
            skill("b", "1c-mxl-compile", "Компилятор макета табличного документа"),
        ];
        let bm = Bm25::build(&skills, &[], &[]);
        // пробелы вместо дефисов и порядок слов неважен
        let hits = bm.search("dump xml конфигурации", Some(&[DocKind::Skill]), None, 10);
        assert_eq!(hits.first().map(|h| h.id.as_str()), Some("a"));
    }

    #[test]
    fn kind_filter_limits_results() {
        let skills = vec![skill("a", "1c-form-add", "форма документа")];
        let bm = Bm25::build(&skills, &[], &[]);
        assert!(bm.search("форма", Some(&[DocKind::Rule]), None, 10).is_empty());
        assert!(!bm.search("форма", Some(&[DocKind::Skill]), None, 10).is_empty());
        // без фильтра — по всем типам
        assert!(!bm.search("форма", None, None, 10).is_empty());
    }

    #[test]
    fn tags_filter_limits_results() {
        let skills = vec![
            skill_with_tags("a", "1c-epf-build", "собрать обработку из xml", &["epf"]),
            skill_with_tags("b", "1c-form-add", "добавить форму к объекту", &["forms"]),
        ];
        let bm = Bm25::build(&skills, &[], &[]);
        // только тег epf
        let epf = bm.search(
            "обработку форму",
            Some(&[DocKind::Skill]),
            Some(&["epf".to_string()]),
            10,
        );
        assert!(epf.iter().all(|h| h.tags.iter().any(|t| t == "epf")));
        assert_eq!(epf.first().map(|h| h.id.as_str()), Some("a"));
        // фильтр по тегу forms
        let forms = bm.search(
            "обработку форму",
            Some(&[DocKind::Skill]),
            Some(&["forms".to_string()]),
            10,
        );
        assert_eq!(forms.first().map(|h| h.id.as_str()), Some("b"));
    }
}
