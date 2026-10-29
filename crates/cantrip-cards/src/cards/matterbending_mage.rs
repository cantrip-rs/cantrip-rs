use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever you cast a spell with {X} in its mana cost" (CR 107.3, 601.2i, 603.2): the spell's mana cost as it is on the
/// stack, whatever X is chosen to be; a copy is not cast (CR 707.10).
fn spell_with_x_in_its_mana_cost(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && q.mana_cost(*spell).x > 0 => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::return_first_target_to_hand, "When this creature enters, return up to one other target creature to its owner's hand.")
            .targets(&[up_to_one(targets::other_creature_filter)]),
        triggered(when_asked(EventMask::SPELL_CAST, spell_with_x_in_its_mana_cost), effects::this_cant_be_blocked_this_turn, "Whenever you cast a spell with {X} in its mana cost, this creature can't be blocked this turn."),
    ],
    ..oracle::MATTERBENDING_MAGE
};
