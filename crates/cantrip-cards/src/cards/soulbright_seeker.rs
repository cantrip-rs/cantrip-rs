use cantrip_api::*;
use cantrip_mechanics::{behold, effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gains trample until end of turn. If this is the third time this ability has resolved this turn,
/// add {R}{R}{R}{R}" (CR 608.2m, 611.2a, 106.4): this turn's history counts the resolutions of this Seeker's ability before this one;
/// two before make it the third. An activation that fizzles for want of a legal target does not resolve, so is not counted.
fn trample_and_maybe_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Trample);
    g.end_sentence();
    if g.history().resolutions(item.object, item.ability) == 2 {
        g.add_mana(item.controller, Mana::R, 4);
    }
    Ok(())
}

/// "As an additional cost to cast this spell, behold an Elemental or pay {2}" (CR 601.2b, 701.4a): one of the two, chosen before
/// any target.
pub static CARD: CardDef = CardDef {
    additional_choice: &[
        ExtraCost { text: "Behold an Elemental", cost: behold::cost(behold::elemental, "Elemental") },
        ExtraCost { text: "Pay {2}", cost: Cost { mana: mana("{2}"), ..Cost::NONE } },
    ],
    abilities: &[ability(cost("{R}"), trample_and_maybe_mana, "{R}: Target creature you control gains trample until end of turn. If this is the third time this ability has resolved this turn, add {R}{R}{R}{R}.")
        .targets(&[targets::CREATURE_YOU_CONTROL])],
    ..oracle::SOULBRIGHT_SEEKER
};
