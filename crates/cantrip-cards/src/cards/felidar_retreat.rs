use cantrip_api::*;
use cantrip_mechanics::{effects, tokens, triggers};
use cantrip_oracle as oracle;

/// A 2/2 white Cat Beast creature token (CR 111.4, 205.3m).
static CAT_BEAST: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Cat Beast)
};

/// Mode 0: "Create a 2/2 white Cat Beast creature token."
fn make_token(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &CAT_BEAST, 1)?;
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(make_token, "Create a 2/2 white Cat Beast creature token."),
    mode(
        effects::plus_one_counter_on_each_creature_you_control_which_gain_until_end_of_turn::<{ Keywords::of(&[Keyword::Vigilance]).0 }>,
        "Put a +1/+1 counter on each creature you control. Those creatures gain vigilance until end of turn.",
    ),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        triggers::LANDFALL,
        &MODAL,
        "Landfall \u{2014} Whenever a land you control enters, choose one \u{2014}\n\u{2022} Create a 2/2 white Cat Beast creature token.\n\u{2022} Put a +1/+1 counter on each creature you control. Those creatures gain vigilance until end of turn.",
    )],
    ..oracle::FELIDAR_RETREAT
};

pub static EXTRA: &[&CardDef] = &[&CAT_BEAST];
