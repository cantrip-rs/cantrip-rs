use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}." (CR 608.2, 120.3, 118.1).
fn player_enchantment_deals_1_damage(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let players_0: Vec<PlayerId> = {
        let q: &dyn Query = g;
        q.state().apnap().collect::<Vec<PlayerId>>()
    };
    let mut each_2 = vec![Default::default(); players_0.len()];
    for (index_0, bound_0) in players_0.iter().copied().enumerate() {
        let payer_1 = bound_0;
        let paid_1 = g
            .may_pay(payer_1, item.object, item.object, cost("{B}"), &PAY_BLACK)?
            || g.may_pay(payer_1, item.object, item.object, cost("{3}"), &PAY_THREE)?;
        each_2[index_0] = paid_1;
    }
    g.end_sentence();
    let recipients_3 = {
        let q: &dyn Query = g;
        ({
            let mut all = q.state().apnap().collect::<Vec<PlayerId>>();
            let other = q
                .state()
                .apnap()
                .filter(|&bound_0: &PlayerId| {
                    !(Some(bound_0)
                        .and_then(|p: PlayerId| {
                            players_0.iter().position(|&next| next == p)
                        })
                        .is_some_and(|at| (each_2[at] as i64) != 0))
                })
                .collect::<Vec<PlayerId>>();
            all.retain(|p: &PlayerId| other.contains(p));
            all
        })
            .into_iter()
            .map(Target::Player)
            .collect::<Vec<Target>>()
    };
    let batch: Vec<(ObjectId, Target, u32)> = recipients_3
        .iter()
        .map(|t| (item.object, *t, 1))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

static PAY_BLACK: Question = Question::new(
    "Pay",
    "pay {cost}, or choose whether to pay {3} instead?",
    "",
    "Pay {cost}",
    "Consider paying {3}",
);

static PAY_THREE: Question = Question::new(
    "Pay",
    "pay {cost}, or take 1 damage?",
    "",
    "Pay {cost}",
    "Take 1 damage",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            player_enchantment_deals_1_damage,
            "At the beginning of your upkeep, for each player, this enchantment deals 1 damage to that player unless they pay {B} or {3}.",
        ),
    ],
    ..oracle::LIM_D_LS_HEX
};
