use cantrip_api::*;
use cantrip_mechanics::{behold, blight, effects, targets, triggers::THIS_LEAVES};
use cantrip_oracle as oracle;

/// "Target opponent blights 2" (CR 701.68a, 608.2b): the opponent chooses a creature they control and the counters go on it;
/// one with no creature can't blight (CR 701.68b), and nothing happens if the target is no longer legal.
fn opponent_blights_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    blight::in_effect(g, opponent, item.object, 2).map(drop)
}

/// "As an additional cost to cast this spell, behold a Goblin and exile it" (CR 601.2b, 701.4a): the Goblin is a permanent or a
/// card in hand, and is exiled as the cost is paid; it is exiled with the creature the spell becomes (CR 607.2q), which returns it
/// when it leaves the battlefield. A spell that is countered leaves it exiled. "Pay 1 life, Blight 2" is a cost its controller pays
/// (CR 118.4, 701.68a): a player with a life total under 1, or no creature to blight, can't activate it (CR 118.3, 701.68b).
pub static CARD: CardDef = CardDef {
    additional_cost: Some(behold::cost_and_exile(behold::goblin, "Goblin")),
    abilities: &[
        ability(life(1).and(blight::cost(2)), opponent_blights_two, "Pay 1 life, Blight 2: Target opponent blights 2. Activate only as a sorcery.")
            .targets(&[targets::OPPONENT])
            .sorcery_speed(),
        triggered(THIS_LEAVES, effects::return_exiled_to_hand, "When this creature leaves the battlefield, return the exiled card to its owner's hand."),
    ],
    ..oracle::CHAMPION_OF_THE_WEIRD
};
