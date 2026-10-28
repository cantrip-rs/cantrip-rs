use cantrip_api::*;
use cantrip_mechanics::effects::may_draw_a_card;
use cantrip_mechanics::triggers::token_you_control_enters;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Whenever one or more tokens you control enter" (CR 603.2c): one trigger for a batch that enters together.
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, token_you_control_enters),
        may_draw_a_card,
        "Whenever one or more tokens you control enter, you may draw a card.",
    )
    .once_per_batch()],
    ..oracle::MISTER_FANTASTIC_REED_RICHARDS
};
