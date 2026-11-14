use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select};
use cantrip_oracle as oracle;

/// Capture each life loss and its amount (CR 119.3, 603.2).
fn you_lose_life(_: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::LifeChange { player, delta } if *player == asker.controller && *delta < 0 =>
            Some(Fired { subject: Target::Player(*player), data: [delta.unsigned_abs(), 0] }),
        _ => None,
    }
}

fn other_permanent(_: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| id != choice.source)
}

static DISCARD: Question = Question::new(
    "DiscardOrSacrifice", "discard a card?",
    "For each life lost, sacrifice another permanent unless you discard a card",
    "Discard a card", "Don't discard",
);

/// Pay the optional discard for each life, then sacrifice the remaining number together (CR 118.12a, 701.21a).
fn life_lost(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let mut remaining = item.data[0];
    while remaining > 0 && !g.state().zone(Zone::hand(you)).is_empty()
        && g.may_pay(you, item.object, item.object, discard_cards(1), &DISCARD)?
    {
        remaining -= 1;
    }
    g.end_sentence();
    let eligible: Vec<ObjectId> = g.permanents_of(you).into_iter()
        .filter(|&id| id != item.object && g.can_sacrifice(you, id)).collect();
    let chosen = if eligible.len() <= remaining as usize {
        eligible
    } else {
        let rule = Selection::new(select::as_many_as_can(remaining))
            .with(select::part(&(other_permanent as SelectFn), 0, ""))
            .called("other permanent");
        let permanents = g.permanents_of(you);
        choose::select(g, you, item.object, Purpose::Sacrifice, permanents, rule)?
    };
    g.sacrifice(you, &chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(when_asked(EventMask::LIFE_CHANGE, you_lose_life), life_lost,
            "Whenever you lose life, for each 1 life you lost, sacrifice a permanent other than this enchantment unless you discard a card."),
        ability(cost("{B}{B}"), |g, item| effects::draw(g, item, 1), "{B}{B}: Draw a card."),
    ],
    ..oracle::OATH_OF_LIM_D_L
};
