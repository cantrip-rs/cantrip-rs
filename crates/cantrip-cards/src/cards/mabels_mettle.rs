use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature gets +2/+2 until end of turn. Up to one other target creature gets +1/+1 until end of turn."
/// A target that is no longer legal as the spell resolves is left alone and the other still gets its bonus (CR 608.2b).
fn pump(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        cantrip_mechanics::pump(g, item.object, creature, 2, 2);
    }
    g.end_sentence();
    let Some(creature) = targets::object(g, item, 1) else { return Ok(()) };
    cantrip_mechanics::pump(g, item.object, creature, 1, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE, targets::CREATURE_OTHER_THAN_THOSE_CHOSEN.between(0, 1)], pump), ..oracle::MABELS_METTLE };
