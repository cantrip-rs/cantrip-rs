use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "You gain 2 life. Surveil 1." (CR 119.3, 701.25.) The card exiled itself as a cost, so the surveil looks at the library
/// of the ability's controller.
fn gain_and_surveil(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::gain_life(g, item, 2)?;
    g.end_sentence();
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{W}").and(EXILE_THIS), gain_and_surveil, "{W}, Exile this card from your graveyard: You gain 2 life. Surveil 1. Activate only as a sorcery.")
        .from(ZoneKind::Graveyard)
        .sorcery_speed()],
    ..oracle::STONE_DOCENT
};
