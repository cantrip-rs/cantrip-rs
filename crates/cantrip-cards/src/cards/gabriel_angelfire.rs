use cantrip_api::*;
use cantrip_mechanics::{choose, rampage, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3. Gabriel Angelfire gains that ability until your next upkeep." (CR 608.2, 611.2a).
fn choose_flying_first_strike_trample(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let chosen_0 = choose::option(
        g,
        item.controller,
        item.object,
        item.ability.card,
        0..4,
    )? - 0;
    g.end_sentence();
    let objects_1 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let duration_1 = Duration::UntilNextStepOf {
        step: Step::Upkeep,
        player: Some(item.controller),
    };
    let kind_1_0_0 = {
        match chosen_0 {
            0 => EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
            1 => EffectKind::AddKeywords(Keywords::of(&[Keyword::FirstStrike])),
            2 => EffectKind::AddKeywords(Keywords::of(&[Keyword::Trample])),
            _ => {
                EffectKind::AddAbility(
                    AbilityRef::delayed(item.ability.card, 0),
                )
            }
        }
    };
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, duration_1);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            choose_flying_first_strike_trample,
            "At the beginning of your upkeep, choose flying, first strike, trample, or rampage 3. Gabriel Angelfire gains that ability until your next upkeep.",
        ),
    ],
    delayed: &[rampage::trigger::<3>("Rampage 3")],
    labels: &["flying", "first strike", "trample", "rampage 3"],
    ..oracle::GABRIEL_ANGELFIRE
};
