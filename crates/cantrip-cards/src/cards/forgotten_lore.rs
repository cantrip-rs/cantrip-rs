use cantrip_api::*;
use cantrip_mechanics::{choose, select, targets};
use cantrip_oracle as oracle;

/// "Target opponent chooses a card in your graveyard. You may pay {G}. If you do, repeat this process except that opponent can't choose a card already chosen for Forgotten Lore. Then put the last chosen card into your hand." (CR 608.2, 400.7, 118.1).
fn target_opponent_chooses_card_in(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let mut last_chosen_1: Vec<ObjectId> = Vec::new();
    let mut previously_chosen_1: Vec<ObjectId> = Vec::new();
    let t_0 = vec![g.state().target(item, 0)];
    loop {
        let mut again_0 = false;
        let Some(chooser_1) = t_0.get(0).copied().and_then(|t: Target| t.player()) else { return Ok(()) };
        let candidates_1: Vec<ObjectId> = ({
            let q: &dyn Query = g;
            Some(item.controller)
                .into_iter()
                .flat_map(|p: PlayerId| {
                    q.state().zone(Zone::graveyard(p)).iter().rev().copied()
                })
                .collect::<Vec<ObjectId>>()
                .into_iter()
                .filter(|&id| !q.state().obj(id).token && !q.state().obj(id).card_copy)
                .collect::<Vec<ObjectId>>()
        })
            .into_iter()
            .filter(|id| !previously_chosen_1.contains(id))
            .collect();
        let picked_1 = choose::select(
            g,
            chooser_1,
            item.object,
            Purpose::Choose,
            candidates_1,
            select::Selection::of(select::Count::Exactly(1), &[]).called("card"),
        )?;
        if !picked_1.is_empty() {
            previously_chosen_1.extend(picked_1.iter().copied());
            last_chosen_1 = picked_1;
        }
        g.end_sentence();
        let paid_2 = g
            .may_pay(
                item.controller,
                item.object,
                item.object,
                cost("{G}"),
                &PAY_QUESTION,
            )?;
        g.end_sentence();
        if (paid_2 as i64) != 0 {
            again_0 = true;
            g.end_sentence();
        }
        g.end_sentence();
        if !again_0 {
            break;
        }
    }
    g.end_sentence();
    let objects_4 = last_chosen_1
        .clone()
        .first()
        .copied()
        .into_iter()
        .collect::<Vec<ObjectId>>();
    g.return_to_hand(&objects_4);
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
    spell: spell(
        &[targets::OPPONENT],
        target_opponent_chooses_card_in,
    ),
    ..oracle::FORGOTTEN_LORE
};
