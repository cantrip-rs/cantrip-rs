use cantrip_api::*;
use cantrip_mechanics::{history, triggers};
use cantrip_oracle as oracle;

const DISCARD: Question = Question::new(
    "Discard",
    "discard a card?",
    "If you don't, this creature deals 3 damage to you and taps if it deals damage to you",
    "Discard a card",
    "Don't discard",
);

/// The controller may discard a card to avoid the damage; tap only if damage was actually dealt to that player
/// (CR 118.12a, 120.3, 701.26a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, discard_cards(1), &DISCARD)? {
        return Ok(());
    }
    let before = g.history().head() as usize;
    g.damage(item.object, Target::Player(item.controller), 3);
    g.end_sentence();
    if history::players_dealt_damage_by(g.history(), item.object, before).contains(&item.controller) {
        g.tap(item.object);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        upkeep,
        "At the beginning of your upkeep, this creature deals 3 damage to you unless you discard a card. If it deals damage to you this way, tap it.",
    )],
    ..oracle::MISHRAS_WAR_MACHINE
};
