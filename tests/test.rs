use ft_logic_io::Action;
use ft_main_io::{FTokenAction, FTokenEvent, InitFToken};
use gstd::prelude::*;
use gtest::{Log, Program, System};
use lottery::*;

#[test]
fn it_works() {
    let system = System::new();

    system.init_logger();

    let storage_code: [u8; 32] = system.submit_code("target/ft_storage.opt.wasm").into();
    let logic_code: [u8; 32] = system.submit_code("target/ft_logic.opt.wasm").into();
    let ft = Program::from_file(&system, "target/ft_main.opt.wasm");
    let lottery = Program::current(&system);

    assert!(!ft
        .send(
            3,
            InitFToken {
                ft_logic_code_hash: logic_code.into(),
                storage_code_hash: storage_code.into(),
            },
        )
        .main_failed());

    assert!(!lottery
        .send(
            3,
            LotteryInit {
                admin: 3.into(),
                ft_contract: 1.into()
            }
        )
        .main_failed());

    assert!(ft
        .send(
            3,
            FTokenAction::Message {
                transaction_id: 0,
                payload: Action::Mint {
                    recipient: 4.into(),
                    amount: 12345,
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));
    assert!(ft
        .send(
            4,
            FTokenAction::Message {
                transaction_id: 1,
                payload: Action::Approve {
                    approved_account: 2.into(),
                    amount: 10000
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));

    assert!(ft
        .send(
            3,
            FTokenAction::Message {
                transaction_id: 2,
                payload: Action::Mint {
                    recipient: 5.into(),
                    amount: 12345,
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));
    assert!(ft
        .send(
            5,
            FTokenAction::Message {
                transaction_id: 3,
                payload: Action::Approve {
                    approved_account: 2.into(),
                    amount: 10000
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));

    assert!(ft
        .send(
            3,
            FTokenAction::Message {
                transaction_id: 4,
                payload: Action::Mint {
                    recipient: 6.into(),
                    amount: 12345,
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));
    assert!(ft
        .send(
            6,
            FTokenAction::Message {
                transaction_id: 5,
                payload: Action::Approve {
                    approved_account: 2.into(),
                    amount: 10000
                }
                .encode(),
            },
        )
        .contains(&Log::builder().payload(FTokenEvent::Ok)));

    assert!(lottery
        .send(
            3,
            LotteryAction::Start {
                duration: 2000,
                participation_cost: 10000,
            },
        )
        .contains(&Log::builder().payload(LotteryEvent::Started {
            ending: system.block_timestamp() + 2000,
            participation_cost: 10000,
        })));

    assert!(lottery
        .send(4, LotteryAction::Enter)
        .contains(&Log::builder().payload(LotteryEvent::PlayerAdded(4.into()))));

    assert!(lottery
        .send(5, LotteryAction::Enter)
        .contains(&Log::builder().payload(LotteryEvent::PlayerAdded(5.into()))));

    assert!(lottery
        .send(6, LotteryAction::Enter)
        .contains(&Log::builder().payload(LotteryEvent::PlayerAdded(6.into()))));

    system.spend_blocks(2);

    println!(
        "{:?}",
        lottery
            .send(3, LotteryAction::PickWinner)
            .decoded_log::<LotteryEvent>()
    );
}
