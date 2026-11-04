use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, tokens};
use cantrip_oracle as oracle;

/// +1: "Exile the top card of your library. You may cast that card. If you don't, Chandra deals 2 damage to each
/// opponent." The card is offered only if it could be cast now (CR 608.2g): it is not a land and has a legal target
/// for everything it needs. An empty library exiles nothing, and then nothing was cast.
fn impulse(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !library::exile_top_may_cast(g, item.controller, item.controller, item.object)? {
        effects::damage_each_opponent(g, item, 2);
    }
    Ok(())
}

fn add_red_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 2);
    Ok(())
}

fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

pub static EMBLEM: CardDef = CardDef {
    name: "Chandra, Torch of Defiance emblem",
    has_cost: false,
    abilities: &[triggered(cantrip_mechanics::triggers::YOU_CAST_A_SPELL, effects::damage_first_target::<5>, "Whenever you cast a spell, this emblem deals 5 damage to any target.")
        .targets(&[targets::ANY])],
    text: "Whenever you cast a spell, this emblem deals 5 damage to any target.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), impulse, "+1: Exile the top card of your library. You may cast that card. If you don't, Chandra deals 2 damage to each opponent."),
        ability(loyalty(1), add_red_red, "+1: Add {R}{R}."),
        ability(loyalty(-3), effects::damage_first_target::<4>, "\u{2212}3: Chandra deals 4 damage to target creature.")
            .targets(&[targets::CREATURE]),
        ability(loyalty(-7), emblem, "\u{2212}7: You get an emblem with \"Whenever you cast a spell, this emblem deals 5 damage to any target.\""),
    ],
    ..oracle::CHANDRA_TORCH_OF_DEFIANCE
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
