use moyodb_core::Board;

fn main() {
    let board = Board::new(19).expect("create 19x19 board");

    println!(
        "MoyoDB Qt application skeleton — {}x{} board",
        board.size(),
        board.size()
    );
}
