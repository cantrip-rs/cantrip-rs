use cantrip_api::*;
use cantrip_mechanics::triggers::{THIS_DIES, THIS_ENTERS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("ExileParamecia", "exile Paramecia Coloniex to put a creature card from your graveyard on top of your library?", "Choose a creature card from your graveyard", "Exile it", "Don't");

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::you_mill::<3>, "When this creature enters, mill three cards."),
        triggered(
            THIS_DIES,
            |g, item| effects::may_exile_it_when_you_do(g, item, &CARD, &ASK),
            "When this creature dies, you may exile it. When you do, put target creature card from your graveyard on top of your library.",
        ),
    ],
    delayed: &[delayed_trigger(effects::put_first_target_on_top_of_library, "When you do, put target creature card from your graveyard on top of your library.")
        .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    ..oracle::PARAMECIA_COLONIEX
};
