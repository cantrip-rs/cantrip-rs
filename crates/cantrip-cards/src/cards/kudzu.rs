use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets, triggers::ATTACHED_PERMANENT_BECOMES_TAPPED};
use cantrip_oracle as oracle;

/// "When enchanted land becomes tapped, destroy it. That land's controller may attach this Aura to a land of their choice." (CR 608.2, 701.8a, 701.3a).
fn destroy_lands_controller_attach_aura(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut hoisted_3: Vec<ObjectId> = Vec::new();
    let objects_0 = item.subject.object().into_iter().collect::<Vec<ObjectId>>();
    g.destroy_all(&objects_0);
    g.end_sentence();
    {
        let selected_1_player = {
            let q: &dyn Query = g;
            let Some(named_1) = item.subject.object().map(|o: ObjectId| q.controller(o)) else { return Ok(()) };
            named_1
        };
        let selected_1_candidates = {
            let attachment = {
                let q: &dyn Query = g;
                Some(item.object).filter(|&a| q.is_on_battlefield(a))
            };
            let q: &dyn Query = g;
            ({
                let q: &dyn Query = g;
                q.state()
                    .battlefield()
                    .iter()
                    .copied()
                    .filter(|&id| q.chars(id).is(CardType::Land))
                    .collect::<Vec<ObjectId>>()
            })
                .into_iter()
                .filter(|&host| {
                    attachment.is_some_and(|a| is_live(q, a) && q.can_attach_to(a, host))
                })
                .collect::<Vec<ObjectId>>()
        };
        let selected_1 = choose::select(
            g,
            selected_1_player,
            item.object,
            Purpose::Choose,
            selected_1_candidates,
            select::land_cards(select::up_to(1)),
        )?;
        if !selected_1.is_empty() {
            let attachments_2 = {
                let q: &dyn Query = g;
                Some(item.object)
                    .into_iter()
                    .filter(|&o| q.state().obj(o).became.is_none())
                    .collect::<Vec<ObjectId>>()
            };
            let host = selected_1[0];
            instruction(
                g,
                |g| {
                    for attachment in attachments_2 {
                        g.attach(attachment, host);
                    }
                },
            );
            g.end_sentence();
            hoisted_3.extend(selected_1.clone());
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            ATTACHED_PERMANENT_BECOMES_TAPPED,
            destroy_lands_controller_attach_aura,
            "When enchanted land becomes tapped, destroy it. That land's controller may attach this Aura to a land of their choice.",
        ),
    ],
    enchant: Some(targets::LAND),
    ..oracle::KUDZU
};
