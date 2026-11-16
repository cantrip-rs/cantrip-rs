use cantrip_api::*;
use cantrip_mechanics::attach::{attach_to_target, attached_object, equip};
use cantrip_mechanics::statics::{lose_flying, plus_pt};
use cantrip_mechanics::{gift, targets, tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    gift: Some(gift::TAPPED_FISH),
    abilities: &[
        gift::trigger("When this permanent enters, if its gift cost was paid, the chosen opponent creates a tapped 1/1 blue Fish creature token."),
        // The intervening "if the gift was promised" (CR 603.4) is the gift's condition; it is attached only if the target
        // is still a legal one as this resolves (CR 701.3b).
        triggered(THIS_ENTERS, attach_to_target, "When this Equipment enters, if the gift was promised, attach this Equipment to target creature you control.")
            .condition(gift::paid)
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        equip(mana("{3}"), "Equip {3}"),
    ],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<3, 3>, "Equipped creature gets +3/+3."),
        static_ability(Layer::Ability, attached_object, lose_flying, "Equipped creature loses flying."),
    ],
    ..oracle::STARFORGED_SWORD
};
