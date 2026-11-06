use cantrip_api::select::{Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers};
use cantrip_oracle as oracle;

/// Blue creatures don't untap during their controllers' untap steps (CR 502.3).
fn blue_creatures_dont_untap(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).colors.has(q.words(asker.source).color(Color::Blue))
        && q.chars(id).is(CardType::Creature)
}

fn tapped_blue_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| {
        q.obj(id).tapped
            && q.chars(id).is(CardType::Creature)
            && q.chars(id).colors.has(q.words(choice.source).color(Color::Blue))
    })
}

/// Choose creatures and pay for them; if paid, untap them (CR 608.2d, 118.12).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    let among = g.permanents_of(player);
    let kind = match g.words(item.object).color(Color::Blue) {
        Color::White => "tapped white creature",
        Color::Blue => "tapped blue creature",
        Color::Black => "tapped black creature",
        Color::Red => "tapped red creature",
        Color::Green => "tapped green creature",
    };
    let rule = select::cards(select::up_to(among.len() as u32))
        .with(select::part(&(tapped_blue_creature as SelectFn), 0, ""))
        .called(kind);
    let chosen = choose::select(g, player, item.object, Purpose::Choose, among, rule)?;
    let price = cost("{4}").mana.times(chosen.len() as u32);
    let paid = match price {
        Some(mana) if !chosen.is_empty() => g.may_pay(player, item.object, item.object, Cost::mana(mana), &PAY_QUESTION)?,
        _ => false,
    };
    g.end_sentence();
    if paid {
        g.untap_together(&chosen);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new("Pay", "pay {cost}?", "", "Pay {cost}", "Decline");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::EACH_PLAYERS_UPKEEP,
        upkeep,
        "At the beginning of each player's upkeep, that player may choose any number of tapped blue creatures they control and pay {4} for each creature chosen this way. If the player does, untap those creatures.",
    )],
    untap_bans: &[blue_creatures_dont_untap],
    ..oracle::MAGNETIC_MOUNTAIN
};
