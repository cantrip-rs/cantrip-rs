use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::THIS_DIES};
use cantrip_oracle as oracle;

/// When this creature dies, it deals 3 plus damage dealt to it this turn by other sources named Blazing Effigy
/// to target creature (CR 603.10a, 608.2h, 120.7).
fn death_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let amount = g.history().this_turn().iter().filter_map(|record| {
        let Event::Damage { source, target, amount, kind, controller, known, .. } = record.event else { return None };
        let dealt = DamageDealt { source, target, amount, combat: kind.is_combat(), controller, known };
        (source != item.object && target == Target::Object(item.object) && dealt.chars(g).name == "Blazing Effigy")
            .then_some(amount)
    }).fold(3u32, u32::saturating_add);
    g.damage(item.object, Target::Object(target), amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_DIES,
        death_damage,
        "When this creature dies, it deals X damage to target creature, where X is 3 plus the amount of damage dealt to this creature this turn by other sources named Blazing Effigy.",
    ).targets(&[targets::CREATURE])],
    ..oracle::BLAZING_EFFIGY
};
