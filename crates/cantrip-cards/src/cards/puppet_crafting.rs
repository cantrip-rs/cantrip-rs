use cantrip_api::*;
use cantrip_mechanics::statics::{also_a_creature_and_a, base_pt};
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::{effects, lands, targets};
use cantrip_oracle as oracle;

/// "Enchant artifact or non-Aura enchantment" (CR 303.4a, 115.1): a permanent that is an artifact, or an enchantment that is not an
/// Aura, as the layers give it - checked as the Aura is cast and, being the Aura's restriction, again whenever it is on the battlefield
/// (CR 303.4d, 704.5m). The Aura subtype is no word of this card's text (Aura is an enchantment type, CR 303.4).
fn artifact_or_non_aura_enchantment(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && {
        let c = q.chars(id);
        c.is(CardType::Artifact) || (c.is(CardType::Enchantment) && !c.has_subtype(Subtype::Aura))
    })
}

pub static CARD: CardDef = CardDef {
    enchant: Some(target(artifact_or_non_aura_enchantment)),
    abilities: &[ability(cost("{4}{G}"), effects::return_this_to_hand, "{4}{G}: Return this card from your graveyard to your hand.").from(ZoneKind::Graveyard)],
    statics: &[
        static_ability(
            Layer::Type,
            attached_object,
            also_a_creature_and_a::<{ lands::sub(Subtype::Construct) }>,
            "Enchanted permanent is a Construct creature with base power and toughness 5/5 in addition to its other types.",
        ),
        static_ability(
            Layer::PtSet,
            attached_object,
            base_pt::<5, 5>,
            "Enchanted permanent is a Construct creature with base power and toughness 5/5 in addition to its other types.",
        )
        .continues(),
    ],
    ..oracle::PUPPET_CRAFTING
};
