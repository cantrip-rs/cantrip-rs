use std::sync::OnceLock;

use cantrip_api::Game as Rules;
use cantrip_engine::*;
use cantrip_script::{GameScript, run_on};

fn opponent_lost_life(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { player, magnitude, negative: true } if q.state().opponents(me.controller).any(|opponent| opponent == player) => {
            Some(Fired { subject: Target::Player(player), data: [magnitude, 0] })
        }
        _ => None,
    }
}

fn opponent_lost_one(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::LifeChange { magnitude: 1, negative: true, .. } => opponent_lost_life(q, me, event),
        _ => None,
    }
}

fn count_batch(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::PlusOne, 1);
    Ok(())
}

fn count_occurrence(g: &mut dyn Rules, item: &StackItem) -> Result<(), Suspend> {
    g.add_counters(item.object, CounterKind::Charge, 1);
    Ok(())
}

// The Ob Nixilis, Captive Kingpin and Mindcrank trigger shapes, with counters measuring firings rather than their effects.
static WITNESS: CardDef = CardDef {
    name: "Life Loss Witness",
    oracle: true,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        triggered(
            when_asked(EventMask::LIFE_CHANGE, opponent_lost_one),
            count_batch,
            "Whenever one or more opponents each lose exactly 1 life, put a +1/+1 counter on this artifact.",
        )
        .once_per_batch(),
        triggered(
            when_asked(EventMask::LIFE_CHANGE, opponent_lost_life),
            count_occurrence,
            "Whenever an opponent loses life, put a charge counter on this artifact.",
        ),
    ],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&WITNESS])))
}

fn play(card: &str, life: i32) {
    let text = format!(
        "
[Bot1 Deck]
{card}
Black Lotus x3
Life Loss Witness
Island x20

[Bot2 Deck]
Mana Confluence
Island x20

[Bot3 Deck]
Mana Confluence
Island x20

[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Battlefield.Black Lotus#1.Activate(Color: Black)
Bot1.Battlefield.Black Lotus#1.Activate(Color: Black)
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.{card}.Cast
[T2 Bot2.Main1]
Bot2.Hand.Mana Confluence.Play
[T3 Bot3.Main1]
Bot3.Hand.Mana Confluence.Play
[T4 Bot1.Main1]
Bot2.Battlefield.Mana Confluence.Activate(Color: Blue)
Bot3.Battlefield.Mana Confluence.Activate(Color: Blue)
Bot1.Hand.Life Loss Witness.Cast
[T5 Bot2.Main1]

[Expect]
Bot1.Life = {life}
Bot2.Life = 18
Bot3.Life = 18
Bot1.Battlefield.Life Loss Witness.Counters.PlusOnePlusOne = 1
Bot1.Battlefield.Life Loss Witness.Counters.Charge = 2
"
    );
    let script = GameScript::parse(&text, card).unwrap();
    if let Err(failure) = run_on(&script, table()) {
        panic!("{}", failure.report);
    }
}

#[test]
fn archfiend_loses_life_in_one_batch_with_two_occurrences() {
    play("Archfiend of Despair", 20);
}

#[test]
fn gul_dukat_loses_life_in_one_batch_with_two_occurrences() {
    play("Gul Dukat, Colonial Prefect", 22);
}
