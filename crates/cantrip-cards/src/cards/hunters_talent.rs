use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{class, effects, ferocious, fight, targets};
use cantrip_oracle as oracle;

/// "Whenever you attack", once the Class is level 2 or more (CR 716.2a): the level abilities stay once gained.
fn you_attack_at_level_two(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 2 { triggers::you_attack(q, me, event) } else { None }
}

/// "At the beginning of your end step", once the Class is level 3 (CR 716.2a).
fn your_end_step_at_level_three(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    if class::level(q, me.source) >= 3 { triggers::your_end_step(q, me, event) } else { None }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            fight::first_target_bites_second,
            "When this Class enters, target creature you control deals damage equal to its power to target creature you don't control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL]),
        class::level_two("{1}{G}", "{1}{G}: Level 2"),
        triggered(
            when_asked(EventMask::ATTACKERS_DECLARED, you_attack_at_level_two),
            effects::target_creature_gets_and_gains_trample::<1, 0>,
            "Whenever you attack, target attacking creature gets +1/+0 and gains trample until end of turn.",
        )
        .targets(&[targets::ATTACKING_CREATURE]),
        class::level_three("{3}{G}", "{3}{G}: Level 3"),
        triggered(
            when_asked(EventMask::STEP_BEGAN, your_end_step_at_level_three),
            |g, item| effects::draw(g, item, 1),
            "At the beginning of your end step, if you control a creature with power 4 or greater, draw a card.",
        )
        .condition(ferocious::you_control_asked),
    ],
    ..oracle::HUNTERS_TALENT
};
