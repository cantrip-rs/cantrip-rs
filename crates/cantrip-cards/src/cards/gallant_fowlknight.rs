use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Creatures you control get +1/+0 until end of turn. Kithkin creatures you control also gain first strike until end of turn":
/// the creatures there are as it resolves (CR 611.2c), this one among them; the Kithkin are those of them that are Kithkin then.
fn pump_and_first_strike_for_kithkin(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        let creatures = effects::pump_creatures_you_control(g, item, 1, 0);
        g.end_sentence();
        let kithkin = g.words(item.object).subtype(Subtype::Kithkin);
        for id in creatures {
            if g.chars(id).has_subtype(kithkin) {
                effects::gain_until_end_of_turn(g, item.object, id, Keyword::FirstStrike);
            }
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        pump_and_first_strike_for_kithkin,
        "When this creature enters, creatures you control get +1/+0 until end of turn. Kithkin creatures you control also gain first strike until end of turn.",
    )],
    ..oracle::GALLANT_FOWLKNIGHT
};
