use cantrip_api::*;
use cantrip_mechanics::triggers::VALIANT;
use cantrip_mechanics::{effects, offspring};
use cantrip_oracle as oracle;

/// "Mice you control get +1/+0 until end of turn." (CR 611.2c): `effects::subtype_you_control_get` over Mouse, a word of this
/// card's text (CR 612.2).
fn mice_get_plus_one_zero(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::subtype_you_control_get(g, item, Subtype::Mouse, 1, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(
            VALIANT,
            mice_get_plus_one_zero,
            "Valiant \u{2014} Whenever this creature becomes the target of a spell or ability you control for the first time each turn, Mice you control get +1/+0 until end of turn.",
        ),
    ],
    ..oracle::FLOWERFOOT_SWORDMASTER
};
