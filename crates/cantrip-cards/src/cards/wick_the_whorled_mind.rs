use cantrip_api::*;
use cantrip_mechanics::triggers::this_or_another_of_subtype_you_control_enters;
use cantrip_mechanics::{choose, effects, lands, select, tokens};
use cantrip_oracle as oracle;

/// A Snail, read through the words of the card whose text says it (CR 612).
fn snail(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Snail))
}

/// "Create a 1/1 black Snail creature token if you don't control a Snail. Otherwise, put a +1/+1 counter on a Snail you control"
/// (CR 701.7a, 122.1, 608.2c): decided as the ability resolves, the Snail a permanent of its controller's; with more than one the
/// counter goes on the one they choose (it is not targeted).
fn create_a_snail_or_put_a_counter_on_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let asker = Asker::new(item.object, item.controller);
    let snails: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| snail(g, &asker, id)).collect();
    if snails.is_empty() {
        tokens::create(g, item.controller, &tokens::SNAIL, 1)?;
        return Ok(());
    }
    let chosen = match snails[..] {
        [only] => Some(only),
        _ => {
            let purpose = Purpose::PutCounters(CounterKind::PlusOne, 1);
            choose::select(g, item.controller, item.object, purpose, snails, select::cards(select::exactly(1)).called("Snail"))?.first().copied()
        }
    };
    if let Some(chosen) = chosen {
        g.add_counters(chosen, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// "Wick deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the sacrificed creature's
/// power" (CR 120.3, 121.1, 608.2h): the power the Snail last had on the battlefield (the ruling), read from this activation's cost
/// (`Query::cost_objects`), pumps and counters included; a power of 0 or less is no damage and no cards.
fn damage_each_opponent_and_draw_cards_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let power = g.cost_objects(item).first().and_then(|&sacrificed| g.chars(sacrificed).power).unwrap_or(0).max(0) as u32;
    effects::damage_each_opponent(g, item, power);
    g.end_sentence();
    g.draw(item.controller, power);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, this_or_another_of_subtype_you_control_enters::<{ lands::sub(Subtype::Rat) }>),
            create_a_snail_or_put_a_counter_on_one,
            "Whenever Wick or another Rat you control enters, create a 1/1 black Snail creature token if you don't control a Snail. Otherwise, put a +1/+1 counter on a Snail you control.",
        ),
        ability(
            cost("{U}{B}{R}").and(sacrifice(1, snail)),
            damage_each_opponent_and_draw_cards_equal_to_its_power,
            "{U}{B}{R}, Sacrifice a Snail: Wick deals damage equal to the sacrificed creature's power to each opponent. Then draw cards equal to the sacrificed creature's power.",
        ),
    ],
    ..oracle::WICK_THE_WHORLED_MIND
};

pub static EXTRA: &[&CardDef] = &[&tokens::SNAIL];
