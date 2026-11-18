use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Return it to the battlefield tapped under its owner's control and you create a Treasure token" (CR 110.2a, 111.10a): "you"
/// is the creature's controller as it died, who gets the Treasure even when the creature was a token and returns nothing.
fn return_it_tapped_and_create_a_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_it_tapped(g, item)?;
    tokens::create_treasure(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effects::target_creature_gets_and_gains_dies_trigger::<2, 0, 0>),
    delayed: &[triggered(
        THIS_DIES,
        return_it_tapped_and_create_a_treasure,
        "When this creature dies, return it to the battlefield tapped under its owner's control and you create a Treasure token.",
    )],
    ..oracle::FAKE_YOUR_OWN_DEATH
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
