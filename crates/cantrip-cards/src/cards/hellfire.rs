use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you, where X is the number of creatures that died this way." (CR 608.2, 120.3, 701.8a).
fn destroy_all_nonblack_creatures_hellfire(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let nonblack_creatures = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.chars(id).colors.has(q.words(item.object).color(Color::Black)))
                    && q.chars(id).is(CardType::Creature)
            })
            .collect::<Vec<ObjectId>>()
    };
    let destroyed = g.destroy_all(&nonblack_creatures);
    g.end_sentence();
    let died = cantrip_mechanics::effects::put_into_a_graveyard(g, &destroyed).len() as u32;
    g.damage_together(&[(item.object, Target::Player(item.controller), died + 3)]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], destroy_all_nonblack_creatures_hellfire),
    ..oracle::HELLFIRE
};
