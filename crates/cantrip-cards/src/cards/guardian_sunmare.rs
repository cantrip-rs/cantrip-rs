use cantrip_api::*;
use cantrip_mechanics::saddle;
use cantrip_mechanics::triggers::{ward, THIS_ATTACKS_WHILE_SADDLED};
use cantrip_mechanics::{library, select};
use cantrip_oracle as oracle;

const NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS: select::Selection =
    select::Selection::of(select::up_to(1), &[select::NONLAND, select::PERMANENT_CARD, select::mana_value_at_most(3)]).called("nonland permanent");

/// "Whenever this creature attacks while saddled, search your library for a nonland permanent card with mana value 3 or less, put
/// it onto the battlefield, then shuffle." (CR 701.23, 110.4a, 202.3): each card's mana value is that of its printed cost, in the library as anywhere (CR 202.3),
/// and the search may find nothing (CR 701.23b).
fn put_a_cheap_permanent_onto_the_battlefield(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::find_to_battlefield(g, item.controller, item.object, NONLAND_PERMANENT_CARD_WITH_MANA_VALUE_3_OR_LESS, false)?;
    Ok(())
}

/// "Ward {2}" (CR 702.21a) and "Saddle 4" (CR 702.171a) are the shared mechanics.
pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            THIS_ATTACKS_WHILE_SADDLED,
            put_a_cheap_permanent_onto_the_battlefield,
            "Whenever this creature attacks while saddled, search your library for a nonland permanent card with mana value 3 or less, put it onto the battlefield, then shuffle.",
        ),
        saddle::ability(4, "Saddle 4 (Tap any number of other creatures you control with total power 4 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)"),
    ],
    ..oracle::GUARDIAN_SUNMARE
};
