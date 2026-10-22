use cantrip_api::*;
use cantrip_mechanics::library::reveal_top_take_one_rest_to_graveyard;
use cantrip_oracle as oracle;

/// The 0/1 colorless Eldrazi Spawn creature token with "Sacrifice this token: Add {C}."
pub static ELDRAZI_SPAWN: CardDef = CardDef {
    name: "Eldrazi Spawn Token",
    token_name: Some(&[Subtype::Eldrazi, Subtype::Spawn]),
    has_cost: false,
    types: TypeSet::of(&[CardType::Creature]),
    subtypes: SubtypeSet::of(&[Subtype::Eldrazi, Subtype::Spawn]),
    power: Some(0),
    toughness: Some(1),
    abilities: &[mana_ability(SACRIFICE_THIS, add(Mana::C), "Sacrifice this token: Add {C}.")],
    text: "Sacrifice this token: Add {C}.",
    ..CardDef::BLANK
};

/// Reveal the top four cards of your library; you may put a permanent card from among them into your hand; put the rest
/// into your graveyard, in the order they were revealed. With no permanent card among them there is nothing to decide.
/// Then create an Eldrazi Spawn.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    reveal_top_take_one_rest_to_graveyard(g, player, item.object, 4, cantrip_mechanics::targets::is_permanent_card)?;
    cantrip_mechanics::tokens::create(g, player, &ELDRAZI_SPAWN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::MALEVOLENT_RUMBLE };

pub static EXTRA: &[&CardDef] = &[&ELDRAZI_SPAWN];
