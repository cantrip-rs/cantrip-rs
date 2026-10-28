use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Iron Fist gains '{T}: ...' until end of turn" (CR 611.2a, 613.1f): a layer 6 effect that gives him the card's delayed ability 0
/// until the cleanup step. It is an ability of his own, so its {T} is subject to summoning sickness (CR 302.6); cast again, the
/// trigger gives him another copy of it.
fn gain_the_ability(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let this = g.card_id(&CARD);
    g.add_effect(item.object, item.object, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    Ok(())
}

/// "Iron Fist deals damage equal to his power to any other target" (CR 120.3): his power as the ability resolves, or as he last
/// existed if he has left the battlefield (CR 113.7a); no damage for a power of 0 or less (CR 120.8).
fn deal_damage_equal_to_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.chars(item.object).power.unwrap_or(0).max(0) as u16;
    g.damage(item.object, item.target(0), power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_CAST_A_SPELL_THAT_TARGETS_A_CREATURE_YOU_CONTROL,
        gain_the_ability,
        "Whenever you cast a spell that targets a creature you control, Iron Fist gains \"{T}: Iron Fist deals damage equal to his power to any other target\" until end of turn.",
    )],
    // The granted ability (the card's delayed ability 0).
    delayed: &[ability(TAP, deal_damage_equal_to_power, "{T}: Iron Fist deals damage equal to his power to any other target.")
        .targets(&[targets::ANY_OTHER_TARGET])],
    ..oracle::IRON_FIST_LIVING_WEAPON
};
