use cantrip_api::*;
use cantrip_mechanics::history::{permanents_sacrificed_this_turn, you_sacrificed_a_permanent_this_turn};
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{damage_target, effects, targets};
use cantrip_oracle as oracle;

/// "This creature deals that much damage to any target" (CR 120.3, 608.2h): as many as the permanents its controller has sacrificed
/// this turn as the ability resolves (one sacrificed in response counts), dealt by the creature as it last existed if it has left.
/// A target no longer legal is dealt nothing (CR 608.2b). Menace is a core keyword.
fn damage_for_each_permanent_sacrificed(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let sacrificed = permanents_sacrificed_this_turn(g, item.controller) as u32;
    damage_target(g, item, 0, sacrificed);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            cost("{2}").and(sacrifice(1, targets::filters::another_creature_or_enchantment)),
            effects::plus_one_counter_on_this,
            "{2}, Sacrifice another creature or enchantment: Put a +1/+1 counter on this creature.",
        ),
        triggered(
            YOUR_END_STEP,
            damage_for_each_permanent_sacrificed,
            "At the beginning of your end step, if you sacrificed one or more permanents this turn, this creature deals that much damage to any target.",
        )
        .targets(&[targets::ANY])
        .condition(you_sacrificed_a_permanent_this_turn),
    ],
    ..oracle::SAWBLADE_SKINRIPPER
};
