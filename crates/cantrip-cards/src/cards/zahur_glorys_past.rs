use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::tokens::{create_by_rule, ZOMBIE};
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static EXTRA: &[&CardDef] = &[&ZOMBIE];

/// "Create a tapped 2/2 black Zombie creature token" (CR 111.1, 110.5b), once for each nontoken creature that died.
fn create_a_tapped_zombie(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    create_by_rule(g, item.controller, &ZOMBIE, 1, true)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[ability(
        sacrifice(1, targets::filters::another_creature),
        |g, item| effects::surveil(g, item, 1),
        "Sacrifice another creature: Surveil 1. Activate only once each turn.",
    )
    .once_each_turn()],
    statics: &[max_speed(
        "Max speed \u{2014} Whenever a nontoken creature you control dies, create a tapped 2/2 black Zombie creature token.",
    )],
    delayed: &[triggered(
        triggers::NONTOKEN_CREATURE_YOU_CONTROL_DIES,
        create_a_tapped_zombie,
        "Whenever a nontoken creature you control dies, create a tapped 2/2 black Zombie creature token.",
    )],
    ..oracle::ZAHUR_GLORYS_PAST
};

