use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{choose_color, triggers};

/// "As this enchantment enters, choose a color. At the beginning of each opponent's upkeep, this enchantment deals X damage to that player, where X is the number of nontoken permanents of the chosen color they control. At the beginning of your upkeep, destroy this enchantment unless you sacrifice two Islands." (CR 614.1c).
fn as_enchantment_enters_choose_color_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    entering.chosen_color = Some(choose_color(g, entering.controller, entering.object)?);
    Ok(())
}

/// "At the beginning of each opponent's upkeep" (CR 603.2).
fn when_beginning_opponents_upkeep(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::StepBegan { step } = event else { return None };
    let subject = Target::Player(q.state().turn.active);
    if matches!(* step, Step::Upkeep)
        && q
            .state()
            .opponents(asker.controller)
            .collect::<Vec<PlayerId>>()
            .contains(&q.state().turn.active)
    {
        return Fired::about(subject);
    }
    None
}

/// "At the beginning of each opponent's upkeep, this enchantment deals X damage to that player, where X is the number of nontoken permanents of the chosen color they control." (CR 608.2, 120.3).
fn enchantment_deals_x_damage_player(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let amount_1 = {
        let q: &dyn Query = g;
        (q
            .state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.state().obj(id).token)
                    && q
                        .state()
                        .chosen_color(q.linked(item))
                        .is_some_and(|chosen| q.chars(id).colors.has(chosen))
                    && item.subject.player() == Some(q.controller(id))
            })
            .collect::<Vec<ObjectId>>()
            .len() as i64) as u32
    };
    let recipients_1 = item
        .subject
        .player()
        .into_iter()
        .map(Target::Player)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u32)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, amount_1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

/// "At the beginning of your upkeep, destroy this enchantment unless you sacrifice two Islands." (CR 118.1).
fn paid_3(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Island))
}

/// "At the beginning of your upkeep, destroy this enchantment unless you sacrifice two Islands." (CR 608.2, 701.8a, 118.1).
fn destroy_enchantment_unless_sacrifice_two(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_6: Vec<ObjectId> = Vec::new();
    let mut hoisted_5: Vec<ObjectId> = Vec::new();
    let paid_2 = ({
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Island))
            })
            .collect::<Vec<ObjectId>>()
    })
        .into_iter()
        .filter(|&o| {
            g.controller(o) == item.controller && g.can_sacrifice(item.controller, o)
        })
        .count() >= 2
        && g
            .may_pay(
                item.controller,
                item.object,
                item.object,
                sacrifice(2, paid_3),
                pay_question(g.words(item.object).subtype(Subtype::Island)),
            )?;
    g.end_sentence();
    if !(((paid_2 as i64) != 0)) {
        let objects_4 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
        let destroyed_4 = g.destroy_all(&objects_4);
        let destroyed_4 = destroyed_4.into_objects();
        g.end_sentence();
        hoisted_5.extend(destroyed_4.clone());
        hoisted_6
            .extend({
                let q: &dyn Query = g;
                objects_4
                    .iter()
                    .copied()
                    .filter(|&o| q.state().obj(o).became.is_some())
                    .collect::<Vec<ObjectId>>()
            });
    }
    Ok(())
}

fn pay_question(land: Subtype) -> &'static Question {
    macro_rules! question {
        ($name:ident, $ask:literal, $yes:literal) => {
            static $name: Question = Question::new("Pay", $ask, "", $yes, "Destroy {subject}").no_default();
        };
    }
    question!(PLAINS, "sacrifice two Plains, or destroy {subject}?", "Sacrifice two Plains");
    question!(ISLANDS, "sacrifice two Islands, or destroy {subject}?", "Sacrifice two Islands");
    question!(SWAMPS, "sacrifice two Swamps, or destroy {subject}?", "Sacrifice two Swamps");
    question!(MOUNTAINS, "sacrifice two Mountains, or destroy {subject}?", "Sacrifice two Mountains");
    question!(FORESTS, "sacrifice two Forests, or destroy {subject}?", "Sacrifice two Forests");
    match land {
        Subtype::Plains => &PLAINS,
        Subtype::Swamp => &SWAMPS,
        Subtype::Mountain => &MOUNTAINS,
        Subtype::Forest => &FORESTS,
        _ => &ISLANDS,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::STEP_BEGAN, when_beginning_opponents_upkeep),
            enchantment_deals_x_damage_player,
            "At the beginning of each opponent's upkeep, this enchantment deals X damage to that player, where X is the number of nontoken permanents of the chosen color they control.",
        ),
        triggered(
            triggers::YOUR_UPKEEP,
            destroy_enchantment_unless_sacrifice_two,
            "At the beginning of your upkeep, destroy this enchantment unless you sacrifice two Islands.",
        ),
    ],
    enters: Some(as_enchantment_enters_choose_color_enters),
    ..oracle::PSYCHIC_ALLERGY
};
