use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Sacrifice an Equipment attached to Ronin" (CR 701.21a, 301.5c): an Equipment attached to the source as the cost is paid. The
/// word is that of the ability's own text (CR 612).
fn equipment_attached_to_this(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.attached_to(id) == Some(asker.source) && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Equipment))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            life(2),
            add_any_color(2).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Equipment).or_equip_abilities())),
            "Pay 2 life: Add two mana of any one color. Spend this mana only to cast Equipment spells or activate equip abilities. Activate only once each turn.",
        )
        .once_each_turn(),
        ability(
            TAP.and(sacrifice(1, equipment_attached_to_this)),
            effects::target_creature_gets::<-4, -4>,
            "{T}, Sacrifice an Equipment attached to Ronin: Target creature gets -4/-4 until end of turn. Activate only as a sorcery.",
        )
        .targets(&[targets::CREATURE])
        .sorcery_speed(),
    ],
    ..oracle::RONIN_SHADOW_STALKER
};
