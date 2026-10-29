use cantrip_api::*;
use cantrip_mechanics::{effects, empower, targets};
use cantrip_oracle as oracle;

/// "Draw a card. Empower Jace 2." The card is drawn first, then empower Jace makes the token if there is none and puts
/// the counters on the Jace token chosen (Fatehold Charm's ruling: never a nontoken Jace).
fn draw_and_empower(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    empower::jace(g, item.controller, item.object, 2)
}

/// "Return target spell or creature to its owner's hand." A spell leaves the stack without being countered (CR 400.3).
fn bounce(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(object) = targets::object(g, item, 0) {
        effects::return_spell_or_permanent_to_hand(g, object);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(draw_and_empower, "Draw a card. Empower Jace 2."),
    mode(bounce, "Return target spell or creature to its owner's hand.").targets(&[targets::SPELL_OR_CREATURE]),
    mode(effects::creatures_you_control_get::<1, 2>, "Creatures you control get +1/+2 until end of turn."),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FATEHOLD_CHARM };

pub static EXTRA: &[&CardDef] = &[&empower::JACE];
