use cantrip_api::*;
use cantrip_mechanics::{effects, lands, permanents_with_subtype_you_control, triggers};
use cantrip_oracle as oracle;

/// "Each opponent loses X life and you gain X life, where X is the number of Shrines you control" (CR 119.3, 608.2h): X is
/// counted once, as the ability resolves, in the layers, the Temple itself among them if it is still there and Shrines the
/// opponents control not at all; the Shrine type is read through this card's words (CR 612).
fn drain_for_each_shrine(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let shrine = g.words(item.object).subtype(Subtype::Shrine);
    let x = i64::try_from(permanents_with_subtype_you_control(g, item.controller, shrine)).expect("shrine count fits the native object domain");
    g.each_opponent_loses_life(item.controller, x);
    g.gain_life(item.controller, x);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            drain_for_each_shrine,
            "When Northern Air Temple enters, each opponent loses X life and you gain X life, where X is the number of Shrines you control.",
        ),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Shrine) }>),
            effects::each_opponent_loses_1_life_you_gain_1,
            "Whenever another Shrine you control enters, each opponent loses 1 life and you gain 1 life.",
        ),
    ],
    ..oracle::NORTHERN_AIR_TEMPLE
};
