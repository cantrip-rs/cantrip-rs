use cantrip_api::*;
use cantrip_mechanics::{effects, history, plot, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, if you've cast two or more spells this turn, draw a card.")
            .condition(|q, asker| history::cast_two_or_more_spells_this_turn(q.history(), asker.controller)),
        plot::plot(mana("{3}{U}"), "Plot {3}{U} (You may pay {3}{U} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)"),
    ],
    ..oracle::LOAN_SHARK
};
