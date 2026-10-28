use cantrip_api::*;
use cantrip_mechanics::{creature_cards_in_graveyard, targets};
use cantrip_oracle as oracle;

/// "This spell costs {2} less to cast if there are two or more creature cards in your graveyard." (CR 601.2f, 118.7a): counted as
/// the total cost is determined; generic mana only.
fn less_with_two_creature_cards(q: &dyn Query, _: ObjectId, controller: PlayerId, _: Costed, _: &[Target]) -> CostDelta {
    if creature_cards_in_graveyard(q, controller) >= 2 { CostDelta::less(2) } else { CostDelta::NONE }
}

/// "Target creature you control deals damage equal to twice its power to target creature an opponent controls." (CR 120.3, 609.7):
/// one-sided, the creature being the source of the damage, so its abilities (lifelink, deathtouch) and colors count; its power is
/// read as the spell resolves (CR 608.2c). If either target is illegal then, no damage is dealt (CR 608.2b); a creature with no
/// power deals none (CR 120.8).
fn deal_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(creature), Some(victim)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        let power = g.chars(creature).power.unwrap_or(0);
        if power > 0 {
            g.damage(creature, Target::Object(victim), 2 * power as u16);
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal damage equal to twice its power"),
            targets::CREATURE_AN_OPPONENT_CONTROLS.labelled("Creature an opponent controls to be dealt that damage"),
        ],
        deal_damage,
    ),
    own_cost_changes: &[less_with_two_creature_cards],
    ..oracle::PUNISHING_PUNCH
};
