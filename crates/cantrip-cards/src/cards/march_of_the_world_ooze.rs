use cantrip_api::*;
use cantrip_mechanics::statics::{also_a, base_pt, your_creatures};
use cantrip_mechanics::{lands, tokens};
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a spell, if it's not their turn" (CR 603.2, 603.4, 601.2i): a spell cast (never a copy, CR 707.10)
/// by a player other than this enchantment's controller while that player is not the active one. In a game of three or more
/// players the spell's caster is the "their": it is another opponent's turn that counts. The intervening "if" (CR 603.4) is read as
/// the spell is cast, which settles it for good: the turn cannot change while the trigger waits on the stack.
fn opponent_casts_a_spell_outside_their_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player != me.controller && q.state().turn.active != *player => Fired::PLAIN,
        _ => None,
    }
}

/// "Creatures you control have base power and toughness 6/6 and are Oozes in addition to their other types." (CR 613.1d, 613.4b):
/// one ability in two layers, Oozes in layer 4 and the base 6/6 in layer 7b, for the creatures its controller controls as the
/// earlier layers made them (a creature token, an animated Vehicle and the Elephants it makes included). Counters and effects
/// that modify power and toughness (layer 7c) apply to the 6/6.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_spell_outside_their_turn),
        tokens::create_a_3_3_elephant,
        "Whenever an opponent casts a spell, if it's not their turn, you create a 3/3 green Elephant creature token.",
    )],
    statics: &[
        static_ability(
            Layer::Type,
            your_creatures,
            also_a::<{ lands::sub(Subtype::Ooze) }>,
            "Creatures you control have base power and toughness 6/6 and are Oozes in addition to their other types.",
        ),
        static_ability(
            Layer::PtSet,
            your_creatures,
            base_pt::<6, 6>,
            "Creatures you control have base power and toughness 6/6 and are Oozes in addition to their other types.",
        )
        .continues(),
    ],
    ..oracle::MARCH_OF_THE_WORLD_OOZE
};

pub static EXTRA: &[&CardDef] = &[&tokens::ELEPHANT_3_3];
