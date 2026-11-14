use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

fn itself(_: &dyn LayerQuery, source: ObjectId, object: ObjectId) -> bool { source == object }

/// Its printed abilities belong to the Tetravite, so replacement tokens do not inherit them (CR 614.6).
pub static TETRAVITE: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(1), toughness: Some(1),
    keywords: Keywords::of(&[Keyword::Flying]),
    text: "Flying\nThis token can't be enchanted.",
    targeting_rules: &[TargetingRule { kind: TargetingRuleKind::NoOtherAuras, applies: itself }],
    ..token!(Tetravite)
};

const REMOVE: Question = Question::new("RemoveCounters", "remove counters from {source}?", "", "Remove counters", "Decline");
const EXILE: Question = Question::new("ExileTokens", "exile tokens created with {source}?", "", "Exile tokens", "Decline");

/// The number removed is the cost paid, including zero (CR 118.11, 118.12).
fn make_tokens(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cost = Cost { variable: Some(VariableCost::RemoveAnyNumberOfCounters(CounterKind::PlusOne)), ..Cost::NONE };
    if let Some(n) = g.may_pay_x(item.controller, item.object, item.object, cost, &REMOVE)? {
        g.end_sentence();
        let card = g.card_id(&TETRAVITE);
        g.create_tokens_worded(&vec![(card, item.controller); n as usize], false, None, g.words(item.object))?;
    }
    Ok(())
}

fn linked_tokens(q: &dyn Query, asker: &Asker) -> Vec<ObjectId> {
    let by = q.linked_to(asker);
    let creator = AbilityRef { card: q.card_id(&CARD), index: 0 };
    q.history().tokens_created_by(by, creator).filter(|&id| q.state().battlefield().contains(&id)).collect()
}
fn any_tokens(_: &dyn Query, _: &Asker, _: &[ObjectId]) -> bool { true }
static EXILE_SELECTION: ExileSelection = ExileSelection { candidates: linked_tokens, valid: any_tokens };

/// The linked creation ability's original tokens, irrespective of their controller (CR 607.2c, 607.5).
fn reclaim(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cost = Cost { variable: Some(VariableCost::ExileAnyNumber(&EXILE_SELECTION)), ..Cost::NONE };
    if let Some(n) = g.may_pay_counted(item.controller, item.object, item.object, cost, &EXILE)? {
        g.end_sentence();
        // Each eligible token has a zone-change record and a provenance record. Within the
        // native u32 history-serial domain their count fits the i32 counter-placement domain.
        g.add_counters(item.object, CounterKind::PlusOne, n as i32);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(effects::enters_with_plus_one_counters::<3>),
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, make_tokens, "At the beginning of your upkeep, you may remove any number of +1/+1 counters from this creature. If you do, create that many 1/1 colorless Tetravite artifact creature tokens. They each have flying and \"This token can't be enchanted.\""),
        triggered(triggers::YOUR_UPKEEP, reclaim, "At the beginning of your upkeep, you may exile any number of tokens created with this creature. If you do, put that many +1/+1 counters on this creature."),
    ],
    ..oracle::TETRAVUS
};
pub static EXTRA: &[&CardDef] = &[&TETRAVITE];
