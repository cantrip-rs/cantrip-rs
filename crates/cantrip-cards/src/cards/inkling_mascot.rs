use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::REPARTEE};
use cantrip_oracle as oracle;

/// "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature gains flying until end of
/// turn. Surveil 1." The gain of flying is a sentence of its own (CR 608.2c); the surveil is made even if this creature has
/// left the battlefield.
fn gain_flying_then_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::this_gains_flying(g, item)?;
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        REPARTEE,
        gain_flying_then_surveil,
        "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature gains flying until end of turn. Surveil 1. (Look at the top card of your library. You may put it into your graveyard.)",
    )],
    ..oracle::INKLING_MASCOT
};
