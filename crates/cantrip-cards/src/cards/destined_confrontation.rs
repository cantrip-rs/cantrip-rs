use cantrip_api::select::{part, Bits, Choice, SelectFn, Verdict};
use cantrip_api::*;
use cantrip_mechanics::{choose, select};
use cantrip_oracle as oracle;

fn power(q: &dyn Query, id: ObjectId) -> i64 {
    i64::from(q.chars(id).power.unwrap_or(0))
}

/// "With total power 4 or less" (CR 208.1, 107.1b): the powers of the creatures chosen, each as it is now, add up to at most
/// `most`, a negative power taking away from the total. That is not downward-closed - a 5/5 and a -2/2 are an answer
/// though the 5/5 alone is not - so a creature is addable while some answer still holds the creatures chosen with it: when the
/// total, with every negative power not yet chosen taken as well, stays within `most` once the creature's own power (if
/// positive) is counted. Picking addable creatures in any order then reaches every answer, and the negative ones can be added
/// at any time.
fn total_power(q: &dyn Query, choice: &Choice, most: u32) -> Verdict {
    let total: i64 = choice.selected.iter().map(|&c| power(q, c)).sum();
    let unchosen_negative: i64 = choice.candidates.iter().filter(|c| !choice.selected.contains(c)).map(|&c| power(q, c).min(0)).sum();
    Verdict {
        complete: total <= i64::from(most),
        addable: Bits::of(choice.candidates.len(), |i| total + unchosen_negative + power(q, choice.candidates[i]).max(0) <= i64::from(most)),
    }
}

const TOTAL_POWER_AT_MOST_FOUR: select::Part = part(&(total_power as SelectFn), 4, "with total power {n} or less");

/// "Each player chooses any number of creatures they control with total power 4 or less, then sacrifices all other creatures
/// they control." (CR 101.4, 608.2d, 701.21a): in turn order, each player chooses from the creatures they control, knowing what
/// the players before them chose; nothing is sacrificed until every choice is made, and then every creature not chosen is
/// sacrificed at once.
fn each_player_keeps_creatures_of_total_power_four(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::Selection::of(select::ANY_NUMBER, &[TOTAL_POWER_AT_MOST_FOUR]).called("creature");
    let mut doomed: Vec<(PlayerId, ObjectId)> = Vec::new();
    for who in g.state().apnap().collect::<Vec<_>>() {
        let creatures: Vec<ObjectId> = g.permanents_of(who).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
        let kept = choose::select(g, who, item.object, Purpose::Keep, creatures.clone(), rule.clone())?;
        doomed.extend(creatures.into_iter().filter(|id| !kept.contains(id)).map(|id| (who, id)));
    }
    g.sacrifice_each(&doomed);
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], each_player_keeps_creatures_of_total_power_four), ..oracle::DESTINED_CONFRONTATION };
