use utils::{prelude::*, Sft};

mod utils;

const ADMIN: u64 = 3;
const PLAYERS: [u64; 3] = [4, 5, 6];
const AMOUNT: u128 = 12345;
const PARTICIPATION_COST: u128 = 10000;
const DURATION: u64 = 2000;
const DURATION_IN_SECS: u32 = (DURATION / 1000) as _;

#[test]
fn two_rounds_and_meta_state() {
    let system = utils::initialize_system();

    let mut sft = Sft::initialize(&system);
    let mut lottery = Lottery::initialize(&system, ADMIN).succeed();

    lottery.meta_state().state().eq(LotteryStateReply::State {
        started: 0,
        ending: 0,
        players: BTreeSet::new(),
        prize_fund: 0,
        participation_cost: 0,
        last_winner: ActorId::zero(),
        ft_actor_id: None,
    });

    sft.mint(PLAYERS[0], AMOUNT).contains(FTokenEvent::Ok);
    sft.mint(PLAYERS[1], AMOUNT).contains(FTokenEvent::Ok);
    sft.mint(PLAYERS[2], AMOUNT).contains(FTokenEvent::Ok);

    sft.approve(PLAYERS[0], lottery.actor_id(), PARTICIPATION_COST)
        .contains(FTokenEvent::Ok);
    sft.approve(PLAYERS[1], lottery.actor_id(), PARTICIPATION_COST)
        .contains(FTokenEvent::Ok);
    sft.approve(PLAYERS[2], lottery.actor_id(), PARTICIPATION_COST)
        .contains(FTokenEvent::Ok);

    let mut started = system.block_timestamp();
    let mut ending = started + DURATION;
    let mut ft_actor_id = Some(sft.actor_id());

    lottery
        .start(ADMIN, DURATION, PARTICIPATION_COST, ft_actor_id)
        .contains((ending, PARTICIPATION_COST, ft_actor_id));
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::new(),
        prize_fund: 0,
        participation_cost: PARTICIPATION_COST,
        last_winner: ActorId::zero(),
        ft_actor_id,
    });

    lottery.enter(PLAYERS[0]).contains(PLAYERS[0]);
    sft.balance(lottery.actor_id()).contains(PARTICIPATION_COST);
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into()]),
        prize_fund: PARTICIPATION_COST,
        participation_cost: PARTICIPATION_COST,
        last_winner: ActorId::zero(),
        ft_actor_id,
    });

    lottery.enter(PLAYERS[1]).contains(PLAYERS[1]);
    sft.balance(lottery.actor_id())
        .contains(PARTICIPATION_COST * 2);
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into()]),
        prize_fund: PARTICIPATION_COST * 2,
        participation_cost: PARTICIPATION_COST,
        last_winner: ActorId::zero(),
        ft_actor_id,
    });

    lottery.enter(PLAYERS[2]).contains(PLAYERS[2]);
    sft.balance(lottery.actor_id())
        .contains(PARTICIPATION_COST * 3);
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into(), PLAYERS[2].into()]),
        prize_fund: PARTICIPATION_COST * 3,
        participation_cost: PARTICIPATION_COST,
        last_winner: ActorId::zero(),
        ft_actor_id,
    });

    system.spend_blocks(DURATION_IN_SECS);

    let mut winner = utils::predict_winner(&system, &PLAYERS);

    lottery.pick_winner(ADMIN).contains(winner.into());
    started = 0;
    sft.balance(winner)
        .contains(PARTICIPATION_COST * 2 + AMOUNT);
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into(), PLAYERS[2].into()]),
        prize_fund: PARTICIPATION_COST * 3,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });

    system.mint_to(PLAYERS[0], AMOUNT);
    system.mint_to(PLAYERS[1], AMOUNT);
    system.mint_to(PLAYERS[2], AMOUNT);

    ft_actor_id = None;
    started = system.block_timestamp();
    ending = started + DURATION;

    lottery
        .start(ADMIN, DURATION, PARTICIPATION_COST, ft_actor_id)
        .contains((ending, PARTICIPATION_COST, ft_actor_id));
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::new(),
        prize_fund: 0,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });

    lottery
        .enter_with_value(PLAYERS[0], PARTICIPATION_COST)
        .contains(PLAYERS[0]);
    assert_eq!(
        system.balance_of(lottery.actor_id().as_ref()),
        PARTICIPATION_COST
    );
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into()]),
        prize_fund: PARTICIPATION_COST,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });

    lottery
        .enter_with_value(PLAYERS[1], PARTICIPATION_COST)
        .contains(PLAYERS[1]);
    assert_eq!(
        system.balance_of(lottery.actor_id().as_ref()),
        PARTICIPATION_COST * 2
    );
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into()]),
        prize_fund: PARTICIPATION_COST * 2,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });

    lottery
        .enter_with_value(PLAYERS[2], PARTICIPATION_COST)
        .contains(PLAYERS[2]);
    assert_eq!(
        system.balance_of(lottery.actor_id().as_ref()),
        PARTICIPATION_COST * 3
    );
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into(), PLAYERS[2].into()]),
        prize_fund: PARTICIPATION_COST * 3,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });

    system.spend_blocks(DURATION_IN_SECS);

    winner = utils::predict_winner(&system, &PLAYERS);

    lottery.pick_winner(ADMIN).contains(winner.into());
    system.claim_value_from_mailbox(winner);
    assert_eq!(system.balance_of(winner), PARTICIPATION_COST * 2 + AMOUNT);
    lottery.meta_state().state().eq(LotteryStateReply::State {
        started: 0,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into(), PLAYERS[2].into()]),
        prize_fund: PARTICIPATION_COST * 3,
        participation_cost: PARTICIPATION_COST,
        last_winner: winner.into(),
        ft_actor_id,
    });
}

