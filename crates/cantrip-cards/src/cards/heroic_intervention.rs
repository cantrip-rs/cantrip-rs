use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Permanents you control gain hexproof and indestructible until end of turn." (CR 608.2, 611.2a).
fn permanents_control_gain_hexproof_indestructible(
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
                (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Battle)
                    || q.chars(id).is(CardType::Creature)
                    || q.chars(id).is(CardType::Enchantment)
                    || q.chars(id).is(CardType::Land)
                    || q.chars(id).is(CardType::Planeswalker))
                    && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Hexproof]));
    let kind_0_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Indestructible]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
                g.add_effect(item.object, object, kind_0_1_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], permanents_control_gain_hexproof_indestructible),
    ..oracle::HEROIC_INTERVENTION
};
