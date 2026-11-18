use cantrip_api::*;
use cantrip_mechanics::statics::permanent_cards_in_your_graveyard;
use cantrip_mechanics::targets::you_have_permanent_cards_in_graveyard;
use cantrip_mechanics::triggers::{THIS_ENTERS, YOUR_END_STEP};
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Neo Exdeath's power is equal to the number of permanent cards in your graveyard" (CR 604.3, layer 7a): its controller's
/// graveyard, each card judged as it is there.
fn power_is_permanent_cards_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.power = Some(permanent_cards_in_your_graveyard(q, source) as i32);
}

pub static BACK: CardDef = CardDef {
    statics: &[
        characteristic_defining(
            Layer::PtDefine,
            power_is_permanent_cards_in_your_graveyard,
            "Neo Exdeath's power is equal to the number of permanent cards in your graveyard.",
        )
        .lost_pt(0, 3),
    ],
    ..oracle::NEO_EXDEATH_DIMENSIONS_END
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, |g, item| effects::gain_life(g, item, 3), "When Exdeath enters, you gain 3 life."),
        triggered(
            YOUR_END_STEP,
            effects::transform_self,
            "At the beginning of your end step, if there are six or more permanent cards in your graveyard, transform Exdeath.",
        )
        .condition(you_have_permanent_cards_in_graveyard::<6>),
    ],
    back: Some(&BACK),
    ..oracle::EXDEATH_VOID_WARLOCK
};
