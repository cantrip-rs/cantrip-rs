use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

const MILL_TWO: Question = Question::new(
    "Mill", "mill two cards?", "If you don't, sacrifice this creature", "Mill two", "Don't mill",
);

/// "At the beginning of your upkeep, sacrifice this creature unless you mill two cards." (CR 608.2, 701.21a, 701.17a).
fn sacrifice_creature_unless_mill_two(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let (paid_0, _, _, _): (bool, Vec<ObjectId>, Vec<ObjectId>, Vec<ObjectId>) = if g
        .state()
        .zone(Zone::library(item.controller))
        .len() >= 2
        && effects::ask_may(g, item.controller, item.object, Purpose::Ask(&MILL_TWO))?
    {
        let mills_2: Vec<(PlayerId, u32)> = Some(item.controller)
            .into_iter()
            .map(|p| (p, 2u32))
            .collect();
        let milled_2 = g.mill_together(&mills_2);
        (true, milled_2.clone(), milled_2.clone().clone(), Vec::new())
    } else {
        (false, Vec::new(), Vec::new(), Vec::new())
    };
    g.end_sentence();
    if !(((paid_0 as i64) != 0)) {
        g.sacrifice(item.controller, &[item.object]);
        g.end_sentence();
    }
    Ok(())
}

/// "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap this creature." (CR 608.2, 611.2a, 701.26a).
fn creature_gains_shroud_doesnt_untap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_6 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_6_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Shroud]));
    instruction(
        g,
        |g| {
            for &object in &objects_6 {
                g.add_effect(item.object, object, kind_6_0_0, Duration::EndOfTurn);
            }
        },
    );
    g.end_sentence();
    let objects_7 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    let kinds_7 = {
        let mut all = Vec::new();
        for &object in &objects_7 {
            all.push((object, EffectKind::DoesntUntap));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_7 {
                g.add_effect(
                    item.object,
                    object,
                    kind,
                    Duration::DuringNextUntapStepOf(item.controller),
                );
            }
        },
    );
    g.end_sentence();
    let objects_8 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    g.tap_together(&objects_8);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_creature_unless_mill_two,
            "At the beginning of your upkeep, sacrifice this creature unless you mill two cards.",
        ),
        ability(
            cost("{U}"),
            creature_gains_shroud_doesnt_untap,
            "{U}: This creature gains shroud until end of turn and doesn't untap during your next untap step. Tap this creature.",
        ),
    ],
    ..oracle::DEEP_SPAWN
};
