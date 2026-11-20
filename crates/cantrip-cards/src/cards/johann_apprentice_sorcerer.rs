use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "An instant or sorcery spell" (CR 601.3): the card on top of the library as the spell it would be, each face judged on its own
/// (CR 715.3a), so the Adventure of an Adventurer on top is one and its creature face is not.
fn instant_or_sorcery_spell(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    let chars = q.chars(card);
    chars.is(CardType::Instant) || chars.is(CardType::Sorcery)
}

/// "Once each turn" (CR 601.3): no cast made by this permanent's rule has happened this turn, asked of the history
/// (`Cast::granted_by`), on any player's turn; there is no marker of the card's own.
fn not_yet_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    !history::permission_used_this_turn(q, asker.source)
}

/// The rule "you may cast an instant or sorcery spell from the top of your library", while it has not been used this turn.
static TOP_INSTANT_OR_SORCERY: &[PlayRule] = &[PlayRule::LibraryTopSpells(instant_or_sorcery_spell, None)];

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and "once each turn, you may cast an instant or sorcery
    // spell from the top of your library" (CR 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::While(not_yet_this_turn, TOP_INSTANT_OR_SORCERY)],
    ..oracle::JOHANN_APPRENTICE_SORCERER
};
