use cantrip_api::*;
use cantrip_mechanics::{counter_unless_pays, discard_chosen, targets};
use cantrip_oracle as oracle;

/// "Counter target spell unless its controller pays {4}." (CR 118.12, 701.6)
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    counter_unless_pays(g, item, 0, mana("{4}"))?;
    Ok(())
}

/// "Draw two cards, then discard a card." (CR 121.1, 701.9): the discard is the caster's choice.
fn loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 2);
    discard_chosen(g, item.controller, item.object, 1)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(counter, "Counter target spell unless its controller pays {4}.").targets(&[targets::SPELL]),
    mode(loot, "Draw two cards, then discard a card."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CONFUSTICATE_AND_BEBOTHER };
