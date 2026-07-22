use crate::{Board, Color};

pub fn render(board: &Board) -> String {
    let mut output = String::new();

    for y in (0..board.size()).rev() {
        output.push_str(&format!("{:>2} ", y + 1));

        for x in 0..board.size() {
            let point = board.point(x, y).unwrap();

            let symbol = match board.color_at(point) {
                Some(Color::Black) => 'X',
                Some(Color::White) => 'O',
                None => '.',
            };

            output.push(symbol);
            output.push(' ');
        }

        output.push('\n');
    }

    output
}
