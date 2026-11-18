use cantrip_api::*;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_mechanics::{effects, pump, targets, total_toughness_of_creatures_you_control};
use cantrip_oracle as oracle;

/// "This spell costs {X} less to cast, where X is the total toughness of creatures you control" (CR 601.2f, 118.7a): the
/// creatures its caster controls as the total cost is determined (the spell is on the stack, not among them), a total below zero
/// counting as nothing (CR 107.1b), and only the generic part of the cost reduced. Only casting the spell is reduced.
fn less_by_total_toughness(q: &dyn Query, _: ObjectId, player: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::less(total_toughness_of_creatures_you_control(q, player).max(0) as u32)
}

/// "Draw cards equal to its toughness": its toughness as the ability resolves, counters and effects included, from last-known
/// information if it has left the battlefield (CR 608.2h); none for a toughness of 0 or less (CR 107.1b).
fn draw_cards_equal_to_its_toughness(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let toughness = if targets::permanent(g, item.object) {
        g.chars(item.object).toughness
    } else {
        g.state().last_known(item.object).and_then(|known| known.toughness)
    };
    let Some(toughness) = toughness else { return Ok(()) };
    effects::draw(g, item, toughness.max(0) as u32)
}

/// "Until end of turn, target creature you control gets +1/+0, gains 'Whenever this creature deals combat damage to a player, draw
/// cards equal to its toughness,' and can attack as though it didn't have defender" (CR 611.2a, 611.2c, 613.1f, 702.3b): all three
/// fixed to the creature as the ability resolves, as one instruction (CR 608.2c); nothing if the target is no longer legal.
fn pump_and_grant(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let this = g.card_id(&CARD);
    g.one_instruction(&mut |g| {
        pump(g, item.object, creature, 1, 0);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        g.add_effect(item.object, creature, EffectKind::CanAttackDespiteDefender, Duration::EndOfTurn);
        Ok(())
    })
}

/// Defender is a core keyword.
pub static CARD: CardDef = CardDef {
    own_cost_changes: &[less_by_total_toughness],
    abilities: &[ability(
        cost("{2}{U}{U}"),
        pump_and_grant,
        "{2}{U}{U}: Until end of turn, target creature you control gets +1/+0, gains \"Whenever this creature deals combat damage to a player, draw cards equal to its toughness,\" and can attack as though it didn't have defender.",
    )
    .targets(&[targets::CREATURE_YOU_CONTROL])],
    delayed: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER,
        draw_cards_equal_to_its_toughness,
        "Whenever this creature deals combat damage to a player, draw cards equal to its toughness.",
    )],
    ..oracle::THE_PRIDE_OF_HULL_CLADE
};
