use cantrip_api::*;
use cantrip_mechanics::behold;
use cantrip_mechanics::tokens::{self, MERFOLK_WHITE_BLUE};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn create_merfolk(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &MERFOLK_WHITE_BLUE, 1)?;
    Ok(())
}

/// "As an additional cost to cast this spell, behold a Merfolk or pay {2}" (CR 601.2b, 701.4a): one of the two, chosen as the spell is cast.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, create_merfolk, "When this creature enters, create a 1/1 white and blue Merfolk creature token.")],
    additional_choice: &[
        ExtraCost { text: "Behold a Merfolk", cost: behold::cost(behold::merfolk, "Merfolk") },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    ..oracle::SILVERGILL_MENTOR
};

pub static EXTRA: &[&CardDef] = &[&MERFOLK_WHITE_BLUE];
