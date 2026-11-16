use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::statics::{other_creatures_of_subtype_you_control, plus_pt};
use cantrip_mechanics::{lands, tokens, triggers};
use cantrip_oracle as oracle;

/// "... has vigilance, trample, and haste" (CR 702.20, 702.19, 702.10, layer 6): all three in the one layer-6 effect.
fn have_vigilance_trample_and_haste(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Vigilance).with(Keyword::Trample).with(Keyword::Haste);
}

/// Cragflame, a legendary colorless Equipment artifact token with "Equipped creature gets +1/+1 and has vigilance, trample, and haste" and
/// equip {2} (CR 111.4, 301.5, 205.4a: the token is named by the text that makes it).
pub static CRAGFLAME: CardDef = CardDef {
    name: "Cragflame",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    supertypes: Supertypes { basic: false, legendary: true, snow: false, world: false },
    subtypes: SubtypeSet::of(&[Subtype::Equipment]),
    abilities: &[equip(mana("{2}"), "Equip {2}")],
    statics: &[
        static_ability(Layer::PtModify, attached_object, plus_pt::<1, 1>, "Equipped creature gets +1/+1."),
        static_ability(Layer::Ability, attached_object, have_vigilance_trample_and_haste, "Equipped creature has vigilance, trample, and haste."),
    ],
    text: "Equipped creature gets +1/+1 and has vigilance, trample, and haste.\nEquip {2}",
    ..CardDef::BLANK
};

/// "Create Cragflame, a legendary colorless Equipment artifact token ..." (CR 111.1): one `CRAGFLAME` under the ability's controller.
fn create_cragflame(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &CRAGFLAME, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        create_cragflame,
        "When Mabel enters, create Cragflame, a legendary colorless Equipment artifact token with \"Equipped creature gets +1/+1 and has vigilance, trample, and haste\" and equip {2}.",
    )],
    statics: &[static_ability(
        Layer::PtModify,
        other_creatures_of_subtype_you_control::<{ lands::sub(Subtype::Mouse) }>,
        plus_pt::<1, 1>,
        "Other Mice you control get +1/+1.",
    )],
    ..oracle::MABEL_HEIR_TO_CRAGFLAME
};

pub static EXTRA: &[&CardDef] = &[&CRAGFLAME];
