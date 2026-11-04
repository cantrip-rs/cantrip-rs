use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "Whenever you activate a loyalty ability of a Chandra planeswalker" (CR 606.2, 603.2): its controller's activation of a
/// loyalty ability of a permanent that is a Chandra planeswalker (CR 205.3j) as the costs were paid, as it last was if paying
/// them took it away.
fn you_activate_chandra_loyalty_ability(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let chars = q.chars(triggers::loyalty_ability_you_activated(q, me, event)?);
    if chars.is(CardType::Planeswalker) && chars.has_subtype(Subtype::Chandra) { Fired::PLAIN } else { None }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ABILITY_ACTIVATED, you_activate_chandra_loyalty_ability),
        cantrip_mechanics::effects::deals_1_damage_to_each_opponent,
        "Whenever you activate a loyalty ability of a Chandra planeswalker, this creature deals 1 damage to each opponent.",
    )],
    ..oracle::KERAL_KEEP_DISCIPLES
};
