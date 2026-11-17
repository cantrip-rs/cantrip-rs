use cantrip_api::*;
use cantrip_mechanics::history;
use cantrip_oracle as oracle;

/// "A creature spell with power 2 or less" (CR 601.3): the card on top of the library as the spell it would be, its power
/// read through `q.chars` (CR 604.3: a power-defining ability works in the library). A creature card with no power has none to
/// compare, and is no creature spell this reaches.
fn creature_with_power_two_or_less(q: &dyn Query, _: &Asker, card: ObjectId) -> bool {
    let chars = q.chars(card);
    chars.is(CardType::Creature) && chars.power.is_some_and(|power| power <= 2)
}

/// "Once each turn" (CR 601.3): no cast made by this permanent's rule has happened this turn, asked of the history
/// (`Cast::granted_by`), on any player's turn; there is no marker of the card's own.
fn not_yet_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    !history::permission_used_this_turn(q, asker.source)
}

/// The rule "you may cast a creature spell with power 2 or less from the top of your library", while it has not been used
/// this turn.
static TOP_CREATURE: &[PlayRule] = &[PlayRule::LibraryTopSpells(creature_with_power_two_or_less, None)];

pub static CARD: CardDef = CardDef {
    // "You may look at the top card of your library any time" (CR 401.2), and "once each turn, you may cast a creature spell
    // with power 2 or less from the top of your library" (CR 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::While(not_yet_this_turn, TOP_CREATURE)],
    ..oracle::ASSEMBLE_THE_PLAYERS
};
