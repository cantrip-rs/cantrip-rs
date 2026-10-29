use cantrip_api::*;
use cantrip_mechanics::{kicker, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -2/-2 until end of turn. If this spell was kicked, that creature gets -6/-6 until end of turn
/// instead." The choice of size is made from whether this spell was kicked (CR 702.33d); each is a continuous effect until
/// end of turn (CR 611.2a), and an illegal target is not affected (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let amount = if item.kicked > 0 { -6 } else { -2 };
    pump(g, item.object, creature, amount, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    // "Kicker—Sacrifice an artifact or creature." (CR 702.33a, 601.2b, 601.2h, 701.21): a kicker whose cost is not mana,
    // announced before the target and paid with the other costs; paying it kicks the spell once (CR 702.33d).
    optional_cost: Some(kicker::non_mana(sacrifice(1, targets::filters::artifact_or_creature))),
    labels: &["Cast without kicking", "Kicker\u{2014}Sacrifice an artifact or creature"],
    ..oracle::EJECT_THE_WARP_CORE
};
