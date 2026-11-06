use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "All creatures lose flying until end of turn. Sacrifice a snow land: Return this card from your graveyard to your hand." (CR 608.2, 611.2a).
fn return_card_graveyard_hand(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| q.chars(id).is(CardType::Creature))
            .collect::<Vec<ObjectId>>()
    };
    let kind_0_0_0 = EffectKind::LoseKeywords(Keywords::of(&[Keyword::Flying]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "Sacrifice a snow land: Return this card from your graveyard to your hand." (CR 608.2).
fn return_card_graveyard_hand_2(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    q.chars(id).supertypes.snow && q.chars(id).is(CardType::Land)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], return_card_graveyard_hand),
    abilities: &[
        ability(
                sacrifice(1, return_card_graveyard_hand_2),
                effects::return_this_to_hand,
                "Sacrifice a snow land: Return this card from your graveyard to your hand.",
            )
            .from(ZoneKind::Graveyard),
    ],
    ..oracle::WHITEOUT
};
