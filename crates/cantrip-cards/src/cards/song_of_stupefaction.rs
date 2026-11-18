use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, effects, statics::permanent_cards_in_your_graveyard, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Fathomless descent: Enchanted permanent gets -X/-0, where X is the number of permanent cards in your graveyard." (CR 613.4c, 110.4a):
/// "your" is the Aura's controller, not the enchanted permanent's, and the count is taken again each time characteristics are computed.
fn minus_x_power_for_permanent_cards_in_your_graveyard(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let x = permanent_cards_in_your_graveyard(q, source);
    chars.power = chars.power.map(|power| power - x as i32);
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_OR_VEHICLE),
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| effects::may_mill(g, item, 2),
        "When this Aura enters, you may mill two cards. (You may put the top two cards of your library into your graveyard.)",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        attached_object,
        minus_x_power_for_permanent_cards_in_your_graveyard,
        "Fathomless descent — Enchanted permanent gets -X/-0, where X is the number of permanent cards in your graveyard.",
    )],
    ..oracle::SONG_OF_STUPEFACTION
};
