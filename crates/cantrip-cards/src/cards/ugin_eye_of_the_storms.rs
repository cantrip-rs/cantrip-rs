use cantrip_api::*;
use cantrip_mechanics::{exile_target, library, select};
use cantrip_oracle as oracle;

/// "Permanent that's one or more colors": checked as it is chosen and again as the ability resolves (CR 115.1, 608.2b).
fn colored_permanent(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id)
        if is_live(q, id) && q.is_on_battlefield(id) && !q.chars(id).colors.is_empty())
}

/// "Exile up to one target permanent that's one or more colors."
fn exile_colored(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.target_count > 0 {
        exile_target(g, item, 0);
    }
    Ok(())
}

/// "Whenever you cast a colorless spell": judged on the spell as it is on the stack, not by its mana cost.
fn colorless_spell_you_cast(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == q.controller(source) && q.chars(*spell).colors.is_empty() => {
            Fired::PLAIN
        }
        _ => None,
    }
}

/// +2: You gain 3 life and draw a card.
fn plus_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 3);
    g.draw(item.controller, 1);
    Ok(())
}

fn colorless_nonland(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |card| { let c = q.chars(card); c.colors.is_empty() && !c.is(CardType::Land) })
}

/// -11: Search your library for any number of colorless nonland cards, exile them, then shuffle. Until end of turn, you
/// may cast those cards without paying their mana costs.
fn minus_eleven(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = item.controller;
    let rule = select::cards(select::ANY_NUMBER)
        .with(select::part(&(colorless_nonland as SelectFn), 0, ""))
        .called("colorless nonland");
    let found = library::search(g, player, player, item.object, SearchDestination::Exile, rule)?;
    let exiled = g.exile_all(&found);
    g.shuffle(player);
    for card in exiled {
        if g.obj(card).zone.is(ZoneKind::Exile) {
            g.allow_play(
                card,
                player,
                PlayGrant { instead: Some(ManaCost::ZERO), expiry: Expiry::EndOfTurn, ..PlayGrant::CAST },
            );
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::CAST_THIS_SPELL, exile_colored, "When you cast this spell, exile up to one target permanent that's one or more colors.")
            .targets(&[up_to_one(colored_permanent)]),
        triggered(when(EventMask::SPELL_CAST, colorless_spell_you_cast), exile_colored, "Whenever you cast a colorless spell, exile up to one target permanent that's one or more colors.")
            .targets(&[up_to_one(colored_permanent)]),
        ability(loyalty(2), plus_two, "+2: You gain 3 life and draw a card."),
        ability(loyalty(0), cantrip_mechanics::effects::add_colorless::<3>, "0: Add {C}{C}{C}."),
        ability(loyalty(-11), minus_eleven, "\u{2212}11: Search your library for any number of colorless nonland cards, exile them, then shuffle. Until end of turn, you may cast those cards without paying their mana costs."),
    ],
    ..oracle::UGIN_EYE_OF_THE_STORMS
};
