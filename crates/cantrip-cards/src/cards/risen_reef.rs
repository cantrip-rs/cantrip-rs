use cantrip_api::*;
use cantrip_mechanics::{library, triggers};
use cantrip_oracle as oracle;

/// "Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand." (CR 608.2, 400.7).
fn look_top_card_library_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_land_onto_battlefield_else_hand(g, item, true)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::ZONE_CHANGE,
                triggers::this_or_another_of_subtype_you_control_enters::<
                    { Subtype::Elemental as u16 },
                >,
            ),
            look_top_card_library_if,
            "Whenever this creature or another Elemental you control enters, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped. If you don't put the card onto the battlefield, put it into your hand.",
        ),
    ],
    ..oracle::RISEN_REEF
};
