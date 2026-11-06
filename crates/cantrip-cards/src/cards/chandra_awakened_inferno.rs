use cantrip_api::*;
use cantrip_mechanics::replacements::{EXILE_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_mechanics::{effects, history, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "+2: Each opponent gets an emblem with ..." (CR 114.2): one emblem for each opponent still in the game, which that
/// opponent owns and controls, so its trigger is about their upkeep.
fn emblems(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    for opponent in g.state().opponents(item.controller).collect::<Vec<_>>() {
        tokens::create_emblem(g, opponent, &EMBLEM);
    }
    Ok(())
}

/// "−3: Chandra deals 3 damage to each non-Elemental creature." Every creature that isn't an Elemental as the effect is
/// carried out, dealt at once (CR 608.2c, 120.2). "Elemental" is a word of Chandra's text, read through its words (CR 612).
fn sweep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let elemental = g.words(item.object).subtype(Subtype::Elemental);
    effects::damage_each_creature_not_of_subtype(g, item, elemental, 3);
    Ok(())
}

/// "−X: Chandra deals X damage to target creature or planeswalker. If a permanent dealt damage this way would die this
/// turn, exile it instead." The replacement is the ability's own effect (CR 611.2c, 614.1a) on the permanent actually dealt
/// damage (CR 120.3): none if the damage was prevented or X is 0, and it covers any way the permanent would die this turn
/// (CR 700.4).
fn burn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    cantrip_mechanics::damage_target(g, item, 0, u32::from(item.x));
    let dealt = history::permanents_dealt_damage_by(g.history(), item.object, before);
    g.end_sentence();
    instruction(g, |g| {
        for permanent in dealt {
            exile_if_it_would_die(g, item, 0, permanent);
        }
    });
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

pub static EMBLEM: CardDef = CardDef {
    name: "Chandra, Awakened Inferno emblem",
    has_cost: false,
    abilities: &[triggered(triggers::YOUR_UPKEEP, cantrip_mechanics::effects::source_deals_one_to_controller, "At the beginning of your upkeep, this emblem deals 1 damage to you.")],
    text: "At the beginning of your upkeep, this emblem deals 1 damage to you.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(2), emblems, "+2: Each opponent gets an emblem with \"At the beginning of your upkeep, this emblem deals 1 damage to you.\""),
        ability(loyalty(-3), sweep, "\u{2212}3: Chandra deals 3 damage to each non-Elemental creature."),
        ability(loyalty_minus_x(), burn, "\u{2212}X: Chandra deals X damage to target creature or planeswalker. If a permanent dealt damage this way would die this turn, exile it instead.")
            .targets(&[targets::CREATURE_OR_PLANESWALKER]),
    ],
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If a permanent dealt damage this way would die this turn, exile it instead.")],
    ..oracle::CHANDRA_AWAKENED_INFERNO
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];
