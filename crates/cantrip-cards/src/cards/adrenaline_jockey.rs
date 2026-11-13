use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever a player casts a spell, if it's not their turn" (CR 601.2i, 603.2, 603.4): any player's spell cast (not a
/// copy, CR 707.10), the ability's controller included, judged by whose turn it is as it is cast. The condition is the
/// intervening "if" (CR 603.4); a turn cannot change while the ability waits on the stack, so the check as it resolves
/// (CR 603.4) holds whenever the check as it triggers does. The ability is about the player who cast it
/// (`StackItem::subject`, "them").
fn a_player_casts_a_spell_on_a_turn_not_theirs(q: &dyn Query, _: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if q.state().turn.active != *player => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

/// "This creature deals 4 damage to them": to the player who cast the spell, dealt by the creature (CR 120.3, off its
/// last-known information if it has left the battlefield, CR 608.2h).
fn four_damage_to_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_that_player(g, item, 4)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when(EventMask::SPELL_CAST, a_player_casts_a_spell_on_a_turn_not_theirs),
            four_damage_to_them,
            "Whenever a player casts a spell, if it's not their turn, this creature deals 4 damage to them.",
        ),
        triggered(
            triggers::YOU_ACTIVATE_AN_EXHAUST_ABILITY,
            effects::plus_one_counter_on_this,
            "Whenever you activate an exhaust ability, put a +1/+1 counter on this creature.",
        ),
    ],
    ..oracle::ADRENALINE_JOCKEY
};
