use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Choose one or both — • Deafening Clarion deals 3 damage to each creature. • Creatures you control gain lifelink until end of turn." (CR 608.2, 120.3).
fn choose_one_both_deafening_clarion_mode_0(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let recipients_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| q.chars(id).is(CardType::Creature))
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .map(Target::Object)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u16)> = recipients_0
        .iter()
        .map(|t| (item.object, *t, 3u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "Choose one or both — • Deafening Clarion deals 3 damage to each creature. • Creatures you control gain lifelink until end of turn." (CR 608.2, 611.2a).
fn choose_one_both_deafening_clarion_mode_1(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature) && item.controller == q.controller(id)
            })
            .collect::<Vec<ObjectId>>()
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let kind_1_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Lifelink]));
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

static ABILITY_0_MODES: ModalDef = modes(
        &[
            mode(
                choose_one_both_deafening_clarion_mode_0,
                "Deafening Clarion deals 3 damage to each creature.",
            ),
            mode(
                choose_one_both_deafening_clarion_mode_1,
                "Creatures you control gain lifelink until end of turn.",
            ),
        ],
    )
    .choose(1, 2);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&ABILITY_0_MODES),
    ..oracle::DEAFENING_CLARION
};
