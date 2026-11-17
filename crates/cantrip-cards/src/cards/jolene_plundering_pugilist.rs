use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{effects, targets, tokens};
use cantrip_oracle as oracle;

/// "Whenever you attack with one or more creatures with power 4 or greater" (CR 508.1m, 603.2): the controller declared
/// attackers, at least one of whose powers, as they stood when the declaration was made, is 4 or more. One trigger for the
/// declaration, however many of them there are.
fn you_attack_with_a_creature_with_power_4_or_greater(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::AttackersDeclared { player, .. }
            if *player == me.controller && q.state().attackers.iter().any(|a| q.chars(a.id).power.is_some_and(|p| p >= 4)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ATTACKERS_DECLARED, you_attack_with_a_creature_with_power_4_or_greater),
            tokens::create_a_treasure,
            "Whenever you attack with one or more creatures with power 4 or greater, create a Treasure token.",
        ),
        ability(
            cost("{1}{R}").and(sacrifice(1, filters::treasure)),
            effects::damage_first_target::<1>,
            "{1}{R}, Sacrifice a Treasure: Jolene deals 1 damage to any target.",
        )
        .targets(&[targets::ANY]),
    ],
    ..oracle::JOLENE_PLUNDERING_PUGILIST
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
