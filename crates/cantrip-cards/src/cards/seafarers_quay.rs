use cantrip_api::*;
use cantrip_mechanics::{banding, statics};
use cantrip_oracle as oracle;

/// "Blue legendary creatures you control have 'bands with other legendary creatures.'" (CR 604.1).
fn your_blue_legendary_creatures(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    chars.colors.has(q.words(source).color(Color::Blue))
        && chars.supertypes.legendary && chars.is(CardType::Creature)
        && q.controller(source) == q.controller(affected)
}

/// "Bands with other legendary creatures" (CR 702.22c).
fn legendary_creature(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).supertypes.legendary && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, your_blue_legendary_creatures, statics::grant_delayed_ability,
        "Blue legendary creatures you control have \"bands with other legendary creatures.\"")],
    delayed: &[banding::with_others(legendary_creature, "Bands with other legendary creatures")],
    ..oracle::SEAFARERS_QUAY
};
