use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// The two damage instructions are sequential, with each source's power read as it happens (CR 608.2h, 120.3).
fn exchange_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let power = g.chars(item.object).power.unwrap_or(0).max(0) as u32;
    g.damage(item.object, Target::Object(target), power);
    g.end_sentence();
    if let Some(power) = g.chars(target).power {
        g.damage(target, Target::Object(item.object),(power.max(0)) as u32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, exchange_damage, "{T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.")
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::KARPLUSAN_YETI
};
