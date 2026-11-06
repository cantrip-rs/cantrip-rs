use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

/// Creatures you control get +1/+1 until end of turn, and untap those creatures. The creatures are fixed as the ability
/// resolves (CR 611.2c): "those creatures" is exactly that set.
fn pump_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = effects::pump_creatures_you_control(g, item, 1, 1);
    g.end_sentence();
    g.untap_together(&creatures);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(NONCREATURE_SPELL_YOU_CAST, pump_untap, "Whenever you cast a noncreature spell, creatures you control get +1/+1 until end of turn. Untap those creatures."),
        triggered(NONCREATURE_SPELL_YOU_CAST, effects::may_draw_then_discard, "Whenever you cast a noncreature spell, you may draw a card. If you do, discard a card."),
    ],
    ..oracle::JESKAI_ASCENDANCY
};
