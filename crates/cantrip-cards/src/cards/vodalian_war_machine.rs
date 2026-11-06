use cantrip_api::*;
use cantrip_oracle as oracle;
use cantrip_mechanics::{behold, effects, triggers};

/// "Tap an untapped Merfolk you control: This creature can attack this turn as though it didn't have defender." (CR 608.2, 611.2a).
fn creature_can_attack_as_though(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::CanAttackDespiteDefender,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    Ok(())
}

/// An untapped Merfolk can pay either ability (CR 118.3).
fn untapped_merfolk(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !q.state().obj(id).tapped && behold::merfolk(q, asker, id)
}

/// "When this creature dies, destroy all Merfolk tapped this turn to pay for its abilities." (CR 608.2, 701.8a).
fn destroy_all_merfolk_tapped_pay(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_4 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Merfolk))
                    && Some(item.object)
                        .is_some_and(|of: ObjectId| {
                            let s = q.state();
                            let began = q
                                .history()
                                .this_turn()
                                .first()
                                .map_or(u32::MAX, |r| r.serial);
                            s.stack
                                .iter()
                                .chain(s.left_stack.iter())
                                .any(|i| {
                                    i.kind == StackKind::Activated && i.since >= began
                                        && (i.object == of || q.obj(i.object).became == Some(of))
                                        && q.cost_objects(i).contains(&id)
                                })
                        })
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            tap_untapped(1, untapped_merfolk),
            creature_can_attack_as_though,
            "Tap an untapped Merfolk you control: This creature can attack this turn as though it didn't have defender.",
        ),
        ability(
            tap_untapped(1, untapped_merfolk),
            effects::pump_self::<2, 1>,
            "Tap an untapped Merfolk you control: This creature gets +2/+1 until end of turn.",
        ),
        triggered(
            triggers::THIS_DIES,
            destroy_all_merfolk_tapped_pay,
            "When this creature dies, destroy all Merfolk tapped this turn to pay for its abilities.",
        ),
    ],
    ..oracle::VODALIAN_WAR_MACHINE
};
