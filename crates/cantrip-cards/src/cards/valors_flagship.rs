use cantrip_api::*;
use cantrip_mechanics::tokens::{self, PILOT_SADDLES_AND_CREWS};
use cantrip_mechanics::{crew, triggers};
use cantrip_oracle as oracle;

/// "When you cycle this card, create X 1/1 colorless Pilot creature tokens with \"This token saddles Mounts and crews Vehicles as
/// though its power were 2 greater.\"" (CR 107.3a, 702.29d, 111.1): X is the X of the cycling cost paid, `data[0]`
/// (`triggers::YOU_CYCLE_THIS_CARD`); an X of 0 makes none.
fn pilots(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &PILOT_SADDLES_AND_CREWS, item.data[0])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        crew::crew(3, "Crew 3 (Tap any number of creatures you control with total power 3 or more: This Vehicle becomes an artifact creature until end of turn.)"),
        triggers::cycling(mana("{X}{2}{W}"), "Cycling {X}{2}{W} ({X}{2}{W}, Discard this card: Draw a card.)"),
        triggered(
            triggers::YOU_CYCLE_THIS_CARD,
            pilots,
            "When you cycle this card, create X 1/1 colorless Pilot creature tokens with \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\"",
        ),
    ],
    ..oracle::VALORS_FLAGSHIP
};

pub static EXTRA: &[&CardDef] = &[&PILOT_SADDLES_AND_CREWS];
