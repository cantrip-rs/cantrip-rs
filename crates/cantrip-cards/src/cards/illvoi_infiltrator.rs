use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// "This creature can't be blocked if you've cast two or more spells this turn" (CR 509.1b): of a possible block of the
/// Infiltrator, the spells its controller has cast this turn as blockers are declared, asked of the event history so a spell that was
/// countered or is still on the stack counts, as does the Infiltrator itself if it was cast this turn (the ruling). A block that has been
/// made stands; casting spells after blockers are declared does not undo it.
fn cant_block_the_infiltrator_after_two_spells(q: &dyn Query, source: ObjectId, _: ObjectId, attacker: ObjectId) -> bool {
    !(attacker == source && history::cast_two_or_more_spells_this_turn(q.history(), q.state().obj(source).controller))
}

pub static CARD: CardDef = CardDef {
    block_rules: &[cant_block_the_infiltrator_after_two_spells],
    abilities: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, |g, item| effects::draw(g, item, 1), "Whenever this creature deals combat damage to a player, draw a card.")],
    ..oracle::ILLVOI_INFILTRATOR
};
