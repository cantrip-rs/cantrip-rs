use cantrip_api::*;
use cantrip_mechanics::statics::{have_vigilance, itself_with_delirium, plus_pt};
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        |g, item| lands::search_basic_land_to_hand(g, item.controller, item.object),
        "When this creature enters, search your library for a basic land card, reveal it, put it into your hand, then shuffle.",
    )],
    statics: &[
        static_ability(Layer::PtModify, itself_with_delirium, plus_pt::<1, 2>, "Delirium \u{2014} This creature gets +1/+2 as long as there are four or more card types among cards in your graveyard."),
        static_ability(Layer::Ability, itself_with_delirium, have_vigilance, "Delirium \u{2014} This creature has vigilance as long as there are four or more card types among cards in your graveyard."),
    ],
    ..oracle::SPINESEEKER_CENTIPEDE
};
