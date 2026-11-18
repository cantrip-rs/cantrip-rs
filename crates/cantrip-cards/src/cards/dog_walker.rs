use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_TURNED_FACE_UP;
use cantrip_mechanics::{disguise, tokens};
use cantrip_oracle as oracle;

/// "Create two tapped 1/1 white Dog creature tokens." (CR 701.7a, 111.2): both made together, as one event (CR 101.4), tapped as they enter.
fn create_two_tapped_dogs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let dog = g.card_id(&tokens::DOG);
    g.create_tokens(dog, item.controller, 2, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{R/W}{R/W}")), "Disguise {R/W}{R/W}")],
    abilities: &[triggered(
        THIS_TURNED_FACE_UP,
        create_two_tapped_dogs,
        "When this creature is turned face up, create two tapped 1/1 white Dog creature tokens.",
    )],
    ..oracle::DOG_WALKER
};

pub static EXTRA: &[&CardDef] = &[&tokens::DOG];
