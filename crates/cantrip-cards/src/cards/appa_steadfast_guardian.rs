use cantrip_api::*;
use cantrip_mechanics::{airbend_targets, triggers::{THIS_ENTERS, YOU_CAST_A_SPELL_FROM_EXILE}, targets, tokens};
use cantrip_oracle as oracle;

fn ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::ALLY, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, airbend_targets, "When Appa enters, airbend any number of other target nonland permanents you control.")
            .targets(&[up_to(TargetSpec::ANY_NUMBER, targets::other_nonland_permanent_you_control_filter)]),
        triggered(YOU_CAST_A_SPELL_FROM_EXILE, ally, "Whenever you cast a spell from exile, create a 1/1 white Ally creature token."),
    ],
    ..oracle::APPA_STEADFAST_GUARDIAN
};

pub static EXTRA: &[&CardDef] = &[&tokens::ALLY];
