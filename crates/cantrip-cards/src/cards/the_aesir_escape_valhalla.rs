use cantrip_api::*;
use cantrip_mechanics::{choose, targets};
use cantrip_oracle as oracle;

/// Chapter I: "Exile a permanent card from your graveyard. You gain life equal to its mana value." Chosen as the chapter
/// resolves, not targeted: a lone permanent card is simply that card, none does nothing. It is exiled with this Saga
/// (CR 607.2a), and no life is gained for a mana value of zero.
fn exile_a_permanent_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let graveyard: Vec<ObjectId> = g.state().zone(Zone::graveyard(you)).to_vec();
    let eligible: Vec<ObjectId> = graveyard.iter().copied().filter(|&c| targets::is_permanent_card(&g.chars(c))).collect();
    let chosen = match eligible.len() {
        0 => return Ok(()),
        1 => eligible[0],
        _ => {
            let Some(card) = choose::card_from(g, you, item.object, Purpose::Exile, graveyard, eligible)? else { return Ok(()) };
            card
        }
    };
    let value = g.mana_value(chosen) as i32;
    g.exile_with(&[chosen], g.linked(item), false);
    if value > 0 {
        g.gain_life(you,(value) as i32);
    }
    Ok(())
}

/// Chapter II: "Put a number of +1/+1 counters on target creature you control equal to the mana value of the exiled card."
/// Read as the chapter resolves: a card that has since left exile is a new object and counts for nothing.
fn counters(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let amount: i32 = g.exiled_with(g.linked(item)).into_iter().map(|c| g.mana_value(c) as i32).sum();
    if amount > 0
        && let Some(creature) = targets::object(g, item, 0)
    {
        g.add_counters(creature, CounterKind::PlusOne, amount as i32);
    }
    Ok(())
}

/// Chapter III: "Return this Saga and the exiled card to their owner's hand", together.
fn return_both(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let saga = item.object;
    let mut returning: Vec<ObjectId> = Vec::new();
    if is_live(g, saga) && g.is_on_battlefield(saga) {
        returning.push(saga);
    }
    returning.extend(g.exiled_with(g.linked(item)));
    g.return_to_hand(&returning);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, exile_a_permanent_card, "I \u{2014} Exile a permanent card from your graveyard. You gain life equal to its mana value."),
        chapter(2, counters, "II \u{2014} Put a number of +1/+1 counters on target creature you control equal to the mana value of the exiled card.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(3, return_both, "III \u{2014} Return this Saga and the exiled card to their owner's hand."),
    ],
    ..oracle::THE_AESIR_ESCAPE_VALHALLA
};
