use cantrip_api::*;
use cantrip_mechanics::{combat, targets, triggers};
use cantrip_oracle as oracle;

/// "Whenever a player activates an ability of enchanted creature with {T} in its activation cost that isn't a mana ability" (CR 603.2).
fn when_player_activates_ability_enchanted_creature(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::AbilityActivated { source, ability, player, .. } = event else { return None };
    let def = q.activated_ability(*ability)?;
    if q.attached_to(asker.source) != Some(*source) || !def.cost.tap || def.is_mana_ability() {
        return None;
    }
    let Some(Target::Ability(serial)) = triggers::activated_ability_on_stack(q, event) else { return None };
    Some(Fired { subject: Target::Player(*player), data: [source.0, serial] })
}

/// "Whenever a player activates an ability of enchanted creature with {T} in its activation cost that isn't a mana ability, you may pay {1}. If you do, counter that ability. If you don't, destroy this Aura." (CR 608.2, 701.8a, 701.6a).
fn pay_1_if_do_counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid {
        let items: Vec<_> = g.state().stack.iter().enumerate()
            .filter(|(_, entry)| entry.serial == item.data[1])
            .map(|(index, entry)| (entry.object, index))
            .collect();
        g.counter_items(&items);
    } else {
        g.destroy(item.object);
    }

    Ok(())
}

/// "Whenever enchanted creature attacks or blocks" (CR 603.2).
fn when_enchanted_creature_attacks_blocks(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    if let Event::Attacks { attacker, .. } = event {
        let id = *attacker;
        let subject = Target::Object(id);
        if Some(asker.source).and_then(|o: ObjectId| q.attached_to(o)) == Some(id) {
            return Fired::about(subject);
        }
    }
    let Event::Blocks { blocker, declared: true, .. } = event else { return None };
    let id = *blocker;
    let subject = Target::Object(id);
    if Some(asker.source).and_then(|o: ObjectId| q.attached_to(o)) == Some(id) {
        return Fired::about(subject);
    }
    None
}

/// "Whenever enchanted creature attacks or blocks, you may pay {1}. If you do, tap the creature, remove it from combat, and creatures it was blocking that had become blocked by only that creature this combat become unblocked. If you don't, destroy this Aura." (CR 608.2, 701.8a, 701.26a).
fn pay_1_if_do_tap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{1}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid {
        let Some(creature) = item.subject.object() else { return Ok(()) };
        g.tap(creature);
        g.end_sentence();
        if is_live(g, creature) {
            combat::remove_from_combat_and_unblock(g, creature);
        }
    } else {
        g.destroy(item.object);
    }

    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::ABILITY_ACTIVATED,
                when_player_activates_ability_enchanted_creature,
            ),
            pay_1_if_do_counter,
            "Whenever a player activates an ability of enchanted creature with {T} in its activation cost that isn't a mana ability, you may pay {1}. If you do, counter that ability. If you don't, destroy this Aura.",
        ),
        triggered(
            when_asked(
                EventMask(EventMask::ATTACKS.0 | EventMask::BLOCKS.0),
                when_enchanted_creature_attacks_blocks,
            ),
            pay_1_if_do_tap,
            "Whenever enchanted creature attacks or blocks, you may pay {1}. If you do, tap the creature, remove it from combat, and creatures it was blocking that had become blocked by only that creature this combat become unblocked. If you don't, destroy this Aura.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::IMPRISON
};
