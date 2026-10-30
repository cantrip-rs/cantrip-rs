use cantrip_api::*;
use cantrip_mechanics::targets::{self, is_nonland_permanent_card};
use cantrip_mechanics::{choose, damage_target, tokens, top_of_library};
use cantrip_oracle as oracle;

/// +1: "Create a 1/1 green and white Kithkin creature token."
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::KITHKIN, 1)?;
    Ok(())
}

/// -2: "Ajani deals 4 damage to target tapped creature." A target that is no longer a tapped creature is illegal
/// (CR 608.2b), and the ability does nothing.
fn minus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    damage_target(g, item, 0, 4);
    Ok(())
}

/// -8: "Look at the top X cards of your library, where X is your life total. You may put any number of nonland
/// permanent cards with mana value 3 or less from among them onto the battlefield. Then shuffle." The cards are put
/// onto the battlefield together (CR 110.2a); a card in a library with {X} in its cost has X of 0 (CR 202.3e), which
/// `mana_value` already says. The shuffle is made whether or not any card was put onto the battlefield.
fn minus_eight(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let x = g.state().player(you).life.max(0) as usize;
    let looked = top_of_library(g, you, x);
    g.look_at(you, &looked);
    let eligible: Vec<ObjectId> =
        looked.iter().copied().filter(|&c| is_nonland_permanent_card(&g.chars(c)) && g.mana_value(c) <= 3).collect();
    let chosen = choose::any_number_of(
        g,
        you,
        item.object,
        Purpose::PutOntoBattlefield,
        "nonland mana-value-3-or-less permanent",
        looked,
        eligible,
    )?;
    if !chosen.is_empty() {
        let entering: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|c| (c, you)).collect();
        g.put_all_onto_battlefield(&entering)?;
    }
    g.shuffle(you);
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&tokens::KITHKIN];

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Create a 1/1 green and white Kithkin creature token."),
        ability(loyalty(-2), minus_two, "\u{2212}2: Ajani deals 4 damage to target tapped creature.")
            .targets(&[targets::TAPPED_CREATURE]),
        ability(
            loyalty(-8),
            minus_eight,
            "\u{2212}8: Look at the top X cards of your library, where X is your life total. You may put any number of nonland permanent cards with mana value 3 or less from among them onto the battlefield. Then shuffle.",
        ),
    ],
    ..oracle::AJANI_OUTLAND_CHAPERONE
};
