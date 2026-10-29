use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

fn create_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::RED_AND_GREEN_WORM, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::landfall),
            create_token,
            "Landfall \u{2014} Whenever a land you control enters, create a 2/2 red and green Worm creature token with \"This token can't block.\""
        ),
        ability(cost("{8}"), effects::creatures_you_control_get::<2, 0>, "{8}: Creatures you control get +2/+0 until end of turn.")
    ],
    ..oracle::CLEVELAND_BOOKER_EMPATH
};

pub static EXTRA: &[&CardDef] = &[&tokens::RED_AND_GREEN_WORM];
