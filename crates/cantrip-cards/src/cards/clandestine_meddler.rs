use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more suspected creatures you control attack" (CR 508.1m, 603.2c, 701.60a): a creature its controller declares as
/// an attacker that is suspected as it is declared. Those declared together are one batch, so with `once_per_batch` the ability
/// triggers once however many of them attack; a creature that is suspected only after attackers are declared does not trigger it.
fn you_attack_with_a_suspected_creature(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if q.controller(*attacker) == me.controller && q.is_suspected(*attacker) => Fired::about(Target::Object(*attacker)),
        _ => None,
    }
}

/// "Surveil 1." (CR 701.25a).
fn surveil_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::suspect_target::<0>,
            "When this creature enters, suspect up to one other target creature you control. (A suspected creature has menace and can't block.)",
        )
        .targets(&[targets::OTHER_CREATURE_YOU_CONTROL.between(0, 1)]),
        triggered(
            when_asked(EventMask::ATTACKS, you_attack_with_a_suspected_creature),
            surveil_1,
            "Whenever one or more suspected creatures you control attack, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)",
        )
        .once_per_batch(),
    ],
    ..oracle::CLANDESTINE_MEDDLER
};
