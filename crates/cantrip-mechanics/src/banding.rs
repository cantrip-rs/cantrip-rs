//! "Bands with other [quality]" (CR 702.22). Combat procedures belong to the core.
use cantrip_api::*;

pub const fn with_others(quality: BandQuality, text: &'static str) -> AbilityDef {
    AbilityDef::BandsWithOthers(BandsWithOthersDef { quality, text })
}
