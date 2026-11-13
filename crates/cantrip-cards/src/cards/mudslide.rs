use cantrip_api::select::{Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::{choose, select, triggers};
use cantrip_oracle as oracle;

/// Creatures without flying don't untap during their controllers' untap steps (CR 502.3).
fn ground_creature(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && !q.chars(id).keywords.has(Keyword::Flying)
}

fn tapped_ground_creature(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| q.obj(id).tapped && q.chars(id).is(CardType::Creature)
        && !q.chars(id).keywords.has(Keyword::Flying))
}

/// Choose creatures and pay for them together, then untap them together (CR 118.12, 701.26b).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(player) = item.subject.player() else { return Ok(()) };
    let among = g.permanents_of(player);
    let rule = select::cards(select::ANY_NUMBER)
        .with(select::part(&(tapped_ground_creature as SelectFn), 0, ""))
        .called("tapped creature without flying");
    let chosen = choose::select(g, player, item.object, Purpose::Choose, among, rule)?;
    let paid = !chosen.is_empty()
        && g.may_pay(player, item.object, item.object, Cost::mana(generic(2 * chosen.len() as u32)), &PAY_QUESTION)?;
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
        "At the beginning of each player's upkeep, that player may choose any number of tapped creatures without flying they control and pay {2} for each creature chosen this way. If the player does, untap those creatures.",
    )],
    untap_bans: &[ground_creature],
    ..oracle::MUDSLIDE
};
