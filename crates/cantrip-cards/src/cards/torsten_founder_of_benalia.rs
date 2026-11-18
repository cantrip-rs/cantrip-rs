use cantrip_api::*;
use cantrip_mechanics::tokens::{self, SOLDIER};
use cantrip_mechanics::top_of_library;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Reveal the top seven cards of your library. Put any number of creature and/or land cards from among them into
/// your hand and the rest on the bottom of your library in a random order." With no creature or land card among them
/// nobody is asked.
fn reveal_seven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let revealed = top_of_library(g, player, 7);
    if revealed.is_empty() {
        return Ok(());
    }
    g.reveal(player, &revealed);
    let eligible: Vec<ObjectId> = revealed
        .iter()
        .copied()
        .filter(|&c| {
            let chars = g.chars(c);
            chars.is(CardType::Creature) || chars.is(CardType::Land)
        })
        .collect();
    let mut taken = Vec::new();
    if !eligible.is_empty() {
        let ask = RequestKind::DistributeCards {
            source: item.object,
            cards: revealed.clone(),
            destinations: vec![
                Destination { kind: DestinationKind::Hand, min: 0, max: eligible.len() as u32, eligible },
                Destination { kind: DestinationKind::LibraryBottom, min: 0, max: u32::MAX, eligible: revealed.clone() },
            ],
            may_shuffle: false,
        };
        if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
            taken = placements.first().cloned().unwrap_or_default();
        }
    }
    if !taken.is_empty() {
        g.return_to_hand_revealed(&taken);
    }
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|c| !taken.contains(c)).collect();
    g.bottom_in_random_order(player, &rest);
    Ok(())
}

fn create_soldiers(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &SOLDIER, 7)?;
    Ok(())
}

pub static EXTRA: &[&CardDef] = &[&SOLDIER];

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, reveal_seven, "When Torsten enters, reveal the top seven cards of your library. Put any number of creature and/or land cards from among them into your hand and the rest on the bottom of your library in a random order."),
        triggered(triggers::THIS_DIES, create_soldiers, "When Torsten dies, create seven 1/1 white Soldier creature tokens."),
    ],
    ..oracle::TORSTEN_FOUNDER_OF_BENALIA
};
