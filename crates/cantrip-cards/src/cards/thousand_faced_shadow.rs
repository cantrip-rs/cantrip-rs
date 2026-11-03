use cantrip_api::*;
use cantrip_mechanics::{ninjutsu, targets};
use cantrip_oracle as oracle;

/// "When this creature enters from your hand" (CR 603.2).
fn when_creature_enters_hand(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::ZoneChange { new, from, to, .. } = event else { return None };
    let id = *new;
    let subject = Target::Object(id);
    if to.is(ZoneKind::Battlefield) && from.is(ZoneKind::Hand)
        && (asker.source == id && asker.controller == q.state().obj(id).owner)
        && subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                q.state().attackers.iter().any(|a| a.id == candidate)
            })
    {
        return Fired::about(subject);
    }
    None
}

/// "When this creature enters from your hand, if it's attacking, create a token that's a copy of another target attacking creature. The token enters tapped and attacking." (CR 608.2, 111.1).
fn if_attacking_create_token_thats(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if !{
        let q: &dyn Query = g;
        item.subject
            .object()
            .is_some_and(|candidate: ObjectId| {
                q.state().attackers.iter().any(|a| a.id == candidate)
                    || q.was_attacking(candidate)
            })
    } {
        return Ok(());
    }
    let t_0 = vec![g.state().target(item, 0)];
    let original_0 = t_0.get(0).copied().and_then(|t: Target| t.object());
    let Some(original_0) = original_0 else { return Ok(()) };
    let values_0 = {
        let q: &dyn Query = g;
        {
            let original = original_0;
            q.copiable_values(original)
        }
    };
    g.create_token_copies_attacking(values_0, item.controller, 1, true, None)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ninjutsu::ninjutsu(mana("{2}{U}{U}"), "Ninjutsu {2}{U}{U}"),
        triggered(
                when_asked(EventMask::ZONE_CHANGE, when_creature_enters_hand),
                if_attacking_create_token_thats,
                "When this creature enters from your hand, if it's attacking, create a token that's a copy of another target attacking creature. The token enters tapped and attacking.",
            )
            .targets(&[targets::ANOTHER_ATTACKING_CREATURE]),
    ],
    ..oracle::THOUSAND_FACED_SHADOW
};
