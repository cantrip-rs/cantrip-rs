use cantrip_api::*;
use cantrip_mechanics::{kicker, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -2/-2 until end of turn. If this spell was kicked, that creature gets -5/-5 until end of turn
/// instead." One effect, so a kicked spell gives -5/-5 only (CR 608.2c), never -2/-2 as well. "Kicker - Sacrifice an artifact
/// or creature" is a kicker (CR 702.33a) whose cost is not mana, so it is announced before the target (CR 601.2b) and paying it
/// kicks the spell once (CR 702.33b): `kicked`, as for any kicker.
fn stomp(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let amount = if item.kicked > 0 { -5 } else { -2 };
    pump(g, item.object, creature, amount, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], stomp),
    optional_cost: Some(kicker::non_mana(sacrifice(1, targets::filters::artifact_or_creature))),
    labels: &["Cast without kicking", "Kicker\u{2014}Sacrifice an artifact or creature"],
    ..oracle::STOMPED_BY_THE_FOOT
};
