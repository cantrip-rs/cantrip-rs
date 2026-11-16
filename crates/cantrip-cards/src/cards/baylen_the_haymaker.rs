use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "An untapped token you control" as the permanents a cost taps (CR 118.3, 111.1): any token, whatever its types, that its
/// controller controls and that is untapped. Tapping them is not the {T} symbol, so a token that came under their control this
/// turn counts (CR 302.6 limits only {T}).
fn untapped_token_you_control(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.controller(id) == asker.controller && !q.obj(id).tapped && q.obj(id).token
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(tap_untapped(2, untapped_token_you_control), add_any_color(1), "Tap two untapped tokens you control: Add one mana of any color."),
        ability(tap_untapped(3, untapped_token_you_control), |g, item| effects::draw(g, item, 1), "Tap three untapped tokens you control: Draw a card."),
        ability(
            tap_untapped(4, untapped_token_you_control),
            effects::plus_one_counters_on_this_that_gains_until_end_of_turn::<3, { Keywords::of(&[Keyword::Trample]).0 }>,
            "Tap four untapped tokens you control: Put three +1/+1 counters on Baylen. It gains trample until end of turn.",
        ),
    ],
    ..oracle::BAYLEN_THE_HAYMAKER
};
