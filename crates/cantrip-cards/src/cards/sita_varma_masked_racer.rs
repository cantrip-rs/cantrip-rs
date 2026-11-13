use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::targets::power_or_last_known;
use cantrip_oracle as oracle;

const ASK: Question = Question::new(
    "BasePowerToughnessEqualToPower",
    "have the base power and toughness of each other creature you control become equal to Sita Varma's power until end of turn?",
    "Chosen as Sita Varma's exhaust ability resolves",
    "Set base power and toughness",
    "Don't",
);

/// "Put X +1/+1 counters on Sita Varma. Then you may have the base power and toughness of each other creature you control become
/// equal to Sita Varma's power until end of turn" (CR 122.1, 613.4b, 611.2c, 608.2h): X is what was paid. The counters go on it
/// only while it is on the battlefield (a new object if it left, CR 400.7). Its power is read once, after the counters, as it
/// last existed on the battlefield if it has gone, and each other creature the controller controls at that moment becomes that
/// size in layer 7b until the cleanup step, so counters and pumps on those creatures still count on top; no creature that
/// enters later does.
fn counters_then_base_pt_equal_to_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.is_on_battlefield(item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, item.x as i32);
    }
    g.end_sentence();
    if ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        let power = power_or_last_known(g, item.object);
        let others: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| id != item.object && g.chars(id).is(CardType::Creature)).collect();
        instruction(g, |g| {
            for id in others {
                g.add_effect(item.object, id, EffectKind::SetPt(power, power), Duration::EndOfTurn);
            }
        });
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{X}{G}{G}{U}"), counters_then_base_pt_equal_to_power, "Exhaust — {X}{G}{G}{U}: Put X +1/+1 counters on Sita Varma. Then you may have the base power and toughness of each other creature you control become equal to Sita Varma's power until end of turn.").exhaust()],
    ..oracle::SITA_VARMA_MASKED_RACER
};
