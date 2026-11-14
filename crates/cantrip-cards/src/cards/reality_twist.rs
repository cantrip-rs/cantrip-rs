use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, cumulative_upkeep::Payment, replacements::land_type_mana};
use cantrip_oracle as oracle;

fn types(q: &dyn Query, asker: &Asker, permanent: ObjectId, _: PlayerId) -> Vec<Mana> {
    land_type_mana(q, asker, permanent, &[(Subtype::Plains, Mana::R), (Subtype::Swamp, Mana::G), (Subtype::Mountain, Mana::W), (Subtype::Forest, Mana::B)])
}

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(Payment::Mana(mana("{1}{U}{U}")), "Cumulative upkeep {1}{U}{U}")],
    replacements: &[mana_type_replacement(types, "land produces its listed mana instead")],
    ..oracle::REALITY_TWIST
};
