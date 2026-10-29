use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{targets, typecycling};
use cantrip_oracle as oracle;

/// "Target creature can't block this turn" (CR 509.1a): a continuous effect from the resolution of the ability.
fn cant_block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::CantBlock, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, cant_block, "When Zog enters, target creature can't block this turn.").targets(&[targets::CREATURE]),
        typecycling::ability(Subtype::Mountain, mana("{2}"), "Mountaincycling {2} ({2}, Discard this card: Search your library for a Mountain card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::ZOG_TRICERATON_CASTAWAY
};
