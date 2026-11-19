use cantrip_api::*;
use cantrip_mechanics::{statics, targets};
use cantrip_oracle as oracle;

/// "Fathomless descent — Souls of the Lost's power is equal to the number of permanent cards in your graveyard and its toughness is
/// equal to that number plus 1" (CR 207.2c ability word, 604.3, 110.4a): counted in its controller's graveyard whenever its
/// characteristics are, in every zone.
fn power_and_toughness_from_permanent_cards_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let count = statics::permanent_cards_in_your_graveyard(q, source) as i32;
    chars.power = Some(count);
    chars.toughness = Some(count + 1);
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(
        Layer::PtDefine,
        power_and_toughness_from_permanent_cards_in_your_graveyard,
        "Fathomless descent \u{2014} Souls of the Lost's power is equal to the number of permanent cards in your graveyard and its toughness is equal to that number plus 1.",
    )
    .lost_pt(0, 1)],
    // As an additional cost to cast this spell, discard a card or sacrifice a permanent (CR 601.2b).
    additional_choice: &[
        ExtraCost { text: "Discard a card", cost: discard_cards(1) },
        ExtraCost { text: "Sacrifice a permanent", cost: Cost::sacrifice(1, targets::filters::any) },
    ],
    ..oracle::SOULS_OF_THE_LOST
};
