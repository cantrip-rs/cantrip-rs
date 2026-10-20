use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the
/// other with a silver counter on it." "An opponent" is chosen by Karn's controller, asked only when there are several; one
/// card is no choice at all. The silver counter belongs to the exiled card (CR 122.1), so any Karn's -1 reaches it.
fn reveal_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let revealed: Vec<ObjectId> = g.state().zone(Zone::library(you)).iter().take(2).copied().collect();
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(you, &revealed);
    let taken = choose::opponent_chooses_one(g, you, item.object, revealed.clone())?.unwrap_or(revealed[0]);
    g.return_to_hand(&[taken]);
    for &other in &revealed {
        if other != taken {
            let owner = g.obj(other).owner;
            let exiled = g.move_to(other, Zone::exile(owner));
            g.add_counters(exiled, CounterKind::Silver, 1);
        }
    }
    Ok(())
}

/// "-1: Put a card you own with a silver counter on it from exile into your hand."
fn reclaim(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let eligible: Vec<ObjectId> = g
        .state()
        .zone(Zone::exile(you))
        .iter()
        .copied()
        .filter(|&c| g.obj(c).counters(CounterKind::Silver) > 0 && g.obj(c).became.is_none())
        .collect();
    let chosen = match eligible.len() {
        1 => eligible[0],
        _ => match choose::card(g, you, item.object, Purpose::PutIntoHand, eligible)? {
            Some(card) => card,
            None => return Ok(()),
        },
    };
    g.return_to_hand(&[chosen]);
    Ok(())
}

/// "-2: Create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\""
fn construct(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::CONSTRUCT, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(loyalty(1), reveal_two, "+1: Reveal the top two cards of your library. An opponent chooses one of them. Put that card into your hand and exile the other with a silver counter on it."),
        ability(loyalty(-1), reclaim, "\u{2212}1: Put a card you own with a silver counter on it from exile into your hand."),
        ability(loyalty(-2), construct, "\u{2212}2: Create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\""),
    ],
    ..oracle::KARN_SCION_OF_URZA
};

pub static EXTRA: &[&CardDef] = &[&tokens::CONSTRUCT];
