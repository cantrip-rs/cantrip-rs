use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, replacements::land_type_mana};
use cantrip_oracle as oracle;

fn types(q: &dyn Query, asker: &Asker, permanent: ObjectId, _: PlayerId) -> Vec<Mana> {
    land_type_mana(q, asker, permanent, &[(Subtype::Plains, Mana::R), (Subtype::Island, Mana::G), (Subtype::Swamp, Mana::W), (Subtype::Mountain, Mana::U), (Subtype::Forest, Mana::B)])
}

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::Mana(mana("{3}")), "Cumulative upkeep {3}")],
    replacements: &[mana_type_replacement(types, "land produces its listed mana instead")],
    ..oracle::NAKED_SINGULARITY
};
