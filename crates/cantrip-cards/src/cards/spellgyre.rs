use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Surveil 2, then draw two cards." (CR 701.25a, 121.1): the surveil is finished, cards in the graveyard and the rest on top,
/// before the draw, so a card kept on top is the one drawn.
fn surveil_two_then_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 2)?;
    g.end_sentence();
    effects::draw(g, item, 2)
}

static MODAL: ModalDef = modes(&[
    mode(effects::counter_first_target, "Counter target spell.").targets(&[targets::SPELL]),
    mode(surveil_two_then_draw_two, "Surveil 2, then draw two cards."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::SPELLGYRE };
