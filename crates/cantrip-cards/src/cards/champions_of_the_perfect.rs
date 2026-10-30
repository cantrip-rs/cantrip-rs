use cantrip_api::*;
use cantrip_mechanics::triggers::{CREATURE_SPELL_YOU_CAST, THIS_LEAVES};
use cantrip_mechanics::{behold, effects};
use cantrip_oracle as oracle;

/// "Whenever you cast a creature spell, draw a card."
fn draw_a_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::draw(g, item, 1)
}

/// "As an additional cost to cast this spell, behold an Elf and exile it" (CR 601.2b, 701.4a), exiled with the creature the spell
/// becomes (CR 607.2q), which returns it when it leaves the battlefield. A spell that is countered leaves it exiled.
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::elf, "Elf")),
    abilities: &[
        triggered(CREATURE_SPELL_YOU_CAST, draw_a_card, "Whenever you cast a creature spell, draw a card."),
        triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::CHAMPIONS_OF_THE_PERFECT
};
