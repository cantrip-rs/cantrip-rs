use cantrip_api::*;
use cantrip_mechanics::{effects, triggers, typecycling};
use cantrip_oracle as oracle;

/// "This creature gains flying until end of turn and you gain 1 life." (CR 611.2a, 119.3, 608.2c): one sentence, so one instruction.
fn gain_flying_and_one_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Flying);
        g.gain_life(item.controller, 1);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::LANDFALL,
            gain_flying_and_one_life,
            "Landfall \u{2014} Whenever a land you control enters, this creature gains flying until end of turn and you gain 1 life.",
        ),
        typecycling::ability(Subtype::Plains, mana("{2}"), "Plainscycling {2} ({2}, Discard this card: Search your library for a Plains card, reveal it, put it into your hand, then shuffle.)"),
    ],
    ..oracle::RABAROO_TROOP
};
