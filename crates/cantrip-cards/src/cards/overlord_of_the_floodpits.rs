use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::impending;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    ways: &[impending::way(mana("{1}{U}{U}"), "Impending 4\u{2014}{1}{U}{U}")],
    enters: Some(impending::enters::<4>),
    statics: &[impending::NOT_A_CREATURE],
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_ATTACKS, effects::draw_then_discard::<2, 1>, "Whenever this permanent enters or attacks, draw two cards, then discard a card."),
        impending::REMOVE_A_TIME_COUNTER,
    ],
    ..oracle::OVERLORD_OF_THE_FLOODPITS
};
