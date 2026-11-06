use cantrip_api::*;
use cantrip_mechanics::pump;
use cantrip_oracle as oracle;

/// "Green creatures you control get +1/+1 until end of turn. Nongreen creatures you control get +1/+0 until end of turn." (CR 608.2, 611.2a).
fn green_creatures_control_get_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).colors.has(q.words(item.object).color(Color::Green))
                    && q.chars(id).is(CardType::Creature)
                    && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                pump(g, item.object, object, 1, 1);
            }
        },
    );
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.chars(id).colors.has(q.words(item.object).color(Color::Green)))
                    && q.chars(id).is(CardType::Creature)
                    && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
    };
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                pump(g, item.object, object, 1, 0);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], green_creatures_control_get_1),
    ..oracle::BATTLE_FRENZY
};
