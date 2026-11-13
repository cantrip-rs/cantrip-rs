use cantrip_api::*;
use cantrip_oracle as oracle;

/// "You may cast spells as though they had flash" (CR 702.8a, 601.3): every spell its controller casts, from whatever zone, each
/// judged as the face that would be cast. Flash and flying are core keywords.
fn spells_you_cast(_: &dyn Query, asker: &Asker, _: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller
}

pub static CARD: CardDef = CardDef { flash_rules: &[spells_you_cast], ..oracle::HIGH_FAE_TRICKSTER };
