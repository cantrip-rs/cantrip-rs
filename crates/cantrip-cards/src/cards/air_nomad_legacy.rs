use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_creatures};
use cantrip_mechanics::{tokens, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Creatures you control with flying" (CR 613.1f, layer 6 before 7c): `your_creatures` that have flying as the earlier
/// layers made them, so a creature that gains flying in layer 6 is pumped and one that loses it is not.
fn your_fliers(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    your_creatures(q, source, affected, so_far) && so_far.keywords.has(Keyword::Flying)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ENTERS,
        |g, item| tokens::create(g, item.controller, &tokens::CLUE, 1).map(|_| ()),
        "When this enchantment enters, create a Clue token. (It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")",
    )],
    statics: &[static_ability(Layer::PtModify, your_fliers, plus_pt::<1, 1>, "Creatures you control with flying get +1/+1.")],
    ..oracle::AIR_NOMAD_LEGACY
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
