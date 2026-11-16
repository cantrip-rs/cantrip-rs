use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::expend::EXPEND_4;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Discard", "discard your hand?", "If you do, draw two cards", "Discard, draw two", "Keep your hand");

/// "You may discard your hand. If you do, draw two cards" (CR 608.2c-d, 701.9): the controller is asked as it resolves; an empty hand
/// may be discarded too, and that counts as having done it.
fn may_discard_hand_to_draw_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_discard_hand(g, item, &ASK)?.is_some() {
        g.draw(item.controller, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        EXPEND_4,
        may_discard_hand_to_draw_two,
        "Whenever you expend 4, you may discard your hand. If you do, draw two cards. (You expend 4 as you spend your fourth total mana to cast spells during a turn.)",
    )],
    ..oracle::BYWAY_BARTERER
};
