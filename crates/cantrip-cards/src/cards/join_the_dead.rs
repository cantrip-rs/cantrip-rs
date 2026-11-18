use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -5/-5 until end of turn. Descend 4 - That creature gets -10/-10 until end of turn instead if there are four
/// or more permanent cards in your graveyard" (CR 207.2c, 700.11, 614.1a, 611.2a): the graveyard is counted once, as the spell
/// resolves (the spell is not a permanent card, CR 110.4a), and one effect or the other is applied, never both.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent_cards_in_graveyard(g, item.controller) >= 4 {
        effects::target_creature_gets::<-10, -10>(g, item)
    } else {
        effects::target_creature_gets::<-5, -5>(g, item)
    }
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::JOIN_THE_DEAD };
