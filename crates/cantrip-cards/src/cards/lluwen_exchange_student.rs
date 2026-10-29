use cantrip_api::*;
use cantrip_mechanics::targets::filters;
use cantrip_mechanics::{prepare, tokens};
use cantrip_oracle as oracle;

/// Pest Friend, Lluwen's prepare spell (CR 722): not a card, only ever a copy.
pub static PEST_FRIEND: CardDef = CardDef {
    name: "Pest Friend",
    cost: mana("{B/G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], |g, item| tokens::create_a_pest_that_gains_life_on_attack(g, item)),
    text: "Create a 1/1 black and green Pest creature token with \"Whenever this token attacks, you gain 1 life.\"",
    ..CardDef::BLANK
};

/// "Exile a creature card from your graveyard: Lluwen becomes prepared" is a cost-paid ability, so it can be activated while
/// already prepared, and then does nothing (CR 722.3a).
pub static CARD: CardDef = CardDef {
    enters: Some(prepare::enters_prepared),
    prepare: Some(&PEST_FRIEND),
    abilities: &[ability(
        exile_from_graveyard_of(1, filters::creature),
        prepare::become_prepared,
        "Exile a creature card from your graveyard: This creature becomes prepared. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    ..oracle::LLUWEN_EXCHANGE_STUDENT
};

pub static EXTRA: &[&CardDef] = &[&PEST_FRIEND, &tokens::PEST_THAT_GAINS_LIFE_ON_ATTACK];
