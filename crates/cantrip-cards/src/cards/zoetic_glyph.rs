use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::{also_a_creature_and_a, base_pt};
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{discover, lands, targets};
use cantrip_oracle as oracle;

const TEXT: &str = "Enchanted artifact is a Golem creature with base power and toughness 5/4 in addition to its other types.";

/// "Discover 3" (CR 701.57a): the Aura's controller discovers as it resolves, whether the enchanted artifact left the battlefield or the Aura itself
/// was destroyed (CR 603.6c, 700.4: "put into a graveyard from the battlefield").
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::ARTIFACT),
    abilities: &[triggered(THIS_DIES, |g, item| discover::discover(g, item.controller, 3), "When this Aura is put into a graveyard from the battlefield, discover 3.")],
    statics: &[
        // Layer 4 (CR 613.1d): it is a creature and a Golem (read through the words of this card's text, CR 612.2) beside its other types; layer 7b
        // (CR 613.4b): its base power and toughness are 5/4, which counters and other effects then modify.
        static_ability(Layer::Type, attached_object, also_a_creature_and_a::<{ lands::sub(Subtype::Golem) }>, TEXT),
        static_ability(Layer::PtSet, attached_object, base_pt::<5, 4>, TEXT).continues(),
    ],
    ..oracle::ZOETIC_GLYPH
};
