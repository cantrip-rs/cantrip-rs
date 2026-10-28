use cantrip_api::*;
use cantrip_mechanics::library::{look_at_top, Rest};
use cantrip_mechanics::triggers::{LANDFALL, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "When this creature enters, look at the top four cards of your library. You may reveal a permanent card from among them and
/// put it into your hand. Put the rest on the bottom of your library in a random order." (CR 110.4a, 401.4, 701.20a)
fn look(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_at_top(g, item.controller, item.object, 4, (0, 1), true, targets::is_permanent_card, Rest::BottomRandom)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            look,
            "When this creature enters, look at the top four cards of your library. You may reveal a permanent card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
        triggered(
            LANDFALL,
            effects::this_gets::<2, 2>,
            "Landfall \u{2014} Whenever a land you control enters, this creature gets +2/+2 until end of turn.",
        ),
    ],
    ..oracle::BOUGHSIDE_WANDERERS
};
