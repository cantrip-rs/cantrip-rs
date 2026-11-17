use cantrip_api::*;
use cantrip_mechanics::{effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever a creature you control with menace attacks" (CR 508.3a, 603.2c): each attacker of the ability's controller that has
/// menace as it is declared, whether by its own text or by being suspected.
fn creature_you_control_with_menace_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && q.chars(*attacker).has(Keyword::Menace) => {
            Fired::about(Target::Object(*attacker))
        }
        _ => None,
    }
}

/// The Treasure token.
pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::suspect_target::<0>,
            "When J. Jonah Jameson enters, suspect up to one target creature. (A suspected creature has menace and can't block.)",
        )
        .targets(&[targets::CREATURE.optional()]),
        triggered(
            when_asked(EventMask::ATTACKS, creature_you_control_with_menace_attacks),
            tokens::create_a_treasure,
            "Whenever a creature you control with menace attacks, create a Treasure token.",
        ),
    ],
    ..oracle::J_JONAH_JAMESON
};
