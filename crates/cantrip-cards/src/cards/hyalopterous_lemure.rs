use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{effects, pump};

/// "{0}: This creature gets -1/-0 and gains flying until end of turn." (CR 608.2, 611.2a).
fn creature_gets_1_0_gains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, -1, 0);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Flying);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            Cost::mana(ManaCost::ZERO),
            creature_gets_1_0_gains,
            "{0}: This creature gets -1/-0 and gains flying until end of turn.",
        ),
    ],
    ..oracle::HYALOPTEROUS_LEMURE
};
