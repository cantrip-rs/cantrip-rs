//! The card index (docs/ai-player-plan.md, "Inputs"): the row each card name has in an encoded position. The encoding
//! (`cantrip_obs::encode`) puts a card's row on its token; row 0 is the unknown card. The runtime's index is the
//! engine's own cards (`CardIndex::engine`), so every card the engine has has a row, and a shard names the index it was
//! recorded with (`shard::Header::cards`). A model keeps the index it was trained with (its `cards.json`); the
//! evaluator maps the runtime's rows onto the model's by name (`eval::check`), and the text vectors a row starts from
//! are the model's own, worked out when it is trained (`ai/cantrip_ai/embed.py`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use cantrip_engine::{CardId, CardTable};
use cantrip_obs::encode::{Cards, UNKNOWN_CARD, fnv1a64};

pub struct CardIndex {
    /// FNV-1a 64 over the names in row order, joined by newlines, as 16 hex digits: what tells two indexes apart.
    pub hash: String,
    /// `names[i]` is row `i + 1`.
    pub names: Vec<String>,
    rows: HashMap<String, u32>,
}

impl CardIndex {
    /// The index of these names, in this order.
    pub fn from_names(names: Vec<String>) -> CardIndex {
        let hash = format!("{:016x}", fnv1a64(names.join("\n").as_bytes()));
        let rows = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i as u32 + 1))
            .collect();
        CardIndex { hash, names, rows }
    }

    /// Every name `cards` has, sorted: a row for each card and face the engine can show.
    pub fn of(cards: &CardTable) -> CardIndex {
        let mut names: Vec<String> = cards
            .cards
            .iter()
            .map(|c| c.name)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .collect();
        names.sort_unstable();
        names.dedup();
        CardIndex::from_names(names)
    }

    /// The engine's cards' index, made once.
    pub fn engine() -> &'static CardIndex {
        static INDEX: OnceLock<CardIndex> = OnceLock::new();
        INDEX.get_or_init(|| CardIndex::of(cantrip_engine::cards()))
    }

    /// A model's index: its `cards.json` (`{"names": [...]}`, and the names' `hash` where it says one, which must be
    /// theirs).
    pub fn read(path: &Path) -> Result<CardIndex, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let v: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let names: Vec<String> = v["names"]
            .as_array()
            .ok_or_else(|| format!("{}: no names", path.display()))?
            .iter()
            .map(|n| {
                n.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{}: a name that is not a string", path.display()))
            })
            .collect::<Result<_, _>>()?;
        let index = CardIndex::from_names(names);
        match v["hash"].as_str() {
            Some(hash) if hash != index.hash => Err(format!(
                "{}: the hash {hash} is not its names' ({})",
                path.display(),
                index.hash
            )),
            _ => Ok(index),
        }
    }

    /// A name's row, `UNKNOWN_CARD` for a name the index lacks.
    pub fn row(&self, name: &str) -> u32 {
        self.rows.get(name).copied().unwrap_or(UNKNOWN_CARD)
    }

    /// Rows, the unknown card's included.
    pub fn len(&self) -> usize {
        self.names.len() + 1
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The engine's cards onto rows, by `CardId`.
    pub fn card_rows(&self, cards: &CardTable) -> CardRows {
        CardRows(cards.cards.iter().map(|c| self.row(c.name)).collect())
    }
}

impl Cards for CardIndex {
    fn row(&self, name: &str) -> u32 {
        CardIndex::row(self, name)
    }
}

/// Each engine card's row in a `CardIndex`.
pub struct CardRows(pub Vec<u32>);

impl CardRows {
    pub fn row(&self, card: CardId) -> u32 {
        self.0.get(card.0 as usize).copied().unwrap_or(UNKNOWN_CARD)
    }

    /// The engine's cards that have no row: their names.
    pub fn missing<'a>(&self, cards: &'a CardTable) -> Vec<&'a str> {
        self.0
            .iter()
            .enumerate()
            .filter(|(_, r)| **r == UNKNOWN_CARD)
            .map(|(i, _)| cards.cards[i].name)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every card the engine has, but a nameless one, has a row of the engine's index.
    #[test]
    fn the_engine_index_covers_the_cards() {
        let cards = cantrip_engine::cards();
        let index = CardIndex::engine();
        let missing: Vec<&str> = index
            .card_rows(cards)
            .missing(cards)
            .into_iter()
            .filter(|n| !n.is_empty())
            .collect();
        assert!(missing.is_empty(), "cards with no row: {missing:?}");
        assert_ne!(index.row("Shock"), UNKNOWN_CARD);
        assert_eq!(index.row("no such card"), UNKNOWN_CARD);
    }

    #[test]
    fn a_model_index_whose_hash_is_not_its_names_is_refused() {
        let dir = std::env::temp_dir().join(format!("cantrip-index-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cards.json");
        let index = CardIndex::from_names(vec!["Shock".into(), "Opt".into()]);
        std::fs::write(
            &path,
            serde_json::json!({ "hash": index.hash, "names": index.names }).to_string(),
        )
        .unwrap();
        assert_eq!(
            CardIndex::read(&path).map(|i| i.hash),
            Ok(index.hash.clone())
        );
        std::fs::write(
            &path,
            serde_json::json!({ "hash": "0000000000000000", "names": index.names }).to_string(),
        )
        .unwrap();
        assert!(CardIndex::read(&path).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
