use cantrip_api::*;
use cantrip_mechanics::{effects, offspring};
use cantrip_oracle as oracle;

/// "{T}: Add one mana of any color." (CR 106.1a, 605.1a) is a mana ability; "{T}: Put a +1/+1 counter on this creature." (CR 122.1a)
/// is a plain activated one. The offspring token copy is a 1/1 with both abilities (CR 702.175a).
pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        ability(TAP, effects::plus_one_counter_on_this, "{T}: Put a +1/+1 counter on this creature."),
    ],
    ..oracle::TENDER_WILDGUIDE
};
