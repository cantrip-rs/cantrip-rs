use cantrip_api::*;
use cantrip_mechanics::triggers::YOUR_END_STEP;
use cantrip_mechanics::{effects, ferocious, targets, tokens};
use cantrip_oracle as oracle;

/// The 3/3 green Raccoon creature token (CR 111.1, 111.4).
pub static RACCOON: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Green)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(3),
    toughness: Some(3),
    ..token!(Raccoon)
};

/// "Create a 3/3 green Raccoon creature token if you control a creature with power 4 or greater." The condition is part of the
/// instruction, not an intervening "if" (CR 603.4): it is judged as the mode resolves, and nothing is created when it fails.
fn raccoon_if_power_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if ferocious::you_control(g, item.controller) {
        tokens::create(g, item.controller, &RACCOON, 1)?;
    }
    Ok(())
}

static MODES: ModalDef = modes(&[
    mode(effects::plus_one_counter_on_target, "Put a +1/+1 counter on target creature you control.").targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(raccoon_if_power_four, "Create a 3/3 green Raccoon creature token if you control a creature with power 4 or greater."),
]);

/// "At the beginning of your end step, choose one —" (CR 700.2, 603.3c): the mode is chosen as the ability is put on the stack,
/// and only the chosen mode's target is chosen.
pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        YOUR_END_STEP,
        &MODES,
        "At the beginning of your end step, choose one — Put a +1/+1 counter on target creature you control; or create a 3/3 green Raccoon creature token if you control a creature with power 4 or greater.",
    )],
    ..oracle::SYLVAN_SCAVENGING
};

pub static EXTRA: &[&CardDef] = &[&RACCOON];
