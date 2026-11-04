use cantrip_api::*;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST_WITH_FOUR_MANA_SPENT;
use cantrip_mechanics::{damage_target, effects, fight, targets};
use cantrip_oracle as oracle;

/// "Blow Up - {T}, Sacrifice this creature: It deals damage equal to its power to target creature." The sacrifice is a cost (CR
/// 601.2h), so it is gone when this resolves and deals damage equal to its power as it last existed on the battlefield (CR 113.7a,
/// 608.2h), counters included; it is the source. A target no longer legal is dealt nothing (CR 608.2b).
fn blow_up(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = fight::damage_equal_to_power(g, item.object);
    damage_target(g, item, 0, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            NONCREATURE_SPELL_YOU_CAST_WITH_FOUR_MANA_SPENT,
            effects::plus_one_counter_on_this,
            "Whenever you cast a noncreature spell, if at least four mana was spent to cast it, put a +1/+1 counter on this creature.",
        ),
        ability(
            TAP.and(SACRIFICE_THIS),
            blow_up,
            "Blow Up \u{2014} {T}, Sacrifice this creature: It deals damage equal to its power to target creature. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
    ],
    ..oracle::BLAZING_BOMB
};
