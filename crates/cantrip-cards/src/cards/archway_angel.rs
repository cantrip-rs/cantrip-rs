use cantrip_api::*;
use cantrip_mechanics::{effects, lands, permanents_with_subtype_you_control, triggers};
use cantrip_oracle as oracle;

/// "You gain 2 life for each Gate you control" (CR 119.3, 608.2c): one gain of 2 per Gate its controller controls as the ability
/// resolves, a Gate that is no creature included (CR 109.2), the subtype read through the words of this card's text (CR 612).
fn gain_2_life_for_each_gate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let gate = lands::word(g, item.object, lands::sub(Subtype::Gate));
    let gates = permanents_with_subtype_you_control(g, item.controller, gate) as i32;
    effects::gain_life(g, item, 2 * gates)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        gain_2_life_for_each_gate,
        "When this creature enters, you gain 2 life for each Gate you control.",
    )],
    ..oracle::ARCHWAY_ANGEL
};
