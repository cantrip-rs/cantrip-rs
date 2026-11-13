use cantrip_api::*;
use cantrip_mechanics::{choose, statics, triggers};
use cantrip_oracle as oracle;

/// Pay {R}{R}{R}, or tap Rohgahh and the named creatures, then an opponent gains them (CR 608.2c, 611.2a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{R}{R}{R}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid { return Ok(()) }

    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| {
        id == item.object || (g.chars(id).is(CardType::Creature) && g.chars(id).name == "Kobolds of Kher Keep")
    }).collect();
    g.tap_together(&creatures);
    g.end_sentence();
    let opponents = g.state().opponents(item.controller).collect();
    let Some(player) = choose::player(g, item.controller, item.object, opponents, "gains control of those permanents", 0)? else { return Ok(()) };
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, EffectKind::Control(player), Duration::Indefinite);
        }
    });
    Ok(())
}

/// Creatures you control named Kobolds of Kher Keep get +2/+2 (CR 604.1).
fn named_kobold(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature) && q.controller(source) == q.controller(affected)
        && so_far.name == "Kobolds of Kher Keep"
}

static PAY_QUESTION: Question = Question::new(
    "Pay", "pay {cost}?", "If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them", "Pay {cost}", "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_UPKEEP, upkeep,
        "At the beginning of your upkeep, you may pay {R}{R}{R}. If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.")],
    statics: &[static_ability(Layer::PtModify, named_kobold, statics::plus_pt::<2, 2>,
        "Creatures you control named Kobolds of Kher Keep get +2/+2.")],
    ..oracle::ROHGAHH_OF_KHER_KEEP
};
