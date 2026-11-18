use cantrip_api::*;
use cantrip_mechanics::top_of_library;
use cantrip_oracle as oracle;

/// Look at the top three cards of your library. Put one of them into your hand, put one of them on the bottom of your
/// library, and exile one of them. You may play the exiled card this turn. Each instruction is followed in order and as far as
/// the cards go (CR 608.2c): hand first, then the bottom, then exile, when fewer than three cards are looked at.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let looked = top_of_library(g, player, 3);
    if looked.is_empty() {
        return Ok(());
    }
    let hand = looked.len().min(1) as u32;
    let bottom = (looked.len() - hand as usize).min(1) as u32;
    let exile = (looked.len() - hand as usize - bottom as usize).min(1) as u32;
    let ask = RequestKind::DistributeCards {
        source: item.object,
        cards: looked.clone(),
        destinations: vec![
            Destination { kind: DestinationKind::Hand, min: hand, max: hand, eligible: looked.clone() },
            Destination { kind: DestinationKind::LibraryBottom, min: bottom, max: bottom, eligible: looked.clone() },
            Destination { kind: DestinationKind::Exile, min: exile, max: exile, eligible: looked },
        ],
        may_shuffle: false,
    };
    if let Answer::Placements { placements, .. } = g.ask(player, ask)? {
        // Three instructions in the order written (CR 608.2c): the hand, then the bottom of the library (nothing changes
        // zones), then the exile.
        g.return_to_hand(&placements[0]);
        g.arrange_library(player, &[], &placements[1]);
        let exiling = placements[2].first().copied();
        let moved = match exiling {
            Some(card) => {
                let owner = g.obj(card).owner;
                g.move_together(&[(card, Zone::exile(owner))], MoveReason::Other)
            }
            None => Vec::new(),
        };
        // Only a card that really arrived in exile may be played: a replacement effect could have sent it elsewhere.
        if exiling.is_some()
            && let Some(&new) = moved.last()
            && g.obj(new).zone.is(ZoneKind::Exile)
        {
            g.allow_play(new, player, PlayGrant { lands: true, expiry: Expiry::EndOfTurn, ..PlayGrant::CAST });
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::EXPRESSIVE_ITERATION };
