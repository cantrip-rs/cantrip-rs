use cantrip_api::*;
use cantrip_mechanics::effects::plus_one_counter_on_each_creature_you_control;
use cantrip_mechanics::tokens::{self, MUTANT};
use cantrip_mechanics::triggers::{THIS_ATTACKS, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Create X 2/2 red Mutant creature tokens, where X is the number of nontoken creatures you control" (CR 107.3, 608.2h): X is
/// worked out once, as the ability resolves, from the creatures its controller then controls - Sally Pride among them if she is
/// still there, and the tokens it makes are no part of it.
fn create_a_mutant_per_nontoken_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = g.permanents_of(you).into_iter().filter(|&id| g.chars(id).is(CardType::Creature) && !g.obj(id).token).count() as u32;
    tokens::create(g, you, &MUTANT, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            create_a_mutant_per_nontoken_creature,
            "When Sally Pride enters, create X 2/2 red Mutant creature tokens, where X is the number of nontoken creatures you control.",
        ),
        triggered(THIS_ATTACKS, plus_one_counter_on_each_creature_you_control, "Whenever Sally Pride attacks, put a +1/+1 counter on each creature you control."),
    ],
    ..oracle::SALLY_PRIDE_LIONESS_LEADER
};

pub static EXTRA: &[&CardDef] = &[&tokens::MUTANT];
