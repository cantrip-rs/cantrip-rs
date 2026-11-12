use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed_in_graveyard;
use cantrip_mechanics::{effects, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    statics: &[
        static_ability(Layer::Ability, statics::itself_during_your_turn, statics::have_first_strike, "During your turn, this creature has first strike."),
        max_speed_in_graveyard(statics::grant_delayed_ability, "Max speed \u{2014} {3}, Exile this card from your graveyard: Draw a card."),
    ],
    delayed: &[ability(cost("{3}").and(EXILE_THIS), |g, item| effects::draw(g, item, 1), "{3}, Exile this card from your graveyard: Draw a card.").from(ZoneKind::Graveyard)],
    ..oracle::LEONIN_SURVEYOR
};
