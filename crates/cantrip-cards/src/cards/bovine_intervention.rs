use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Destroy target artifact or creature. Its controller creates a 2/2 white Ox creature token."
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::destroy_and_its_controller_creates(g, item, &OX)
}

/// The 2/2 white Ox creature token.
pub static OX: CardDef = CardDef {
    name: "Ox Token",
    token_name: Some(&[Subtype::Ox]),
    has_cost: false,
    colors: Some(ColorSet::of(Color::White)),
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Ox]),
    power: Some(2),
    toughness: Some(2),
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&OX];

pub static CARD: CardDef = CardDef { spell: spell(&[targets::ARTIFACT_OR_CREATURE], effect), ..oracle::BOVINE_INTERVENTION };
