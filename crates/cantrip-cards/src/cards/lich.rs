use cantrip_api::*;
use cantrip_mechanics::{choose, select, statics, triggers};
use cantrip_oracle as oracle;

/// "As this enchantment enters, you lose life equal to your life total. You don't lose the game for having 0 or less life. If you would gain life, draw that many cards instead. Whenever you're dealt damage, sacrifice that many nontoken permanents. If you can't, you lose the game. When this enchantment is put into a graveyard from the battlefield, you lose the game." (CR 614.1c, 119.3).
fn as_enchantment_enters_lose_life_enters(
    g: &mut dyn Game,
    entering: &mut Entering,
) -> Result<(), Suspend> {
    let life = g.state().player(entering.controller).life.max(0);
    g.lose_life(entering.controller,(life) as i32);
    Ok(())
}

/// "If you would gain life, draw that many cards instead." (CR 604.1).
fn if_would_gain_life_draw_applies(
    _: &dyn Query,
    asker: &Asker,
    proposal: &Proposal,
) -> bool {
    let Proposal::LifeChange(l) = proposal else { return false };
    if l.amount > 0 && asker.controller == l.player {
        return true;
    }
    false
}

/// "If you would gain life, draw that many cards instead." (CR 604.1).
fn if_would_gain_life_draw_apply(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    let seen = *proposal;
    let Proposal::LifeChange(_) = &seen else { return };
    let Proposal::LifeChange(l) = proposal else { return };
    l.amount = 0;
}

/// "If you would gain life, draw that many cards instead." (CR 608.2, 121.1).
fn if_would_gain_life_draw_then_steps(
    g: &mut dyn Game,
    asker: &Asker,
    _: Target,
    data: [u32; 2],
) -> Result<(), Suspend> {
    let amount_1 = i64::from(data[0]) as u32;
    g.draw(asker.controller, amount_1);
    Ok(())
}

/// "If you would gain life, draw that many cards instead." (CR 608.2).
fn if_would_gain_life_draw_then(
    g: &mut dyn Game,
    asker: &Asker,
    before: &Proposal,
    _: &Proposal,
) {
    let Proposal::LifeChange(l) = before else { return };
    let amount = l.amount.unsigned_abs();
    let subject = Target::Player(l.player);
    let data = [u32::try_from(amount).expect("draw quantity exceeds existing u32 instruction domain"), u32::MAX];
    let _ = if_would_gain_life_draw_then_steps(g, asker, subject, data);
}

/// "Whenever you're dealt damage" (CR 603.2).
fn when_youre_dealt_damage(
    _: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::Damage { target, .. } = event else { return None };
    let subject = *target;
    let data = [
        0,
        match target {
            Target::Player(p) => u32::from(p.0),
            _ => u32::MAX,
        },
    ];
    if matches!(* target, Target::Player(p) if asker.controller == p) {
        return Some(Fired { subject, data });
    }
    None
}

/// The permanents chosen for Lich's sacrifice must be nontokens (CR 111.6).
fn nontoken(q: &dyn Query, choice: &Choice, _: u32) -> Verdict {
    Verdict::each(choice, |id| !q.obj(id).token)
}

/// Sacrifice as many nontoken permanents as the damage dealt, then lose if fewer were sacrificed (CR 701.21a).
fn sacrifice_many_nontoken_permanents_if(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owed = item.matched_damage_amount();
    let available = g.permanents_of(item.controller).len();
    let selectable = owed.min(available.min(u32::MAX as usize) as u32);
    let rule = Selection::new(select::as_many_as_can(selectable))
        .with(select::part(&(nontoken as SelectFn), 0, ""))
        .called("nontoken permanent");
    let permanents = g.permanents_of(item.controller);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Sacrifice, permanents, rule)?;
    g.sacrifice(item.controller, &chosen);
    let sacrificed = chosen.iter().filter(|&&id| !is_live(g, id)).count();
    g.end_sentence();
    if (sacrificed as u64) < u64::from(owed) {
        g.lose_game(item.controller);
    }
    Ok(())
}

/// "When this enchantment is put into a graveyard from the battlefield, you lose the game." (CR 608.2).
fn lose_game(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.lose_game(item.controller);
    if g.state().is_over() {
        return Ok(());
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                when_asked(EventMask::DAMAGE, when_youre_dealt_damage),
                sacrifice_many_nontoken_permanents_if,
                "Whenever you're dealt damage, sacrifice that many nontoken permanents. If you can't, you lose the game.",
            ).damage_amount()
            .sums_per_subject(),
        triggered(
            triggers::THIS_DIES,
            lose_game,
            "When this enchantment is put into a graveyard from the battlefield, you lose the game.",
        ),
    ],
    replacements: &[
        replacement(
                ReplacementKind::LifeChange,
                if_would_gain_life_draw_applies,
                if_would_gain_life_draw_apply,
                "If you would gain life, draw that many cards instead.",
            )
            .then(if_would_gain_life_draw_then),
    ],
    enters: Some(as_enchantment_enters_lose_life_enters),
    loss_rules: &[statics::you_cant_lose_from_life],
    ..oracle::LICH
};
