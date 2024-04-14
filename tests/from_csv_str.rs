use std::io::BufRead;
use std::sync::Arc;

use assert_str::assert_str_trim_eq;

use payments_engine::domain::Ledger;
use payments_engine::run_csv_stream::run;

#[tokio::test]
async fn test_deposit_and_withdraw_work() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 10
deposit, 1, 2, 20
deposit, 2, 3, 10
withdrawal, 1, 4, 5
";
    let expected = "
client, available, held, total, locked
1, 25, 0, 25, false
2, 10, 0, 10, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_deposit_with_existing_tx_doesnt_change_balance() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 10
deposit, 1, 1, 20
";
    let expected = "
client, available, held, total, locked
1, 10, 0, 10, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_withdraw_with_insufficient_funds_doesnt_change_balance() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 10
withdrawal, 1, 2, 20
";
    let expected = "
client, available, held, total, locked
1, 10, 0, 10, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_dispute_moves_funds_from_available_to_held() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
deposit, 1, 2, 10
dispute, 1, 1
";
    let expected = "
client, available, held, total, locked
1, 10, 5, 15, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_dispute_on_inexistent_tx_doesnt_change_ballance() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
deposit, 1, 2, 10
dispute, 1, 3
";
    let expected = "
client, available, held, total, locked
1, 15, 0, 15, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_resolve_and_chargeback_dont_change_ballances_for_inexistent_tx() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
deposit, 2, 2, 10
resolve, 1, 1
chargeback, 1, 2
";
    let expected = "
client, available, held, total, locked
1, 5, 0, 5, false
2, 10, 0, 10, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_resolve_increases_available_and_total() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
deposit, 1, 2, 10
dispute, 1, 1
resolve, 1, 1
";
    let expected = "
client, available, held, total, locked
1, 15, 0, 15, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_chargeback_decreases_held_and_total_and_sets_locked() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
deposit, 1, 2, 10
dispute, 1, 1
chargeback, 1, 1
";
    let expected = "
client, available, held, total, locked
1, 10, 0, 10, true
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_deposit_dispute_chargeback_withdraw() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5
dispute, 1, 1
chargeback, 1, 1
withdraw, 1, 2, 10
";
    let expected = "
client, available, held, total, locked
1, 0, 0, 0, true
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

#[tokio::test]
async fn test_decimals_up_to_4_places_are_accepted() {
    let test_data = "
type, client, tx, amount
deposit, 1, 1, 5.1234
withdrawal, 1, 2, 0.0003
";
    let expected = "
client, available, held, total, locked
1, 5.1231, 0, 5.1231, false
";
    let output = get_sorted_ledger_dump(test_data).await;

    assert_str_trim_eq!(expected, output);
}

async fn get_sorted_ledger_dump(test_data: &'static str) -> String {
    let ledger = Arc::new(Ledger::new());
    run(test_data.as_bytes(), ledger.clone()).await;
    let mut output = Vec::new();
    ledger.dump_to_writer(&mut output).unwrap();

    let mut output = output.lines();
    let mut header = output.next().unwrap().unwrap();
    let mut lines = output.map(|v| v.unwrap()).collect::<Vec<_>>();
    lines.sort_by_key(|l| l.split(',').collect::<Vec<_>>()[0].parse::<u16>().unwrap());
    header.push('\n');
    lines.iter_mut().for_each(|l| l.push('\n'));
    header.extend(lines);

    header
}
