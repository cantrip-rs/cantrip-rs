use cantrip_api::*;
use cantrip_mechanics::{choose, delayed, exile_target, targets, tokens};
use cantrip_oracle as oracle;

/// +1: draw a card, then "at the beginning of the next end step, untap up to two lands" (CR 603.7), created as the ability
/// resolves.
fn plus_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    delayed::at_next_end_step(g, item, 0, Target::None);
    Ok(())
}

/// Untap up to two lands, chosen as the delayed ability resolves and not targeted: any lands on the battlefield, tapped or
/// not, nothing asked when there are none.
fn untap_two_lands(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    let chosen = choose::cards(g, item.controller, item.object, Purpose::Untap, lands, 2, true)?;
    g.untap_together(&chosen);
    Ok(())
}

/// -3: put target nonland permanent into its owner's library third from the top (a token simply ceases to exist).
fn minus_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(id) = targets::object(g, item, 0) {
        g.put_into_library(&[id], LibraryPlace::FromTop(2));
    }
    Ok(())
}

fn emblem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_emblem(g, item.controller, &EMBLEM);
    Ok(())
}

// --- The emblem ---------------------------------------------------------------------------------------------------

fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    exile_target(g, item, 0);
    Ok(())
}

pub static EMBLEM: CardDef = CardDef {
    name: "Teferi, Hero of Dominaria emblem",
    has_cost: false,
    abilities: &[triggered(cantrip_mechanics::triggers::YOU_DRAW_A_CARD, exile_it, "Whenever you draw a card, exile target permanent an opponent controls.")
        .targets(&[targets::PERMANENT_AN_OPPONENT_CONTROLS])],
    text: "Whenever you draw a card, exile target permanent an opponent controls.",
    ..CardDef::BLANK
};

pub static EXTRA: &[&CardDef] = &[&EMBLEM];

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), plus_one, "+1: Draw a card. At the beginning of the next end step, untap up to two lands."),
        ability(loyalty(-3), minus_three, "\u{2212}3: Put target nonland permanent into its owner's library third from the top.")
            .targets(&[targets::NONLAND_PERMANENT]),
        ability(loyalty(-8), emblem, "\u{2212}8: You get an emblem with \"Whenever you draw a card, exile target permanent an opponent controls.\""),
    ],
    delayed: &[delayed_trigger(untap_two_lands, "Untap up to two lands.")],
    ..oracle::TEFERI_HERO_OF_DOMINARIA
};
