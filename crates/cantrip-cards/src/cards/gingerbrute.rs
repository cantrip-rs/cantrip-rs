use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "{1}: This creature can't be blocked this turn except by creatures with haste."
/// The resolved restriction lasts independently of this creature's abilities (CR 611.2a).
fn cant_be_blocked_this_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_effect(item.object, item.object, EffectKind::BlockRestriction { card: item.ability.card, index: 0 }, Duration::EndOfTurn);
    Ok(())
}

/// The resolved restriction applies only to this attacker (CR 509.1b, 611.2a).
fn cant_be_blocked_except_by_haste(q: &dyn Query, source: ObjectId, blocker: ObjectId, attacker: ObjectId) -> bool {
    source != attacker || q.chars(blocker).has(Keyword::Haste)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), cant_be_blocked_this_turn, "{1}: This creature can't be blocked this turn except by creatures with haste."),
        ability(
            cost("{2}, {T}").and(SACRIFICE_THIS),
            |g, item| effects::gain_life(g, item, 3),
            "{2}, {T}, Sacrifice this creature: You gain 3 life.",
        ),
    ],
    resolved_block_rules: &[cant_be_blocked_except_by_haste],
    ..oracle::GINGERBRUTE
};
