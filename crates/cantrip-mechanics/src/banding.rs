//! "Bands with other [quality]" (CR 702.22). Combat procedures belong to the core.
use cantrip_api::*;

pub const fn with_others(quality: BandQuality, text: &'static str) -> AbilityDef {
    AbilityDef::BandsWithOthers(BandsWithOthersDef { quality, text })
}

/// "Legendary creature" as a band quality (CR 702.22c): Adventurers' Guildhouse and Cathedral of Serra grant "bands with
/// other legendary creatures."
pub fn legendary_creature(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).supertypes.legendary && q.chars(id).is(CardType::Creature)
}

/// "Bands with other legendary creatures." in its printed words, as a granted ability shows them.
pub const WITH_OTHER_LEGENDARY_CREATURES: AbilityDef = with_others(legendary_creature, "Bands with other legendary creatures");
