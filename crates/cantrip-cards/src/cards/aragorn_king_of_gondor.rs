use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ENTERS, triggers};
use cantrip_oracle as oracle;

/// Up to one target creature can't block this turn. If you're the monarch, creatures can't block this turn.
fn cant_block(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::first_target_cant_block_this_turn(g, item)?;
    g.end_sentence();
    if g.state().player(item.controller).monarch {
        g.creatures_cant_block();
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, cantrip_mechanics::effects::become_monarch, "When Aragorn enters, you become the monarch."),
        triggered(triggers::THIS_ATTACKS, cant_block, "Whenever Aragorn attacks, up to one target creature can't block this turn. If you're the monarch, creatures can't block this turn.")
            .targets(&[targets::CREATURE.optional()]),
    ],
    ..oracle::ARAGORN_KING_OF_GONDOR
};
