//! The same rules-text renderer used by engine choice labels and scenarios.
pub use cantrip_engine::text::changed_text;
pub(crate) use cantrip_engine::text::plural;

/// The kind of a chosen subtype, for prompts and permanent views: a land type, or else a creature type (the two a card
/// chooses; every other choice of a subtype is of a creature type).
pub(crate) fn subtype_kind(sub: cantrip_engine::Subtype) -> &'static str {
    if sub.card_types().has(cantrip_engine::CardType::Land) { "land type" } else { "creature type" }
}
