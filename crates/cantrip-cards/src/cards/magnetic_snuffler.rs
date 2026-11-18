use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

fn equipment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::subtype_card_in_your_graveyard(q, asker, t, &[Subtype::Equipment])
}

/// "Return target Equipment card from your graveyard to the battlefield attached to this creature" (CR 301.5e, 400.7): an
/// Equipment that cannot be attached to it - the creature has left the battlefield, or has protection from it - enters
/// unattached. Nothing happens if the card is no longer a legal target (CR 608.2b).
fn return_equipment_attached(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_onto_battlefield_attached(card, item.controller, item.object)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            return_equipment_attached,
            "When this creature enters, return target Equipment card from your graveyard to the battlefield attached to this creature.",
        )
        .targets(&[target(equipment_card_in_your_graveyard)]),
        triggered(
            triggers::YOU_SACRIFICE_AN_ARTIFACT,
            effects::plus_one_counter_on_this,
            "Whenever you sacrifice an artifact, put a +1/+1 counter on this creature.",
        )
        .on_sacrifice(),
    ],
    ..oracle::MAGNETIC_SNUFFLER
};
