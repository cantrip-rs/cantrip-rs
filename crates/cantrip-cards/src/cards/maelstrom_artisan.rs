use cantrip_api::*;
use cantrip_mechanics::{destroy_target, prepare, targets};
use cantrip_oracle as oracle;

/// "Destroy target nonbasic land": with its target illegal the spell does not resolve (CR 608.2b).
fn rocket_volley(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    destroy_target(g, item, 0);
    Ok(())
}

/// Maelstrom Artisan's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static ROCKET_VOLLEY: CardDef = CardDef {
    name: "Rocket Volley",
    cost: mana("{1}{R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::NONBASIC_LAND], rocket_volley),
    text: "Destroy target nonbasic land.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&ROCKET_VOLLEY),
    ..oracle::MAELSTROM_ARTISAN
};

pub static EXTRA: &[&CardDef] = &[&ROCKET_VOLLEY];
