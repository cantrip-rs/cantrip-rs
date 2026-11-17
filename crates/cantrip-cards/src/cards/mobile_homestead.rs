use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::statics::{have_haste, itself_while_you_control_a};
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{lands, library};
use cantrip_oracle as oracle;

/// "Look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped." (CR 701.20a, 305.4):
/// a card that is not put onto the battlefield stays on top of the library.
fn look_at_the_top_card_and_may_put_a_land_onto_the_battlefield_tapped(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_may_put_land_onto_battlefield(g, item, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ATTACKS,
            look_at_the_top_card_and_may_put_a_land_onto_the_battlefield_tapped,
            "Whenever this Vehicle attacks, look at the top card of your library. If it's a land card, you may put it onto the battlefield tapped.",
        ),
        crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_control_a::<{ lands::sub(Subtype::Mount) }>,
        have_haste,
        "This Vehicle has haste as long as you control a Mount.",
    )],
    ..oracle::MOBILE_HOMESTEAD
};
