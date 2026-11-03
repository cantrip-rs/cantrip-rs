use cantrip_api::*;
use cantrip_mechanics::{tokens, triggers};
use cantrip_oracle as oracle;

/// "When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time. When Hazezon leaves the battlefield, exile all Sand Warriors." (CR 608.2).
fn delayed_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let count = g.permanents_of(item.controller).into_iter()
        .filter(|&id| g.chars(id).is(CardType::Land))
        .count() as u32;
    tokens::create(g, item.controller, &tokens::SAND_WARRIOR, count)?;
    Ok(())
}

/// "When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time." (CR 608.2).
fn create_x_1_1_sand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::Upkeep,
            on_turn_of: Some(item.controller),
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// "When Hazezon leaves the battlefield, exile all Sand Warriors." (CR 608.2, 701.13).
fn exile_all_sand_warriors(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_4 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Sand))
                    && q
                        .chars(id)
                        .has_subtype(q.words(item.object).subtype(Subtype::Warrior))
            })
            .collect::<Vec<ObjectId>>()
    };
    g.exile_all(&objects_4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            create_x_1_1_sand,
            "When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time.",
        ),
        triggered(
            triggers::THIS_LEAVES,
            exile_all_sand_warriors,
            "When Hazezon leaves the battlefield, exile all Sand Warriors.",
        ),
    ],
    delayed: &[
        delayed_trigger(
            delayed_1,
            "When Hazezon enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time.",
        ),
    ],
    ..oracle::HAZEZON_TAMAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::SAND_WARRIOR];
