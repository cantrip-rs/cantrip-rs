use cantrip_api::*;
use cantrip_mechanics::{effects, lands, targets, tokens, triggers};
use cantrip_oracle as oracle;

fn make_officers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = lands::differently_named_lands(g, item.controller) as u32;
    tokens::create(g, item.controller, &tokens::OFFICER, x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            make_officers,
            "When this creature enters, create X 1/1 red Officer creature tokens, where X is the number of differently named lands you control.",
        ),
        triggered(
            triggers::CREATURE_YOU_CONTROL_DIES,
            effects::plus_one_counter_on_target,
            "Whenever this creature or another creature you control dies, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    ..oracle::EXCALBIAN_GAMEMASTER
};

pub static EXTRA: &[&CardDef] = &[&tokens::OFFICER];
