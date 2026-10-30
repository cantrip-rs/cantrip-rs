use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::mana::{aura_enters_choosing_a_color, enchanted_permanent_tapped_for_mana, enchanted_permanents_controller_adds_the_chosen_color};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Enchanted land is the chosen color" (CR 105.3, 613.1e): the color chosen as the Aura entered replaces the land's colors
/// (CR 607.2d); it is no color word, so a text-changing effect leaves it (CR 612.2). An Aura with none chosen leaves the land as it is.
fn the_chosen_color(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    if let Some(color) = q.state().chosen_color(q.linked_to(source)) {
        chars.colors = ColorSet::of(color);
    }
}

pub static CARD: CardDef = CardDef {
    // "Enchant land" (CR 702.5a, 303.4a).
    enchant: Some(targets::LAND),
    enters: Some(aura_enters_choosing_a_color),
    abilities: &[triggered(
        when(EventMask::TAPPED, enchanted_permanent_tapped_for_mana),
        enchanted_permanents_controller_adds_the_chosen_color,
        "Whenever enchanted land is tapped for mana, its controller adds an additional one mana of the chosen color.",
    )
    .mana_ability(TriggeredMana::ChosenColor)],
    statics: &[static_ability(Layer::Color, attached_object, the_chosen_color, "Enchanted land is the chosen color.")],
    ..oracle::SHIMMERWILDS_GROWTH
};