#[test]
fn failures() {
    let system = utils::initialize_system();

    // Should fail because `admin` mustn't be `ActorId::zero()`.
    Lottery::initialize(&system, ActorId::zero()).failed();

    let mut lottery = Lottery::initialize(&system, ADMIN).succeed();

    // Should fail because `msg::source()` must be an administrator
    lottery.start(FOREIGN_USER, 0, 0, None).failed();

    // Should fail because `ft_actor_id` mustn't be `ActorId::zero()`.
    lottery.start(ADMIN, 0, 0, Some(ActorId::zero())).failed();

    //Should fail because the players entry stage mustn't be over.
    lottery.enter(PLAYERS[0]).failed();

    lottery
        .start(ADMIN, DURATION, PARTICIPATION_COST, None)
        .contains((
            system.block_timestamp() + DURATION,
            PARTICIPATION_COST,
            None,
        ));

    // Should fail because the previous lottery round must be over.
    lottery.start(ADMIN, 0, 0, None).failed();

    system.mint_to(PLAYERS[0], AMOUNT);
    lottery
        .enter_with_value(PLAYERS[0], PARTICIPATION_COST)
        .contains(PLAYERS[0]);

    // Should fail because `msg::source()` mustn't already participate.
    lottery.enter(PLAYERS[0]).failed();

    system.mint_to(PLAYERS[1], AMOUNT);

    // Should fail because `msg::source()` must send the amount of the native
    // value exactly equal to a participation cost.
    lottery
        .enter_with_value(PLAYERS[1], PARTICIPATION_COST + 1)
        .failed();
    lottery
        .enter_with_value(PLAYERS[1], PARTICIPATION_COST - 1)
        .failed();

    // Should fail because `msg::source()` must be an administrator.
    lottery.pick_winner(FOREIGN_USER).failed();

    // Should fail because the players entry stage must be over.
    lottery.pick_winner(ADMIN).failed();

    system.spend_blocks(DURATION_IN_SECS);
    lottery.pick_winner(ADMIN).contains(PLAYERS[0].into());

    // Should fail because a winner mustn't already be picked.
    lottery.pick_winner(ADMIN).failed();

    // Should fail because the players entry stage mustn't be over.
    lottery.enter(PLAYERS[1]).failed();
}

#[test]
fn round_without_players() {
    let system = utils::initialize_system();

    let mut lottery = Lottery::initialize(&system, ADMIN).succeed();

    lottery
        .start(ADMIN, 0, 0, None)
        .contains((system.block_timestamp(), 0, None));

    lottery.pick_winner(ADMIN).contains(ActorId::zero());
}

#[test]
fn prize_fund_overflow() {
    const AMOUNT: u128 = u128::MAX;
    const PARTICIPATION_COST: u128 = u128::MAX;

    let system = utils::initialize_system();

    let mut sft = Sft::initialize(&system);
    let mut lottery = Lottery::initialize(&system, ADMIN).succeed();

    let started = system.block_timestamp();
    let ending = started + DURATION;
    let ft_actor_id = Some(sft.actor_id());

    lottery
        .start(ADMIN, DURATION, PARTICIPATION_COST, ft_actor_id)
        .contains((ending, PARTICIPATION_COST, ft_actor_id));

    sft.mint(PLAYERS[0], AMOUNT).contains(FTokenEvent::Ok);
    sft.mint(PLAYERS[1], AMOUNT).contains(FTokenEvent::Ok);

    sft.approve(PLAYERS[0], lottery.actor_id(), PARTICIPATION_COST)
        .contains(FTokenEvent::Ok);
    sft.approve(PLAYERS[1], lottery.actor_id(), PARTICIPATION_COST)
        .contains(FTokenEvent::Ok);

    lottery.enter(PLAYERS[0]).contains(PLAYERS[0]);
    lottery.enter(PLAYERS[1]).contains(PLAYERS[1]);

    lottery.meta_state().state().eq(LotteryStateReply::State {
        started,
        ending,
        players: BTreeSet::from([PLAYERS[0].into(), PLAYERS[1].into()]),
        prize_fund: u128::MAX,
        participation_cost: PARTICIPATION_COST,
        last_winner: ActorId::zero(),
        ft_actor_id,
    })
}
