use cantrip_api::*;
use cantrip_mechanics::{library, targets, triggers::THIS_ENTERS_IF_CAST};
use cantrip_oracle as oracle;

/// "Return target creature card with mana value 3 or less from your graveyard to the battlefield": it enters under the
/// ability's controller's control (CR 110.2a). A target gone by resolution is left alone (CR 608.2b).
fn return_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield(card, item.controller, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS_IF_CAST,
        library::mill_then_reflexive_trigger::<4>,
        "When this creature enters, if you cast it, mill four cards. When you do, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
    )],
    delayed: &[delayed_trigger(
        return_creature,
        "When you do, return target creature card with mana value 3 or less from your graveyard to the battlefield.",
    )
    .targets(&[targets::CREATURE_CARD_WITH_MANA_VALUE_3_OR_LESS_IN_YOUR_GRAVEYARD])],
    ..oracle::YATHAN_ROADWATCHER
};
