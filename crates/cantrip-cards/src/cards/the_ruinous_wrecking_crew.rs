use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{destroy_target, effects, targets};
use cantrip_oracle as oracle;

/// "Target token" (CR 115.1, 111.1): a token permanent on the battlefield, under any player's control.
fn token(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::PERMANENT.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

/// Mode 1: "Discard a card, then draw a card." (CR 701.9, 121.1): the controller chooses what to discard, and draws whether or not
/// they discarded.
fn discard_then_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::discard_then_draw(g, item, 1)
}

/// Mode 2: "Target opponent loses 2 life." (CR 119.3), made only if the target is still legal (CR 608.2b).
fn opponent_loses_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Player(player) = item.target(0)
    {
        g.lose_life(player, 2);
    }
    Ok(())
}

/// Mode 3: "Destroy target token." (CR 701.8)
fn destroy_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// "Choose up to X —" (CR 700.2, 603.3c): its modes are chosen as the ability is put on the stack, as many as the X the
/// creature was cast with (read from its source, `ModalDef::up_to_that_many`), each at most once; with none chosen it is removed
/// from the stack.
static MODAL: ModalDef = modes(&[
    mode(discard_then_draw, "Discard a card, then draw a card."),
    mode(opponent_loses_2, "Target opponent loses 2 life.").targets(&[targets::OPPONENT]),
    mode(destroy_token, "Destroy target token.").targets(&[target(token)]),
    mode(effects::each_player_sacrifices_creatures::<1>, "Each player sacrifices a creature of their choice."),
])
.up_to_that_many();

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_x_counters::<{ CounterKind::PlusOne as usize }>),
    abilities: &[modal_triggered(THIS_ENTERS, &MODAL, "When The Ruinous Wrecking Crew enters, choose up to X —")],
    ..oracle::THE_RUINOUS_WRECKING_CREW
};
