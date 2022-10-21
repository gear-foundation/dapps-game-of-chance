#![no_std]

use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent};
use gstd::{async_main, exec, msg, prelude::*, util, ActorId};
use rand::{rngs::SmallRng, RngCore, SeedableRng};

mod io;

pub use io::*;

static mut CONTRACT: Option<Lottery> = None;

#[derive(Default)]
struct Lottery {
    admin: ActorId,
    ft_contract: ActorId,

    started: u64,
    ending: u64,
    players: BTreeSet<ActorId>,
    prize_fund: u128,
    participation_cost: u128,

    last_winner: ActorId,

    transactions: BTreeMap<ActorId, u64>,
    transaction_id_nonce: u64,
}

impl Lottery {
    fn start(&mut self, duration: u64, participation_cost: u128) -> LotteryEvent {
        if msg::source() != self.admin {
            panic!("Lottery must be started by its admin");
        }

        if self.started != 0 {
            panic!("Lottery mustn't be started");
        }

        self.players.clear();

        self.prize_fund = 0;
        self.started = exec::block_timestamp();
        self.ending = self.started + duration;
        self.participation_cost = participation_cost;

        LotteryEvent::Started {
            ending: self.ending,
            participation_cost,
        }
    }

    async fn pick_winner(&mut self) -> LotteryEvent {
        let block_timestamp = exec::block_timestamp();

        if msg::source() != self.admin {
            panic!("Winner must be picked by a lottery's admin");
        }

        if self.ending > block_timestamp {
            panic!("Winner can't be picked if a lottery is on");
        }

        let winner = if self.players.is_empty() {
            ActorId::zero()
        } else {
            let mut random_data = [0; (usize::BITS / 8) as usize];

            SmallRng::seed_from_u64(block_timestamp).fill_bytes(&mut random_data);

            let mystical_number = usize::from_le_bytes(random_data);

            let winner = *self
                .players
                .iter()
                .nth(mystical_number % self.players.len())
                .expect("Failed to receive a winner");

            let result = self
                .transfer_tokens(self.admin, exec::program_id(), winner, self.prize_fund)
                .await;

            if let FTokenEvent::Err = result {
                panic!("Failed to transfer tokens to a winner")
            }

            winner
        };

        self.started = 0;

        LotteryEvent::Winner(winner)
    }

    async fn transfer_tokens(
        &mut self,
        msg_source: ActorId,
        sender: ActorId,
        recipient: ActorId,
        amount: u128,
    ) -> FTokenEvent {
        let transaction_id = *self.transactions.entry(msg_source).or_insert_with(|| {
            let id = self.transaction_id_nonce;

            self.transaction_id_nonce = self.transaction_id_nonce.wrapping_add(1);

            id
        });

        let result = msg::send_for_reply_as(
            self.ft_contract,
            FTokenAction::Message {
                transaction_id,
                payload: Action::Transfer {
                    sender,
                    recipient,
                    amount,
                }
                .encode(),
            },
            0,
        )
        .expect("Failed to send `FTokenAction`")
        .await
        .expect("Failed to decode `FTokenEvent`");

        self.transactions.remove(&msg_source);

        result
    }

    async fn enter(&mut self) -> LotteryEvent {
        if self.ending <= exec::block_timestamp() {
            panic!("Lottery must be on");
        }

        let msg_source = msg::source();

        if self.players.contains(&msg_source) {
            panic!("Every player can't enter a lottery more than once")
        }

        let result = self
            .transfer_tokens(
                msg_source,
                msg_source,
                exec::program_id(),
                self.participation_cost,
            )
            .await;

        if let FTokenEvent::Err = result {
            panic!("Failed to transfer tokens for a participation");
        }

        self.players.insert(msg_source);
        self.prize_fund += self.participation_cost;

        LotteryEvent::PlayerAdded(msg_source)
    }
}

#[no_mangle]
extern "C" fn init() {
    let LotteryInit { admin, ft_contract } = msg::load().expect("Failed to decode `LotteryInit`");

    let contract = Lottery {
        admin,
        ft_contract,
        ..Default::default()
    };

    unsafe { CONTRACT = Some(contract) }
}

#[async_main]
async fn main() {
    let action: LotteryAction = msg::load().expect("Failed to decode `LotteryAction`");
    let contract = contract();

    let event = match action {
        LotteryAction::Start {
            duration,
            participation_cost,
        } => contract.start(duration, participation_cost),
        LotteryAction::PickWinner => contract.pick_winner().await,
        LotteryAction::Enter => contract.enter().await,
    };

    msg::reply(event, 0).expect("Failed to reply with `LotteryEvent`");
}

#[no_mangle]
extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: LotteryStateQuery = msg::load().expect("Failed to decode `LotteryStateQuery`");
    let contract = contract();

    let reply = match query {
        LotteryStateQuery::State => {
            let Lottery {
                started,
                ending,
                players,
                prize_fund,
                participation_cost,
                last_winner,
                ..
            } = contract;

            LotteryStateReply::State {
                started: *started,
                ending: *ending,
                players: players.clone(),
                prize_fund: *prize_fund,
                participation_cost: *participation_cost,
                last_winner: *last_winner,
            }
        }

        LotteryStateQuery::FTContract => LotteryStateReply::FTContract(contract.ft_contract),
    };

    util::to_leak_ptr(reply.encode())
}

fn contract() -> &'static mut Lottery {
    unsafe { CONTRACT.get_or_insert(Lottery::default()) }
}
