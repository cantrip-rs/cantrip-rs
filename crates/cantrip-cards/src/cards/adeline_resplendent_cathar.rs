use cantrip_api::*;
use cantrip_mechanics::combat::{choose_among, player_or_their_planeswalkers};
use cantrip_mechanics::{statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "For each opponent, create a 1/1 white Human creature token that's tapped and attacking that player or a planeswalker
/// they control" (CR 508.4): its controller chooses which, asked only when the opponent controls a planeswalker. The Humans are
/// created together (CR 101.4), and a further Human a replacement makes for an opponent may attack that player or a
/// planeswalker they control too.
fn human_for_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let human = g.card_id(&tokens::HUMAN);
    let opponents: Vec<PlayerId> = g.state().opponents(item.controller).collect();
    let mut entries: Vec<EntersAttacking> = Vec::new();
    for opponent in opponents {
        let others = player_or_their_planeswalkers(g, opponent);
        if let Some(defender) = choose_among(g, item.controller, item.object, &others)? {
            entries.push(EntersAttacking { defender, others });
        }
    }
    g.create_tokens_attacking_each(human, item.controller, true, &entries)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[characteristic_defining(Layer::PtDefine, statics::power_is_creatures_you_control, "Adeline's power is equal to the number of creatures you control.").lost_pt(0, 4)],
    abilities: &[triggered(triggers::YOU_ATTACK, human_for_each_opponent, "Whenever you attack, for each opponent, create a 1/1 white Human creature token that's tapped and attacking that player or a planeswalker they control.")],
    ..oracle::ADELINE_RESPLENDENT_CATHAR
};

pub static EXTRA: &[&CardDef] = &[&tokens::HUMAN];
