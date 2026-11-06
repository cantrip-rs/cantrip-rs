use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Life: "All lands you control become 1/1 creatures until end of turn. They're still lands."
fn life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let lands: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Land)).collect();
    instruction(g, |g| {
        for land in lands {
            g.add_effect(item.object, land, EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])), Duration::EndOfTurn);
            g.add_effect(item.object, land, EffectKind::SetPt(1, 1), Duration::EndOfTurn);
        }
    });
    Ok(())
}

fn creature_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
}

/// Death: "Return target creature card from your graveyard to the battlefield. You lose life equal to its mana value."
fn death(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    let value = g.mana_value(card) as i32;
    g.put_onto_battlefield(card, item.controller, false)?;
    g.lose_life(item.controller,(value) as i32);
    Ok(())
}

pub static LIFE: CardDef = CardDef { spell: spell(&[], life), ..oracle::LIFE };

pub static DEATH: CardDef = CardDef {
    spell: spell(&[target(creature_card_in_your_graveyard)], death),
    ..oracle::DEATH
};

pub static CARD: CardDef = CardDef { split: Some((&LIFE, &DEATH)), ..oracle::LIFE_DEATH };

pub static EXTRA: &[&CardDef] = &[&LIFE, &DEATH];
