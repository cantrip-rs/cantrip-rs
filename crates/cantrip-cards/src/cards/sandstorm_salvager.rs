use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn golem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::GOLEM, 1)?;
    Ok(())
}

/// Put a +1/+1 counter on each creature token you control; they gain trample until end of turn. The tokens are
/// the ones there as the ability resolves (CR 611.2c).
fn grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let tokens: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| g.obj(id).token && g.chars(id).is(CardType::Creature))
        .collect();
    let puts: Vec<(ObjectId, CounterKind, i64)> = tokens.iter().map(|&token| (token, CounterKind::PlusOne, 1)).collect();
    g.add_counters_together(&puts);
    g.end_sentence();
    instruction(g, |g| {
        for &token in &tokens {
            cantrip_mechanics::trample_until_end_of_turn(g, item.object, token);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, golem, "When Sandstorm Salvager enters, create a 3/3 colorless Golem artifact creature token."),
        ability(cost("{2}, {T}"), grow, "{2}, {T}: Put a +1/+1 counter on each creature token you control. They gain trample until end of turn."),
    ],
    ..oracle::SANDSTORM_SALVAGER
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOLEM];
